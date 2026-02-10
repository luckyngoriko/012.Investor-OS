//! Temporal-inspired Durable Workflow Engine for Rust
//!
//! Гарантира изпълнение на trading workflows дори при:
//! - Process crashes
//! - Network failures
//! - API timeouts
//! - System restarts

pub mod workflow;
pub mod activity;
pub mod context;
pub mod client;
pub mod saga;
pub mod worker;

pub use workflow::{Workflow, WorkflowHandle, WorkflowComposer};
pub use activity::{Activity, ActivityError, ActivityExecutor, ActivityContext as ActCtx};
pub use context::{WorkflowContext, ActivityContext};
pub use client::{TemporalClient, WorkflowClient};
pub use saga::{Saga, SagaBuilder, SagaStep, SagaResult};

/// Статус на workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Suspended,
}

/// Конфигурация на workflow engine
#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    pub worker_count: usize,
    pub max_concurrent_workflows: usize,
    pub default_timeout: std::time::Duration,
    pub retry_policy: RetryPolicy,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            worker_count: 4,
            max_concurrent_workflows: 100,
            default_timeout: std::time::Duration::from_secs(300),
            retry_policy: RetryPolicy::default(),
        }
    }
}

/// Policy за retry при activity failure
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_interval: std::time::Duration,
    pub max_interval: std::time::Duration,
    pub backoff_coefficient: f64,
    pub non_retryable_errors: Vec<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_interval: std::time::Duration::from_secs(1),
            max_interval: std::time::Duration::from_secs(60),
            backoff_coefficient: 2.0,
            non_retryable_errors: vec![],
        }
    }
}

impl RetryPolicy {
    pub fn calculate_backoff(&self, attempt: u32) -> std::time::Duration {
        let backoff = self.initial_interval.as_secs_f64() * 
            self.backoff_coefficient.powi(attempt as i32);
        let backoff = backoff.min(self.max_interval.as_secs_f64());
        std::time::Duration::from_secs_f64(backoff)
    }
    
    pub fn should_retry(&self, attempt: u32, error: &str) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }
        
        for non_retryable in &self.non_retryable_errors {
            if error.contains(non_retryable) {
                return false;
            }
        }
        
        true
    }
}

/// Query за workflow state (external observers)
#[derive(Debug, Clone)]
pub struct WorkflowQuery<T> {
    pub query_type: String,
    pub _phantom: std::marker::PhantomData<T>,
}

/// Signal за комуникация с running workflow
#[derive(Debug, Clone)]
pub struct WorkflowSignal<T> {
    pub signal_name: String,
    pub payload: T,
}

/// Информация за workflow execution
#[derive(Debug, Clone)]
pub struct WorkflowExecutionInfo {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type: String,
    pub status: WorkflowStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub close_time: Option<chrono::DateTime<chrono::Utc>>,
    pub execution_duration: Option<std::time::Duration>,
}

/// Грешки в workflow системата
#[derive(Debug, thiserror::Error)]
pub enum TemporalError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    
    #[error("Activity failed: {0}")]
    ActivityFailed(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Cancellation requested")]
    Cancelled,
    
    #[error("Signal not received: {0}")]
    SignalNotReceived(String),
    
    #[error("Query failed: {0}")]
    QueryFailed(String),
    
    #[error("Persistence error: {0}")]
    PersistenceError(String),
    
    #[error("Saga compensation failed: {0}")]
    CompensationFailed(String),
}

/// Trading-specific workflows
pub mod trading_workflows {
    
    use serde::{Deserialize, Serialize};
    
    /// Input за signal generation workflow
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SignalGenerationInput {
        pub ticker: String,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub require_confirmation: bool,
        pub min_cq: f64,
    }
    
    /// Output от signal generation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SignalGenerationOutput {
        pub signal_id: String,
        pub ticker: String,
        pub action: String,
        pub confidence: f64,
        pub executed: bool,
        pub order_id: Option<String>,
        pub error: Option<String>,
    }
    
    /// Input за order execution
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OrderExecutionInput {
        pub signal_id: String,
        pub ticker: String,
        pub action: String,
        pub quantity: rust_decimal::Decimal,
        pub order_type: OrderType,
        pub max_slippage: f64,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum OrderType {
        Market,
        Limit(rust_decimal::Decimal),
        Stop(rust_decimal::Decimal),
        StopLimit(rust_decimal::Decimal, rust_decimal::Decimal),
    }
    
    /// Output от order execution
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OrderExecutionOutput {
        pub order_id: String,
        pub status: String,
        pub filled_quantity: rust_decimal::Decimal,
        pub avg_price: rust_decimal::Decimal,
        pub commission: rust_decimal::Decimal,
        pub execution_time_ms: u64,
    }
    
    /// Input за Phoenix training
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PhoenixTrainingInput {
        pub strategy_id: String,
        pub epochs: u32,
        pub validation_days: u32,
        pub graduation_criteria: GraduationCriteria,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GraduationCriteria {
        pub min_sharpe: f64,
        pub max_drawdown: f64,
        pub min_win_rate: f64,
        pub min_profit_factor: f64,
    }
    
    /// Output от Phoenix training
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PhoenixTrainingOutput {
        pub strategy_id: String,
        pub graduated: bool,
        pub epochs_completed: u32,
        pub final_metrics: StrategyMetrics,
        pub graduation_reason: Option<String>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StrategyMetrics {
        pub sharpe_ratio: f64,
        pub max_drawdown: f64,
        pub win_rate: f64,
        pub profit_factor: f64,
        pub total_return: f64,
    }
}

// Примерен usage:
//
// ```rust,ignore
// // Стартиране на signal generation workflow
// let client = TemporalClient::new(config).await?;
//
// let workflow_id = format!("signal-{}", Uuid::new_v4());
// let input = SignalGenerationInput {
//     ticker: "AAPL".to_string(),
//     timestamp: Utc::now(),
//     require_confirmation: true,
//     min_cq: 0.7,
// };
//
// let handle = client
//     .start_workflow::<SignalGenerationWorkflow>(&workflow_id, input)
//     .await?;
//
// // Query за текущ статус
// let status = handle.query::<WorkflowStatus>("status").await?;
//
// // Изпращаме signal (напр. потребителско потвърждение)
// handle.signal("user_confirmation", true).await?;
//
// // Чакаме резултат
// let result = handle.result().await?;
// ```
