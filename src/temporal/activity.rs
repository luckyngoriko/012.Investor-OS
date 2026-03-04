//! Activity trait and implementations

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

use super::{RetryPolicy, TemporalError};

/// Activity trait — idempotent операции
#[async_trait]
pub trait Activity: Send + Sync + Clone + 'static {
    type Input: Serialize + DeserializeOwned + Send + Sync + Clone;
    type Output: Serialize + DeserializeOwned + Send + Sync;

    fn name() -> &'static str
    where
        Self: Sized;
    async fn execute(
        &self,
        ctx: ActivityContext,
        input: Self::Input,
    ) -> Result<Self::Output, ActivityError>;
}

/// Контекст за activity execution
#[derive(Debug, Clone)]
pub struct ActivityContext {
    pub attempt: u32,
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub heartbeat_details: Vec<u8>,
}

impl Default for ActivityContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityContext {
    pub fn new() -> Self {
        Self {
            attempt: 1,
            scheduled_at: chrono::Utc::now(),
            started_at: chrono::Utc::now(),
            deadline: None,
            heartbeat_details: vec![],
        }
    }

    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    pub fn is_retry(&self) -> bool {
        self.attempt > 1
    }

    pub fn elapsed(&self) -> Duration {
        let now = chrono::Utc::now();
        Duration::from_millis((now - self.started_at).num_milliseconds() as u64)
    }

    pub fn has_time_remaining(&self) -> bool {
        match self.deadline {
            Some(deadline) => chrono::Utc::now() < deadline,
            None => true,
        }
    }
}

/// Грешка при activity execution
#[derive(Debug, Clone)]
pub struct ActivityError {
    pub message: String,
    pub error_type: ErrorType,
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    Application,
    Timeout,
    Cancelled,
    Panic,
}

impl ActivityError {
    pub fn application(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_type: ErrorType::Application,
            details: None,
            retryable: false,
        }
    }

    pub fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_type: ErrorType::Application,
            details: None,
            retryable: true,
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_type: ErrorType::Timeout,
            details: None,
            retryable: true,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::fmt::Display for ActivityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.error_type.as_str(), self.message)
    }
}

impl std::error::Error for ActivityError {}

impl ErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorType::Application => "ApplicationError",
            ErrorType::Timeout => "TimeoutError",
            ErrorType::Cancelled => "CancelledError",
            ErrorType::Panic => "PanicError",
        }
    }
}

/// Activity executor с retry логика
pub struct ActivityExecutor {
    retry_policy: RetryPolicy,
}

impl ActivityExecutor {
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            retry_policy: policy,
        }
    }

    pub async fn execute<A: Activity>(
        &self,
        activity: &A,
        input: A::Input,
    ) -> Result<A::Output, TemporalError> {
        let mut attempt = 1u32;

        loop {
            let ctx = ActivityContext {
                attempt,
                scheduled_at: chrono::Utc::now(),
                started_at: chrono::Utc::now(),
                deadline: None,
                heartbeat_details: vec![],
            };

            match activity.execute(ctx, input.clone()).await {
                Ok(output) => return Ok(output),
                Err(e) if !e.retryable => {
                    return Err(TemporalError::ActivityFailed(e.message));
                }
                Err(e) => {
                    if !self.retry_policy.should_retry(attempt, &e.message) {
                        return Err(TemporalError::ActivityFailed(format!(
                            "Max retries exceeded: {}",
                            e.message
                        )));
                    }

                    let backoff = self.retry_policy.calculate_backoff(attempt);
                    tracing::warn!(
                        "Activity failed (attempt {}), retrying in {:?}: {}",
                        attempt,
                        backoff,
                        e.message
                    );

                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }
}

// ==================== Trading Activities ====================

use rust_decimal::Decimal;

/// Fetch market data activity
#[derive(Debug, Clone)]
pub struct FetchMarketData;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FetchMarketDataInput {
    pub ticker: String,
    pub days: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FetchMarketDataOutput {
    pub ticker: String,
    pub price: Option<Decimal>,
    pub ohlcv: Vec<OhlcvData>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OhlcvData {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: u64,
}

#[async_trait]
impl Activity for FetchMarketData {
    type Input = FetchMarketDataInput;
    type Output = FetchMarketDataOutput;

    fn name() -> &'static str {
        "FetchMarketData"
    }

    async fn execute(
        &self,
        _ctx: ActivityContext,
        input: Self::Input,
    ) -> Result<Self::Output, ActivityError> {
        // Implementation would call market data service
        // For now, return mock data
        Ok(FetchMarketDataOutput {
            ticker: input.ticker.clone(),
            price: Some(Decimal::from(150)),
            ohlcv: vec![],
            fetched_at: chrono::Utc::now(),
        })
    }
}

/// Call LLM activity
#[derive(Debug, Clone)]
pub struct CallLLM;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallLLMInput {
    pub prompt: String,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallLLMOutput {
    pub response: String,
    pub model: String,
    pub tokens_used: u32,
    pub cost_usd: f64,
}

#[async_trait]
impl Activity for CallLLM {
    type Input = CallLLMInput;
    type Output = CallLLMOutput;

    fn name() -> &'static str {
        "CallLLM"
    }

    async fn execute(
        &self,
        _ctx: ActivityContext,
        input: Self::Input,
    ) -> Result<Self::Output, ActivityError> {
        // Would integrate with ml::apis
        Ok(CallLLMOutput {
            response: "Mock response".to_string(),
            model: input.model,
            tokens_used: 100,
            cost_usd: 0.01,
        })
    }
}

/// Place order activity
#[derive(Debug, Clone)]
pub struct PlaceOrder;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlaceOrderInput {
    pub ticker: String,
    pub action: String, // BUY, SELL
    pub quantity: Decimal,
    pub order_type: String, // MARKET, LIMIT, STOP
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlaceOrderOutput {
    pub order_id: String,
    pub status: String, // PENDING, FILLED, PARTIAL, REJECTED
    pub filled_quantity: Decimal,
    pub avg_price: Option<Decimal>,
    pub commission: Decimal,
    pub placed_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl Activity for PlaceOrder {
    type Input = PlaceOrderInput;
    type Output = PlaceOrderOutput;

    fn name() -> &'static str {
        "PlaceOrder"
    }

    async fn execute(
        &self,
        _ctx: ActivityContext,
        input: Self::Input,
    ) -> Result<Self::Output, ActivityError> {
        // Would call broker service
        Ok(PlaceOrderOutput {
            order_id: uuid::Uuid::new_v4().to_string(),
            status: "FILLED".to_string(),
            filled_quantity: input.quantity,
            avg_price: None,
            commission: Decimal::from(1),
            placed_at: chrono::Utc::now(),
        })
    }
}

/// Write to journal activity
#[derive(Debug, Clone)]
pub struct WriteJournal;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WriteJournalInput {
    pub entry_type: String,
    pub data: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WriteJournalOutput {
    pub entry_id: String,
    pub written_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl Activity for WriteJournal {
    type Input = WriteJournalInput;
    type Output = WriteJournalOutput;

    fn name() -> &'static str {
        "WriteJournal"
    }

    async fn execute(
        &self,
        _ctx: ActivityContext,
        _input: Self::Input,
    ) -> Result<Self::Output, ActivityError> {
        Ok(WriteJournalOutput {
            entry_id: uuid::Uuid::new_v4().to_string(),
            written_at: chrono::Utc::now(),
        })
    }
}

/// Calculate CQ activity
#[derive(Debug, Clone)]
pub struct CalculateCQ;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalculateCQInput {
    pub ticker: String,
    pub quality_score: f64,
    pub insider_score: f64,
    pub sentiment_score: f64,
    pub regime_fit: f64,
    pub breakout_score: f64,
    pub atr_trend: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalculateCQOutput {
    pub ticker: String,
    pub cq: f64,
    pub breakdown: serde_json::Value,
}

#[async_trait]
impl Activity for CalculateCQ {
    type Input = CalculateCQInput;
    type Output = CalculateCQOutput;

    fn name() -> &'static str {
        "CalculateCQ"
    }

    async fn execute(
        &self,
        _ctx: ActivityContext,
        input: Self::Input,
    ) -> Result<Self::Output, ActivityError> {
        // CQ v2.0 formula
        let cq = input.quality_score * 0.20
            + input.insider_score * 0.20
            + input.sentiment_score * 0.15
            + input.regime_fit * 0.20
            + input.breakout_score * 0.15
            + input.atr_trend * 0.10;

        Ok(CalculateCQOutput {
            ticker: input.ticker.clone(),
            cq,
            breakdown: serde_json::json!({
                "pegy_relative": input.quality_score * 0.20,
                "insider": input.insider_score * 0.20,
                "sentiment": input.sentiment_score * 0.15,
                "regime_fit": input.regime_fit * 0.20,
                "breakout": input.breakout_score * 0.15,
                "atr_trend": input.atr_trend * 0.10,
            }),
        })
    }
}
