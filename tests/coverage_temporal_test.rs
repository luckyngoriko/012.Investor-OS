//! Coverage tests for Temporal implementations
//!
//! Fills gaps from coverage analysis

use investor_os::temporal::{
    activity::{ActivityContext, ActivityError},
    RetryPolicy, TemporalError, WorkflowContext, WorkflowStatus,
};
use std::time::Duration;

#[test]
fn test_retry_policy_max_interval() {
    let policy = RetryPolicy {
        max_attempts: 10,
        initial_interval: Duration::from_secs(1),
        max_interval: Duration::from_secs(5),
        backoff_coefficient: 2.0,
        non_retryable_errors: vec![],
    };

    // Backoff should not exceed max_interval
    let backoff_high = policy.calculate_backoff(100);
    assert!(backoff_high <= Duration::from_secs(5));
}

#[test]
fn test_activity_context_elapsed() {
    let ctx = ActivityContext::new();

    // Should be very small (just created)
    let elapsed = ctx.elapsed();
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn test_activity_context_time_remaining_no_deadline() {
    let ctx = ActivityContext::new();

    // No deadline = always has time
    assert!(ctx.has_time_remaining());
}

#[tokio::test]
async fn test_workflow_context_sleep() {
    let ctx = WorkflowContext::new("wf-1", "run-1");

    let start = std::time::Instant::now();
    ctx.sleep(Duration::from_millis(10)).await;
    let elapsed = start.elapsed();

    assert!(elapsed >= Duration::from_millis(10));
}

#[tokio::test]
async fn test_workflow_context_sleep_until() {
    let ctx = WorkflowContext::new("wf-1", "run-1");

    let target = chrono::Utc::now() + chrono::Duration::milliseconds(10);
    ctx.sleep_until(target).await;

    // Should have waited
    assert!(chrono::Utc::now() >= target);
}

#[test]
fn test_workflow_context_elapsed() {
    let ctx = WorkflowContext::new("wf-1", "run-1");

    // Sleep a tiny bit
    std::thread::sleep(Duration::from_millis(5));

    let elapsed = ctx.elapsed();
    assert!(elapsed >= Duration::from_millis(5));
}

#[test]
fn test_activity_error_with_details() {
    let error =
        ActivityError::application("Base error").with_details(serde_json::json!({"code": 500}));

    assert_eq!(error.message, "Base error");
    assert!(!error.retryable);
    assert_eq!(error.details, Some(serde_json::json!({"code": 500})));
}

#[test]
fn test_temporal_error_variants() {
    let not_found = TemporalError::WorkflowNotFound("wf-123".to_string());
    assert!(matches!(not_found, TemporalError::WorkflowNotFound(_)));

    let activity_failed = TemporalError::ActivityFailed("DB error".to_string());
    assert!(matches!(activity_failed, TemporalError::ActivityFailed(_)));

    let timeout = TemporalError::Timeout("deadline exceeded".to_string());
    assert!(matches!(timeout, TemporalError::Timeout(_)));

    let cancelled = TemporalError::Cancelled;
    assert!(matches!(cancelled, TemporalError::Cancelled));
}

#[test]
fn test_workflow_status_display() {
    use investor_os::temporal::WorkflowStatus::*;

    // Just verify all variants exist and can be compared
    assert_ne!(Pending, Running);
    assert_ne!(Running, Completed);
    assert_ne!(Completed, Failed);
    assert_ne!(Failed, Cancelled);
    assert_ne!(Cancelled, Suspended);
}

#[test]
fn test_retry_policy_default() {
    let policy = RetryPolicy::default();

    assert_eq!(policy.max_attempts, 3);
    assert_eq!(policy.initial_interval, Duration::from_secs(1));
    assert_eq!(policy.max_interval, Duration::from_secs(60));
    assert_eq!(policy.backoff_coefficient, 2.0);
}

#[test]
fn test_activity_context_default() {
    let ctx = ActivityContext::new();

    assert_eq!(ctx.attempt, 1);
    assert!(!ctx.is_retry());
    assert!(ctx.heartbeat_details.is_empty());
}

#[test]
fn test_activity_context_with_attempt() {
    let ctx = ActivityContext::new().with_attempt(3);

    assert_eq!(ctx.attempt, 3);
    assert!(ctx.is_retry());
}

#[test]
fn test_workflow_context_is_cancelling() {
    let ctx = WorkflowContext::new("wf-1", "run-1");

    // Default is not cancelling
    assert!(!ctx.is_cancelling());
}
