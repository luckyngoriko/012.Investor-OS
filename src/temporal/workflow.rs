//! Workflow trait and implementations

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::marker::PhantomData;

use super::{TemporalError, WorkflowStatus};

/// Основен workflow trait
#[async_trait]
pub trait Workflow: Send + Sync + 'static {
    type Input: Serialize + DeserializeOwned + Send + Sync;
    type Output: Serialize + DeserializeOwned + Send + Sync;
    
    /// Уникално име на workflow типа
    fn name() -> &'static str where Self: Sized;
    
    /// Главна логика
    async fn run(
        &self,
        ctx: super::context::WorkflowContext,
        input: Self::Input,
    ) -> Result<Self::Output, TemporalError>;
}

/// Workflow handle за управление на running workflow
pub struct WorkflowHandle<W: Workflow> {
    workflow_id: String,
    run_id: String,
    client: Arc<dyn WorkflowClient>,
    _phantom: PhantomData<W>,
}

use std::sync::Arc;

/// Workflow client for handles - uses erased types for object safety
#[async_trait]
pub trait WorkflowClient: Send + Sync {
    async fn query_raw(&self, workflow_id: &str, query_type: &str) -> Result<serde_json::Value, TemporalError>;
    async fn signal_raw(&self, workflow_id: &str, signal_name: &str, payload: serde_json::Value) -> Result<(), TemporalError>;
    async fn cancel(&self, workflow_id: &str) -> Result<(), TemporalError>;
    async fn get_result_raw(&self, workflow_id: &str) -> Result<serde_json::Value, TemporalError>;
    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus, TemporalError>;
}

impl<W: Workflow> WorkflowHandle<W> {
    pub fn new(
        workflow_id: impl Into<String>,
        run_id: impl Into<String>,
        client: Arc<dyn WorkflowClient>,
    ) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            run_id: run_id.into(),
            client,
            _phantom: PhantomData,
        }
    }
    
    /// Query за текущо състояние
    pub async fn query<T: DeserializeOwned>(&self, query_type: &str) -> Result<T, TemporalError> {
        let raw = self.client.query_raw(&self.workflow_id, query_type).await?;
        serde_json::from_value(raw).map_err(|e| TemporalError::QueryFailed(e.to_string()))
    }
    
    /// Изпращане на signal
    pub async fn signal<T: Serialize>(&self, signal_name: &str, payload: T) -> Result<(), TemporalError> {
        let raw = serde_json::to_value(payload).map_err(|e| TemporalError::ActivityFailed(e.to_string()))?;
        self.client.signal_raw(&self.workflow_id, signal_name, raw).await
    }
    
    /// Cancel workflow
    pub async fn cancel(&self) -> Result<(), TemporalError> {
        self.client.cancel(&self.workflow_id).await
    }
    
    /// Чакане за резултат
    pub async fn result(&self) -> Result<W::Output, TemporalError> {
        let raw = self.client.get_result_raw(&self.workflow_id).await?;
        serde_json::from_value(raw).map_err(|e| TemporalError::QueryFailed(e.to_string()))
    }
    
    /// Текущ статус
    pub async fn status(&self) -> Result<WorkflowStatus, TemporalError> {
        self.client.get_status(&self.workflow_id).await
    }
}

/// Signal types за trading workflows
pub mod signals {
    use super::*;
    
    /// User confirmation signal
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserConfirmation {
        pub confirmed: bool,
        pub user_id: String,
        pub timestamp: chrono::DateTime<chrono::Utc>,
    }
    
    /// Kill switch signal
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct KillSwitch {
        pub triggered: bool,
        pub reason: String,
        pub triggered_by: String,
    }
    
    /// Manual override signal
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ManualOverride {
        pub action: String,
        pub new_params: serde_json::Value,
    }
    
    /// Market data update signal
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MarketDataUpdate {
        pub ticker: String,
        pub price: rust_decimal::Decimal,
        pub timestamp: chrono::DateTime<chrono::Utc>,
    }
}

/// Query types за trading workflows
pub mod queries {
    use super::*;
    
    /// Текущ прогрес на workflow
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Progress {
        pub current_step: String,
        pub steps_completed: u32,
        pub steps_total: u32,
        pub percentage: f32,
    }
    
    /// Текущо състояние
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CurrentState {
        pub status: WorkflowStatus,
        pub started_at: chrono::DateTime<chrono::Utc>,
        pub duration_seconds: u64,
        pub metadata: serde_json::Value,
    }
}

/// Помощни функции за workflow composition
pub struct WorkflowComposer;

impl WorkflowComposer {
    /// Sequence: workflow1 → workflow2
    pub async fn sequence<W1, W2>(
        ctx: &super::context::WorkflowContext,
        w1: &W1,
        input1: W1::Input,
        w2: &W2,
        transform: impl FnOnce(W1::Output) -> W2::Input,
    ) -> Result<W2::Output, TemporalError>
    where
        W1: Workflow,
        W2: Workflow,
    {
        let output1 = w1.run(ctx.clone(), input1).await?;
        let input2 = transform(output1);
        w2.run(ctx.clone(), input2).await
    }
    
    /// Parallel: [workflow1, workflow2] → merge
    pub async fn parallel<W1, W2, R>(
        ctx: &super::context::WorkflowContext,
        w1: &W1,
        input1: W1::Input,
        w2: &W2,
        input2: W2::Input,
        merge: impl FnOnce(W1::Output, W2::Output) -> R,
    ) -> Result<R, TemporalError>
    where
        W1: Workflow,
        W2: Workflow,
    {
        let (result1, result2) = tokio::join!(
            w1.run(ctx.clone(), input1),
            w2.run(ctx.clone(), input2)
        );
        
        let output1 = result1?;
        let output2 = result2?;
        
        Ok(merge(output1, output2))
    }
}
