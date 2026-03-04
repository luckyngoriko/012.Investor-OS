//! LLM Strategist for Phoenix
//!
//! Uses LLM APIs to make trading decisions based on:
//! - RAG memory (past experiences)
//! - Market context
//! - Technical analysis

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::memory::{ExperienceQuery, MarketRegime, OutcomeFilter, RagMemory, TradingExperience};
use super::{Action, TradingDecision};

/// LLM Strategist that makes trading decisions
pub struct LlmStrategist {
    provider: Arc<dyn LlmProvider>,
    config: StrategistConfig,
}

/// LLM Provider trait
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate_decision(
        &self,
        prompt: &str,
        context: &DecisionContext,
    ) -> Result<TradingDecision, LlmError>;
    async fn analyze_experience(&self, experience: &TradingExperience) -> Result<String, LlmError>;
}

/// LLM Provider errors
#[derive(Debug, Clone)]
pub enum LlmError {
    ApiError(String),
    RateLimitExceeded,
    InvalidResponse(String),
    ContextTooLong,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::ApiError(msg) => write!(f, "API Error: {}", msg),
            LlmError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            LlmError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            LlmError::ContextTooLong => write!(f, "Context too long"),
        }
    }
}

impl std::error::Error for LlmError {}

/// Strategist configuration
#[derive(Debug, Clone)]
pub struct StrategistConfig {
    pub confidence_threshold: f64,
    pub max_context_experiences: usize,
    pub use_regime_filter: bool,
    pub min_confidence_to_trade: f64,
}

impl Default for StrategistConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.6,
            max_context_experiences: 5,
            use_regime_filter: true,
            min_confidence_to_trade: 0.7,
        }
    }
}

/// Context for decision making
#[derive(Debug, Clone)]
pub struct DecisionContext {
    pub ticker: String,
    pub current_price: rust_decimal::Decimal,
    pub rsi: Option<f64>,
    pub trend: String,
    pub regime: MarketRegime,
    pub portfolio_value: rust_decimal::Decimal,
    pub cash_available: rust_decimal::Decimal,
    pub current_position: Option<u32>,
    pub market_sentiment: Sentiment,
}

/// Market sentiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentiment {
    pub overall: f64, // -1.0 to 1.0
    pub news: f64,
    pub social: f64,
    pub analyst: f64,
}

impl Default for Sentiment {
    fn default() -> Self {
        Self {
            overall: 0.0,
            news: 0.0,
            social: 0.0,
            analyst: 0.0,
        }
    }
}

impl LlmStrategist {
    pub fn new(provider: Arc<dyn LlmProvider>, config: StrategistConfig) -> Self {
        Self { provider, config }
    }

    /// Make trading decision based on context and memory
    pub async fn decide(
        &self,
        context: &DecisionContext,
        memory: &RagMemory,
    ) -> Result<TradingDecision, LlmError> {
        // Query similar experiences from memory
        let query = ExperienceQuery {
            ticker: Some(context.ticker.clone()),
            regime: if self.config.use_regime_filter {
                Some(context.regime.clone())
            } else {
                None
            },
            outcome_filter: OutcomeFilter::WinnersOnly,
            limit: self.config.max_context_experiences,
            ..Default::default()
        };

        let similar_experiences = memory.query_similar_cases(&query);
        let regime_insight = memory.what_works_in_regime(&context.regime);

        // Build decision prompt
        let prompt = self.build_decision_prompt(context, &similar_experiences, &regime_insight);

        // Get decision from LLM
        self.provider.generate_decision(&prompt, context).await
    }

    /// Generate lesson from experience
    pub async fn generate_lesson(
        &self,
        experience: &TradingExperience,
    ) -> Result<String, LlmError> {
        self.provider.analyze_experience(experience).await
    }

    /// Build decision prompt for LLM
    fn build_decision_prompt(
        &self,
        context: &DecisionContext,
        experiences: &[&TradingExperience],
        regime_insight: &super::memory::RegimeInsight,
    ) -> String {
        let mut prompt = format!(
            r#"You are an expert quantitative trader. Analyze the market conditions and decide whether to BUY, SELL, or HOLD.

CURRENT MARKET CONDITIONS:
- Ticker: {}
- Current Price: ${}
- RSI: {}
- Trend: {}
- Market Regime: {:?}
- Portfolio Value: ${}
- Cash Available: ${}
- Current Position: {} shares
- Market Sentiment: {:.2} (range: -1 to 1)

REGIME ANALYSIS:
{}

SIMILAR PAST EXPERIENCES:
"#,
            context.ticker,
            context.current_price,
            context
                .rsi
                .map(|r| format!("{:.1}", r))
                .unwrap_or_else(|| "N/A".to_string()),
            context.trend,
            context.regime,
            context.portfolio_value,
            context.cash_available,
            context.current_position.unwrap_or(0),
            context.market_sentiment.overall,
            regime_insight.recommendation,
        );

        // Add similar experiences
        if experiences.is_empty() {
            prompt.push_str("No similar past experiences available.\n");
        } else {
            for (i, exp) in experiences.iter().enumerate() {
                prompt.push_str(&format!(
                    r#"{}. {} {} at ${} → P&L: {:.1}% | Lesson: {}
"#,
                    i + 1,
                    format!("{:?}", exp.decision.action),
                    exp.market_condition.ticker,
                    exp.market_condition.price,
                    exp.outcome.profit_loss_pct,
                    exp.lesson,
                ));
            }
        }

        prompt.push_str(
            r#"
DECISION INSTRUCTIONS:
1. Analyze the current market conditions
2. Consider similar past experiences and their outcomes
3. Evaluate the regime context
4. Make a decision: BUY, SELL, or HOLD
5. Provide your confidence level (0.0 to 1.0)
6. Explain your reasoning in 1-2 sentences

RESPONSE FORMAT:
{
    "action": "BUY|SELL|HOLD",
    "confidence": 0.85,
    "rationale": "Brief explanation of decision"
}

Only respond with the JSON object, nothing else.
"#,
        );

        prompt
    }
}

/// Mock LLM provider for testing
pub struct MockLlmProvider {
    default_action: Action,
    default_confidence: f64,
}

impl MockLlmProvider {
    pub fn new(action: Action, confidence: f64) -> Self {
        Self {
            default_action: action,
            default_confidence: confidence,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for MockLlmProvider {
    async fn generate_decision(
        &self,
        _prompt: &str,
        context: &DecisionContext,
    ) -> Result<TradingDecision, LlmError> {
        Ok(TradingDecision {
            action: self.default_action.clone(),
            ticker: context.ticker.clone(),
            quantity: Some(10),
            confidence: self.default_confidence,
            rationale: "Mock decision for testing".to_string(),
        })
    }

    async fn analyze_experience(&self, experience: &TradingExperience) -> Result<String, LlmError> {
        let lesson = if experience.outcome.success {
            format!(
                "Good trade: captured {}%",
                experience.outcome.profit_loss_pct
            )
        } else {
            format!("Mistake: lost {}%", experience.outcome.profit_loss_pct)
        };
        Ok(lesson)
    }
}

/// Rule-based strategist (fallback when LLM unavailable)
pub struct RuleBasedStrategist;

impl RuleBasedStrategist {
    pub fn decide(&self, context: &DecisionContext) -> TradingDecision {
        let action = match context.rsi {
            Some(rsi) if rsi < 30.0 => Action::Buy,  // Oversold
            Some(rsi) if rsi > 70.0 => Action::Sell, // Overbought
            _ => Action::Hold,
        };

        let confidence = match action {
            Action::Buy if context.rsi.unwrap_or(50.0) < 20.0 => 0.8,
            Action::Sell if context.rsi.unwrap_or(50.0) > 80.0 => 0.8,
            Action::Buy | Action::Sell => 0.6,
            Action::Hold => 0.5,
        };

        TradingDecision {
            action,
            ticker: context.ticker.clone(),
            quantity: Some(10),
            confidence,
            rationale: format!("RSI-based rule: {:?}", context.rsi),
        }
    }
}
