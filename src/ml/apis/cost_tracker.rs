//! ML API Cost Tracker
//!
//! Tracks API usage and costs across providers
//! Sprint 10: Cost tracking & budget enforcement

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc, Duration};

/// Cost tracker for ML API usage
#[derive(Debug, Clone)]
pub struct CostTracker {
    inner: Arc<Mutex<CostTrackerInner>>,
}

#[derive(Debug)]
struct CostTrackerInner {
    /// Usage by provider
    usage: HashMap<String, ProviderUsage>,
    /// Daily costs
    daily_costs: HashMap<String, f64>, // YYYY-MM-DD -> cost
    /// Monthly budget
    monthly_budget: f64,
    /// Current month total
    current_month_cost: f64,
    /// Rate limit tracking
    rate_limits: HashMap<String, RateLimitStatus>,
}

/// Usage statistics for a provider
#[derive(Debug, Clone, Default)]
pub struct ProviderUsage {
    pub provider: String,
    pub requests_count: u64,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub total_cost: f64,
    pub last_request: Option<DateTime<Utc>>,
}

/// Rate limit status
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub provider: String,
    pub requests_remaining: u32,
    pub reset_time: DateTime<Utc>,
    pub limit_per_minute: u32,
}

/// Cost per 1K tokens for each provider/model
#[derive(Debug, Clone)]
pub struct PricingConfig {
    pub gemini_input: f64,      // Free tier: $0
    pub gemini_output: f64,     // Free tier: $0
    pub openai_gpt4o_mini_input: f64,   // $0.15 per 1M tokens = $0.00015 per 1K
    pub openai_gpt4o_mini_output: f64,  // $0.60 per 1M tokens = $0.00060 per 1K
    pub claude_input: f64,      // ~$15 per 1M tokens = $0.015 per 1K
    pub claude_output: f64,     // ~$75 per 1M tokens = $0.075 per 1K
    pub huggingface: f64,       // Free tier: $0
}

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            gemini_input: 0.0,
            gemini_output: 0.0,
            openai_gpt4o_mini_input: 0.00015,
            openai_gpt4o_mini_output: 0.00060,
            claude_input: 0.015,
            claude_output: 0.075,
            huggingface: 0.0,
        }
    }
}

impl CostTracker {
    pub fn new(monthly_budget: f64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CostTrackerInner {
                usage: HashMap::new(),
                daily_costs: HashMap::new(),
                monthly_budget,
                current_month_cost: 0.0,
                rate_limits: HashMap::new(),
            })),
        }
    }
    
    /// Record API usage
    pub fn record_usage(
        &self,
        provider: &str,
        tokens_input: u64,
        tokens_output: u64,
    ) -> f64 {
        let mut inner = self.inner.lock().unwrap();
        
        let cost = self.calculate_cost(provider, tokens_input, tokens_output);
        
        // Update provider usage
        let usage = inner.usage.entry(provider.to_string()).or_insert_with(|| ProviderUsage {
            provider: provider.to_string(),
            ..Default::default()
        });
        
        usage.requests_count += 1;
        usage.tokens_input += tokens_input;
        usage.tokens_output += tokens_output;
        usage.total_cost += cost;
        usage.last_request = Some(Utc::now());
        
        // Update daily costs
        let today = Utc::now().format("%Y-%m-%d").to_string();
        *inner.daily_costs.entry(today).or_insert(0.0) += cost;
        
        // Update monthly cost
        inner.current_month_cost += cost;
        
        cost
    }
    
    /// Check if we're within budget
    pub fn is_within_budget(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.current_month_cost < inner.monthly_budget
    }
    
    /// Get remaining budget
    pub fn remaining_budget(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        (inner.monthly_budget - inner.current_month_cost).max(0.0)
    }
    
    /// Get usage stats for a provider
    pub fn get_provider_usage(&self, provider: &str) -> Option<ProviderUsage> {
        let inner = self.inner.lock().unwrap();
        inner.usage.get(provider).cloned()
    }
    
    /// Get all usage stats
    pub fn get_all_usage(&self) -> Vec<ProviderUsage> {
        let inner = self.inner.lock().unwrap();
        inner.usage.values().cloned().collect()
    }
    
    /// Get today's cost
    pub fn today_cost(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        inner.daily_costs.get(&today).copied().unwrap_or(0.0)
    }
    
    /// Get current month total
    pub fn current_month_cost(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        inner.current_month_cost
    }
    
    /// Get monthly budget
    pub fn monthly_budget(&self) -> f64 {
        let inner = self.inner.lock().unwrap();
        inner.monthly_budget
    }
    
    /// Check if provider is rate limited
    pub fn is_rate_limited(&self, provider: &str) -> bool {
        let inner = self.inner.lock().unwrap();
        
        if let Some(limit) = inner.rate_limits.get(provider) {
            if limit.requests_remaining == 0 && Utc::now() < limit.reset_time {
                return true;
            }
        }
        
        false
    }
    
    /// Update rate limit status
    pub fn update_rate_limit(&self, provider: &str, remaining: u32, reset_in_seconds: i64) {
        let mut inner = self.inner.lock().unwrap();
        
        inner.rate_limits.insert(provider.to_string(), RateLimitStatus {
            provider: provider.to_string(),
            requests_remaining: remaining,
            reset_time: Utc::now() + Duration::seconds(reset_in_seconds),
            limit_per_minute: 0, // Would be set from API response
        });
    }
    
    /// Calculate cost for a request
    fn calculate_cost(&self, provider: &str, tokens_input: u64, tokens_output: u64) -> f64 {
        let pricing = PricingConfig::default();
        
        let (input_price, output_price) = match provider {
            "gemini" => (pricing.gemini_input, pricing.gemini_output),
            "openai" => (pricing.openai_gpt4o_mini_input, pricing.openai_gpt4o_mini_output),
            "claude" => (pricing.claude_input, pricing.claude_output),
            "huggingface" => (pricing.huggingface, pricing.huggingface),
            _ => (0.0, 0.0),
        };
        
        let input_cost = (tokens_input as f64 / 1000.0) * input_price;
        let output_cost = (tokens_output as f64 / 1000.0) * output_price;
        
        input_cost + output_cost
    }
    
    /// Reset monthly costs (call at start of new month)
    pub fn reset_month(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.current_month_cost = 0.0;
        inner.usage.clear();
    }
    
    /// Generate cost report
    pub fn generate_report(&self) -> CostReport {
        let inner = self.inner.lock().unwrap();
        
        CostReport {
            generated_at: Utc::now(),
            monthly_budget: inner.monthly_budget,
            current_month_cost: inner.current_month_cost,
            remaining_budget: inner.monthly_budget - inner.current_month_cost,
            usage_percentage: (inner.current_month_cost / inner.monthly_budget) * 100.0,
            provider_usage: inner.usage.values().cloned().collect(),
            today_cost: inner.daily_costs.get(&Utc::now().format("%Y-%m-%d").to_string()).copied().unwrap_or(0.0),
        }
    }
}

/// Cost report for Grafana/dashboard
#[derive(Debug, Clone)]
pub struct CostReport {
    pub generated_at: DateTime<Utc>,
    pub monthly_budget: f64,
    pub current_month_cost: f64,
    pub remaining_budget: f64,
    pub usage_percentage: f64,
    pub provider_usage: Vec<ProviderUsage>,
    pub today_cost: f64,
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new(250.0) // Default $250/month budget
    }
}

/// Middleware to wrap API calls with cost tracking
pub struct TrackedClient<T> {
    inner: T,
    tracker: CostTracker,
    provider_name: String,
    /// Estimated average tokens per request
    avg_input_tokens: u64,
    avg_output_tokens: u64,
}

impl<T> TrackedClient<T> {
    pub fn new(
        inner: T,
        tracker: CostTracker,
        provider_name: impl Into<String>,
        avg_input: u64,
        avg_output: u64,
    ) -> Self {
        Self {
            inner,
            tracker,
            provider_name: provider_name.into(),
            avg_input_tokens: avg_input,
            avg_output_tokens: avg_output,
        }
    }
    
    /// Track a request (call this after successful API call)
    pub fn track_request(&self, actual_input: Option<u64>, actual_output: Option<u64>) -> f64 {
        let input = actual_input.unwrap_or(self.avg_input_tokens);
        let output = actual_output.unwrap_or(self.avg_output_tokens);
        
        self.tracker.record_usage(&self.provider_name, input, output)
    }
    
    /// Check budget before making request
    pub fn can_make_request(&self) -> bool {
        if !self.tracker.is_within_budget() {
            return false;
        }
        
        if self.tracker.is_rate_limited(&self.provider_name) {
            return false;
        }
        
        true
    }
    
    pub fn tracker(&self) -> &CostTracker {
        &self.tracker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cost_tracking() {
        let tracker = CostTracker::new(100.0);
        
        // Simulate OpenAI usage
        let cost1 = tracker.record_usage("openai", 1000, 500);
        assert!(cost1 > 0.0);
        
        let cost2 = tracker.record_usage("openai", 2000, 1000);
        assert!(cost2 > cost1);
        
        // Check total
        let usage = tracker.get_provider_usage("openai").unwrap();
        assert_eq!(usage.requests_count, 2);
        assert_eq!(usage.tokens_input, 3000);
        
        // Check budget
        assert!(tracker.is_within_budget());
        assert!(tracker.remaining_budget() < 100.0);
    }
    
    #[test]
    fn test_budget_enforcement() {
        let tracker = CostTracker::new(0.01); // Very small budget
        
        // One expensive request should exceed budget
        tracker.record_usage("claude", 10000, 5000);
        
        assert!(!tracker.is_within_budget());
        assert_eq!(tracker.remaining_budget(), 0.0);
    }
}
