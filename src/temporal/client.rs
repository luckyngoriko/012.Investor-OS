//! Temporal client for workflow management

use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
    async fn query_raw(
        &self,
        workflow_id: &str,
        query_type: &str,
    ) -> Result<serde_json::Value, TemporalError>;
    async fn signal_raw(
        &self,
        workflow_id: &str,
        signal_name: &str,
        payload: serde_json::Value,
    ) -> Result<(), TemporalError>;
    async fn cancel(&self, workflow_id: &str) -> Result<(), TemporalError>;
    async fn get_result_raw(&self, workflow_id: &str) -> Result<serde_json::Value, TemporalError>;
    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus, TemporalError>;
}

/// In-memory implementation for testing
pub struct InMemoryClient {
    workflows: Arc<Mutex<HashMap<String, WorkflowStatus>>>,
    query_results: Arc<Mutex<HashMap<(String, String), Value>>>,
    workflow_results: Arc<Mutex<HashMap<String, Value>>>,
    signals: Arc<Mutex<HashMap<String, Vec<(String, Value)>>>>,
}

impl InMemoryClient {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(Mutex::new(HashMap::new())),
            query_results: Arc::new(Mutex::new(HashMap::new())),
            workflow_results: Arc::new(Mutex::new(HashMap::new())),
            signals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_workflow_status(&self, workflow_id: impl Into<String>, status: WorkflowStatus) {
        let workflow_id = workflow_id.into();
        self.workflows
            .lock()
            .expect("workflows mutex poisoned")
            .insert(workflow_id.clone(), status);
    }

    pub fn set_query_result(
        &self,
        workflow_id: impl Into<String>,
        query_type: impl Into<String>,
        payload: Value,
    ) {
        let workflow_id = workflow_id.into();
        let query_type = query_type.into();
        self.query_results
            .lock()
            .expect("query results mutex poisoned")
            .insert((workflow_id, query_type), payload);
    }

    pub fn set_workflow_result(&self, workflow_id: impl Into<String>, payload: Value) {
        let workflow_id = workflow_id.into();
        self.workflow_results
            .lock()
            .expect("workflow results mutex poisoned")
            .insert(workflow_id, payload);
    }

    pub fn push_signal(
        &self,
        workflow_id: impl Into<String>,
        signal_name: impl Into<String>,
        payload: Value,
    ) {
        let workflow_id = workflow_id.into();
        let signal_name = signal_name.into();
        self.signals
            .lock()
            .expect("signals mutex poisoned")
            .entry(workflow_id)
            .or_default()
            .push((signal_name, payload));
    }
}

impl Default for InMemoryClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowClient for InMemoryClient {
    async fn query_raw(
        &self,
        workflow_id: &str,
        query_type: &str,
    ) -> Result<serde_json::Value, TemporalError> {
        let workflows = self.workflows.lock().expect("workflows mutex poisoned");
        let status = workflows
            .get(workflow_id)
            .copied()
            .ok_or_else(|| TemporalError::WorkflowNotFound(workflow_id.to_string()))?;

        if query_type == "workflow_status" {
            return Ok(json!({
                "workflow_id": workflow_id,
                "status": status,
            }));
        }

        let query_results = self
            .query_results
            .lock()
            .expect("query results mutex poisoned");
        let query_results_entry: Option<&Value> =
            query_results.get(&(workflow_id.to_string(), query_type.to_string()));
        if let Some(result) = query_results_entry {
            return Ok(result.clone());
        }

        Err(TemporalError::QueryFailed(format!(
            "query '{}' not available for workflow '{}'",
            query_type, workflow_id
        )))
    }

    async fn signal_raw(
        &self,
        workflow_id: &str,
        signal_name: &str,
        payload: serde_json::Value,
    ) -> Result<(), TemporalError> {
        let workflows = self.workflows.lock().expect("workflows mutex poisoned");
        if !workflows.contains_key(workflow_id) {
            return Err(TemporalError::WorkflowNotFound(workflow_id.to_string()));
        }
        drop(workflows);

        self.push_signal(workflow_id, signal_name, payload);
        Ok(())
    }

    async fn cancel(&self, workflow_id: &str) -> Result<(), TemporalError> {
        let mut workflows = self.workflows.lock().expect("workflows mutex poisoned");
        workflows.insert(workflow_id.to_string(), WorkflowStatus::Cancelled);
        Ok(())
    }

    async fn get_result_raw(&self, workflow_id: &str) -> Result<serde_json::Value, TemporalError> {
        let workflows = self.workflows.lock().expect("workflows mutex poisoned");
        let status = workflows
            .get(workflow_id)
            .copied()
            .ok_or_else(|| TemporalError::WorkflowNotFound(workflow_id.to_string()))?;
        drop(workflows);

        if !matches!(status, WorkflowStatus::Completed) {
            return Err(TemporalError::QueryFailed(format!(
                "workflow '{}' has no result yet (status={:?})",
                workflow_id, status
            )));
        }

        let results = self
            .workflow_results
            .lock()
            .expect("workflow results mutex poisoned");
        let result = results.get(workflow_id).ok_or_else(|| {
            TemporalError::QueryFailed(format!("workflow '{}' result not available", workflow_id))
        })?;

        Ok(result.clone())
    }

    async fn get_status(&self, workflow_id: &str) -> Result<WorkflowStatus, TemporalError> {
        let workflows = self.workflows.lock().expect("workflows mutex poisoned");
        workflows
            .get(workflow_id)
            .copied()
            .ok_or_else(|| TemporalError::WorkflowNotFound(workflow_id.to_string()))
    }
}
