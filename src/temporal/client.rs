//! Temporal client for workflow management

use async_trait::async_trait;

use super::{TemporalError, WorkflowStatus};

/// Client for interacting with Temporal server
#[async_trait]
pub trait TemporalClient: Send + Sync {
    type Handle<W: super::Workflow>;
    
    /// Start a new workflow execution
    async fn start_workflow<W: super::Workflow>(
        &self,
        workflow_id: &str,
        input: W::Input,
    ) -> Result<Self::Handle<W>, TemporalError>;
    
    /// List running workflows
    async fn list_workflows(&self) -> Result<Vec<WorkflowExecutionInfo>, TemporalError>;
    
    /// Cancel a workflow
    async fn cancel_workflow(&self, workflow_id: &str) -> Result<(), TemporalError>;
}

/// Information about a workflow execution
#[derive(Debug, Clone)]
pub struct WorkflowExecutionInfo {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type: String,
    pub status: WorkflowStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

/// Workflow client trait for handles - uses erased types for object safety
#[async_trait]
pub trait WorkflowClient: Send + Sync {
    async fn query_raw(&self, workflow_id: &str, query_type: &str) -> Result<serde_json::Value, TemporalError>;
    async fn signal_raw(&self, workflow_id: &str, signal_name: &str, payload: serde_json::Value) -> Result<(), TemporalError>;
    async fn cancel(&self, workflow_id: &str) -> Result<(), TemporalError>;
    async fn get_result_raw(&self, workflow_id: &str) -> Result<serde_json::Value, TemporalError>;
    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus, TemporalError>;
}

/// In-memory implementation for testing
pub struct InMemoryClient {
    workflows: std::sync::Mutex<std::collections::HashMap<String, WorkflowStatus>>,
}

impl InMemoryClient {
    pub fn new() -> Self {
        Self {
            workflows: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowClient for InMemoryClient {
    async fn query_raw(&self, _workflow_id: &str, _query_type: &str) -> Result<serde_json::Value, TemporalError> {
        Err(TemporalError::QueryFailed("Not implemented".to_string()))
    }
    
    async fn signal_raw(&self, _workflow_id: &str, _signal_name: &str, _payload: serde_json::Value) -> Result<(), TemporalError> {
        Ok(())
    }
    
    async fn cancel(&self, workflow_id: &str) -> Result<(), TemporalError> {
        let mut workflows = self.workflows.lock().unwrap();
        workflows.insert(workflow_id.to_string(), WorkflowStatus::Cancelled);
        Ok(())
    }
    
    async fn get_result_raw(&self, _workflow_id: &str) -> Result<serde_json::Value, TemporalError> {
        Err(TemporalError::QueryFailed("Not implemented".to_string()))
    }
    
    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus, TemporalError> {
        let workflows = self.workflows.lock().unwrap();
        workflows.get(workflow_id).copied()
            .ok_or_else(|| TemporalError::WorkflowNotFound(workflow_id.to_string()))
    }
}
