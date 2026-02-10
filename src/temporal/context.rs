//! Workflow and activity context

use super::{Activity, RetryPolicy, TemporalError, Workflow};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::Pin;

/// Context available during workflow execution
#[derive(Debug, Clone)]
pub struct WorkflowContext {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type: String,
    pub started_at: DateTime<Utc>,
    pub attempt: u32,
    pub continued_from: Option<String>,
}

impl WorkflowContext {
    pub fn new(workflow_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            run_id: run_id.into(),
            workflow_type: String::new(),
            started_at: Utc::now(),
            attempt: 1,
            continued_from: None,
        }
    }
    
    /// Execute an activity
    pub async fn activity<A: Activity>(
        &self,
        activity: A,
        input: A::Input,
    ) -> Result<A::Output, TemporalError> {
        let executor = super::activity::ActivityExecutor::new(RetryPolicy::default());
        executor.execute(&activity, input).await
    }
    
    /// Execute an activity with custom retry policy
    pub async fn activity_with_retry<A: Activity>(
        &self,
        activity: A,
        input: A::Input,
        retry_policy: RetryPolicy,
    ) -> Result<A::Output, TemporalError> {
        let executor = super::activity::ActivityExecutor::new(retry_policy);
        executor.execute(&activity, input).await
    }
    
    /// Sleep for a duration
    pub async fn sleep(&self, duration: std::time::Duration) {
        tokio::time::sleep(duration).await;
    }
    
    /// Sleep until a specific time
    pub async fn sleep_until(&self, deadline: DateTime<Utc>) {
        let now = Utc::now();
        if deadline > now {
            let duration = (deadline - now).to_std().unwrap_or_default();
            tokio::time::sleep(duration).await;
        }
    }
    
    /// Wait for a signal
    pub async fn wait_for_signal<T: DeserializeOwned>(
        &self,
        signal_name: &str,
    ) -> Result<T, TemporalError> {
        // Would integrate with actual signal system
        // For now, this is a placeholder
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Err(TemporalError::SignalNotReceived(signal_name.to_string()))
    }
    
    /// Wait for a signal with timeout
    pub async fn wait_for_signal_with_timeout<T: DeserializeOwned>(
        &self,
        signal_name: &str,
        timeout: std::time::Duration,
    ) -> Result<Option<T>, TemporalError> {
        // Would use tokio::time::timeout with actual signal system
        let result = tokio::time::timeout(timeout, self.wait_for_signal::<T>(signal_name)).await;
        
        match result {
            Ok(Ok(value)) => Ok(Some(value)),
            Ok(Err(TemporalError::SignalNotReceived(_))) => Ok(None),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(None), // Timeout
        }
    }
    
    /// Query external state
    pub async fn query<T: DeserializeOwned>(
        &self,
        query_type: &str,
    ) -> Result<T, TemporalError> {
        // Would query workflow state
        Err(TemporalError::QueryFailed(format!(
            "Query '{}' not implemented", query_type
        )))
    }
    
    /// Start a child workflow
    pub async fn child_workflow<W: Workflow>(
        &self,
        workflow: W,
        input: W::Input,
    ) -> Result<W::Output, TemporalError> {
        let child_ctx = WorkflowContext::new(
            format!("{}-child-{}", self.workflow_id, uuid::Uuid::new_v4()),
            uuid::Uuid::new_v4().to_string(),
        );
        
        workflow.run(child_ctx, input).await
    }
    
    /// Execute a side effect (non-deterministic operation)
    pub async fn side_effect<T, F>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        // In real Temporal, this would be recorded and replayed
        f()
    }
    
    /// Get current time (workflow-safe)
    pub fn now(&self) -> DateTime<Utc> {
        // In real Temporal, this returns the workflow's current time
        Utc::now()
    }
    
    /// Check if workflow is being cancelled
    pub fn is_cancelling(&self) -> bool {
        // Would check cancellation state
        false
    }
    
    /// Record a heartbeat (for long-running activities)
    pub async fn heartbeat(&self, _details: &[u8]) {
        tracing::debug!(workflow_id = %self.workflow_id, "Heartbeat");
    }
    
    /// Get elapsed time since workflow start
    pub fn elapsed(&self) -> std::time::Duration {
        let now = Utc::now();
        (now - self.started_at).to_std().unwrap_or_default()
    }
}

/// Context available during activity execution
#[derive(Debug, Clone)]
pub struct ActivityContext {
    pub activity_id: String,
    pub activity_type: String,
    pub attempt: u32,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub heartbeat_details: Vec<u8>,
    pub workflow_execution: WorkflowExecutionInfo,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionInfo {
    pub workflow_id: String,
    pub run_id: String,
    pub workflow_type: String,
}

impl ActivityContext {
    pub fn new(activity_id: impl Into<String>, activity_type: impl Into<String>) -> Self {
        Self {
            activity_id: activity_id.into(),
            activity_type: activity_type.into(),
            attempt: 1,
            scheduled_at: Utc::now(),
            started_at: Utc::now(),
            deadline: None,
            heartbeat_details: vec![],
            workflow_execution: WorkflowExecutionInfo {
                workflow_id: String::new(),
                run_id: String::new(),
                workflow_type: String::new(),
            },
        }
    }
    
    /// Check if this is a retry
    pub fn is_retry(&self) -> bool {
        self.attempt > 1
    }
    
    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        let now = Utc::now();
        (now - self.started_at).to_std().unwrap_or_default()
    }
    
    /// Check if deadline is approaching
    pub fn is_deadline_near(&self, buffer: std::time::Duration) -> bool {
        match self.deadline {
            Some(deadline) => {
                let now = Utc::now();
                let buffer_chrono = chrono::Duration::from_std(buffer).unwrap_or_default();
                now + buffer_chrono >= deadline
            }
            None => false,
        }
    }
    
    /// Record a heartbeat
    pub async fn heartbeat(&self, _details: &[u8]) {
        tracing::debug!(
            activity_id = %self.activity_id,
            activity_type = %self.activity_type,
            "Activity heartbeat"
        );
    }
    
    /// Check if activity has been cancelled
    pub fn is_cancelled(&self) -> bool {
        // Would check cancellation state
        false
    }
    
    /// Get time remaining before deadline
    pub fn time_remaining(&self) -> Option<std::time::Duration> {
        self.deadline.map(|deadline| {
            let now = Utc::now();
            if deadline > now {
                (deadline - now).to_std().unwrap_or_default()
            } else {
                std::time::Duration::from_secs(0)
            }
        })
    }
}

/// Future type for async operations in workflows
pub type WorkflowFuture<T> = Pin<Box<dyn Future<Output = Result<T, TemporalError>> + Send>>;
