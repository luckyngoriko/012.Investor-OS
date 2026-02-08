//! ML API Integrations - Sprint 10

pub mod gemini;
pub mod openai;
pub mod claude;
pub mod huggingface;
pub mod cost_tracker;
pub mod cache;

/// Simple enum for LLM providers (avoids dyn compatibility issues)
#[derive(Debug)]
pub enum LLMProvider {
    Gemini(gemini::GeminiClient),
    OpenAI(openai::OpenAIClient),
    Claude(claude::ClaudeClient),
    HuggingFace(huggingface::HFClient),
}

impl LLMProvider {
    pub async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        match self {
            Self::Gemini(c) => c.generate(prompt).await,
            Self::OpenAI(c) => c.generate(prompt).await,
            Self::Claude(c) => c.generate(prompt).await,
            Self::HuggingFace(c) => c.generate(prompt).await,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::Gemini(_) => "Gemini",
            Self::OpenAI(_) => "OpenAI",
            Self::Claude(_) => "Claude",
            Self::HuggingFace(_) => "HuggingFace",
        }
    }
}

/// ML API Orchestrator with cost tracking and caching
pub struct MLApiOrchestrator {
    providers: Vec<LLMProvider>,
    cost_tracker: cost_tracker::CostTracker,
    cache: Option<cache::ResponseCache>,
}

impl MLApiOrchestrator {
    pub fn new() -> Self {
        Self {
            providers: vec![],
            cost_tracker: cost_tracker::CostTracker::default(),
            cache: None,
        }
    }
    
    pub fn with_cache(mut self, cache: cache::ResponseCache) -> Self {
        self.cache = Some(cache);
        self
    }
    
    pub fn with_budget(mut self, budget: f64) -> Self {
        self.cost_tracker = cost_tracker::CostTracker::new(budget);
        self
    }
    
    pub fn add_provider(&mut self, provider: LLMProvider) {
        self.providers.push(provider);
    }
    
    /// Generate with caching and cost tracking
    pub async fn generate(&self, prompt: &str) -> Result<String, LLMError> {
        // Check budget first
        if !self.cost_tracker.is_within_budget() {
            return Err(LLMError::BudgetExceeded);
        }
        
        // Try cache first
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get("any", prompt).await {
                return Ok(cached);
            }
        }
        
        // Try each provider with fallback
        for provider in &self.providers {
            // Check rate limits
            if self.cost_tracker.is_rate_limited(provider.name().to_lowercase().as_str()) {
                tracing::warn!("{} rate limited, trying next", provider.name());
                continue;
            }
            
            match provider.generate(prompt).await {
                Ok(response) => {
                    // Track cost (estimated)
                    let provider_name = provider.name().to_lowercase();
                    let (input_tokens, output_tokens) = estimate_tokens(prompt, &response);
                    self.cost_tracker.record_usage(&provider_name, input_tokens, output_tokens);
                    
                    // Cache response
                    if let Some(cache) = &self.cache {
                        let _ = cache.set(&provider_name, prompt, &response).await;
                    }
                    
                    return Ok(response);
                }
                Err(e) => {
                    tracing::warn!("{} failed: {}", provider.name(), e);
                    // Update rate limit if rate limited
                    if matches!(e, LLMError::RateLimitExceeded) {
                        self.cost_tracker.update_rate_limit(&provider.name().to_lowercase(), 0, 60);
                    }
                }
            }
        }
        
        Err(LLMError::AllProvidersFailed)
    }
    
    /// Get cost statistics
    pub fn cost_stats(&self) -> cost_tracker::CostReport {
        self.cost_tracker.generate_report()
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> Option<cache::CacheStats> {
        self.cache.as_ref().map(|c| c.stats())
    }
    
    /// Check if within budget
    pub fn is_within_budget(&self) -> bool {
        self.cost_tracker.is_within_budget()
    }
    
    /// Get remaining budget
    pub fn remaining_budget(&self) -> f64 {
        self.cost_tracker.remaining_budget()
    }
}

impl Default for MLApiOrchestrator {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limit")]
    RateLimitExceeded,
    #[error("Parse: {0}")]
    ParseError(String),
    #[error("All providers failed")]
    AllProvidersFailed,
    #[error("Budget exceeded")]
    BudgetExceeded,
}

/// Estimate token count (rough approximation: 4 chars ≈ 1 token)
fn estimate_tokens(input: &str, output: &str) -> (u64, u64) {
    let input_tokens = (input.len() as u64 / 4).max(1);
    let output_tokens = (output.len() as u64 / 4).max(1);
    (input_tokens, output_tokens)
}

#[derive(Debug, Clone, Default)]
pub struct LLMConfig {
    pub gemini_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub claude_api_key: Option<String>,
    pub hf_token: Option<String>,
}

pub fn create_orchestrator(config: &LLMConfig) -> MLApiOrchestrator {
    let mut orch = MLApiOrchestrator::new();
    
    if let Some(key) = &config.gemini_api_key {
        orch.add_provider(LLMProvider::Gemini(gemini::GeminiClient::new(key.clone())));
    }
    if let Some(key) = &config.openai_api_key {
        orch.add_provider(LLMProvider::OpenAI(openai::OpenAIClient::new(key.clone())));
    }
    if let Some(key) = &config.claude_api_key {
        orch.add_provider(LLMProvider::Claude(claude::ClaudeClient::new(key.clone())));
    }
    
    orch
}
