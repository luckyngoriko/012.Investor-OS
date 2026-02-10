//! Integration tests for Temporal module
//!
//! Sprint 4: Temporal Core Tests

use investor_os::temporal::{
    Workflow, WorkflowContext, WorkflowStatus, TemporalError,
    activity::{Activity, ActivityError, ActivityContext, FetchMarketData, FetchMarketDataInput},
    saga::{Saga, SagaBuilder, ConcreteStep},
    RetryPolicy,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

// ==================== Workflow Tests ====================

/// Simple test workflow
struct TestWorkflow;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestInput {
    value: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestOutput {
    result: i32,
}

#[async_trait]
impl Workflow for TestWorkflow {
    type Input = TestInput;
    type Output = TestOutput;
    
    fn name() -> &'static str where Self: Sized {
        "TestWorkflow"
    }
    
    async fn run(&self, _ctx: WorkflowContext, input: Self::Input) -> Result<Self::Output, TemporalError> {
        Ok(TestOutput {
            result: input.value * 2,
        })
    }
}

#[tokio::test]
async fn test_simple_workflow() {
    let workflow = TestWorkflow;
    let ctx = WorkflowContext::new("test-1", "run-1");
    let input = TestInput { value: 21 };
    
    let result = workflow.run(ctx, input).await.unwrap();
    
    assert_eq!(result.result, 42);
}

#[tokio::test]
async fn test_workflow_context() {
    let ctx = WorkflowContext::new("wf-123", "run-456");
    
    assert_eq!(ctx.workflow_id, "wf-123");
    assert_eq!(ctx.run_id, "run-456");
    assert_eq!(ctx.attempt, 1);
}

// ==================== Activity Tests ====================

#[tokio::test]
async fn test_activity_execution() {
    let activity = FetchMarketData;
    let ctx = ActivityContext::new();
    let input = FetchMarketDataInput {
        ticker: "AAPL".to_string(),
        days: 30,
    };
    
    let result = activity.execute(ctx, input).await;
    
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.ticker, "AAPL");
}

#[tokio::test]
async fn test_activity_context() {
    let ctx = ActivityContext::new();
    
    assert_eq!(ctx.attempt, 1);
    assert!(!ctx.is_retry());
}

// ==================== Retry Policy Tests ====================

#[test]
fn test_retry_policy_backoff() {
    let policy = RetryPolicy::default();
    
    let backoff0 = policy.calculate_backoff(0);
    let backoff1 = policy.calculate_backoff(1);
    let backoff2 = policy.calculate_backoff(2);
    
    // Backoff should increase
    assert!(backoff1 >= backoff0);
    assert!(backoff2 >= backoff1);
}

#[test]
fn test_retry_policy_should_retry() {
    let policy = RetryPolicy::default();
    
    assert!(policy.should_retry(1, "Network error"));
    assert!(policy.should_retry(2, "Timeout"));
    assert!(!policy.should_retry(5, "Some error")); // Exceeds max_attempts (3)
}

#[test]
fn test_retry_policy_non_retryable() {
    let policy = RetryPolicy {
        max_attempts: 3,
        initial_interval: std::time::Duration::from_secs(1),
        max_interval: std::time::Duration::from_secs(60),
        backoff_coefficient: 2.0,
        non_retryable_errors: vec!["Invalid API key".to_string()],
    };
    
    assert!(!policy.should_retry(1, "Invalid API key"));
    assert!(policy.should_retry(1, "Network timeout"));
}

// ==================== Workflow Status Tests ====================

#[test]
fn test_workflow_status_variants() {
    let pending = WorkflowStatus::Pending;
    let running = WorkflowStatus::Running;
    let completed = WorkflowStatus::Completed;
    let failed = WorkflowStatus::Failed;
    let cancelled = WorkflowStatus::Cancelled;
    
    assert!(matches!(pending, WorkflowStatus::Pending));
    assert!(matches!(running, WorkflowStatus::Running));
    assert!(matches!(completed, WorkflowStatus::Completed));
    assert!(matches!(failed, WorkflowStatus::Failed));
    assert!(matches!(cancelled, WorkflowStatus::Cancelled));
}

// ==================== Saga Pattern Tests ====================

#[tokio::test]
async fn test_saga_success() {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    
    let step1 = ConcreteStep::new(
        "step1",
        |_ctx| async {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        |_ctx| async { Ok(()) },
    );
    
    let step2 = ConcreteStep::new(
        "step2",
        |_ctx| async {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        |_ctx| async { Ok(()) },
    );
    
    let saga = SagaBuilder::new()
        .step(step1)
        .step(step2)
        .build();
    
    let ctx = WorkflowContext::new("saga-test", "run-1");
    let result = saga.execute(&ctx).await;
    
    assert!(result.is_ok());
    assert_eq!(COUNTER.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_saga_compensation() {
    static EXECUTED: AtomicU32 = AtomicU32::new(0);
    static COMPENSATED: AtomicU32 = AtomicU32::new(0);
    
    let step1 = ConcreteStep::new(
        "step1",
        |_ctx| async {
            EXECUTED.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        |_ctx| async {
            COMPENSATED.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    );
    
    let step2 = ConcreteStep::new(
        "step2",
        |_ctx| async {
            EXECUTED.fetch_add(1, Ordering::SeqCst);
            Err("Step 2 failed".to_string())
        },
        |_ctx| async {
            COMPENSATED.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    );
    
    let saga = SagaBuilder::new()
        .step(step1)
        .step(step2)
        .build();
    
    let ctx = WorkflowContext::new("saga-test", "run-1");
    let result = saga.execute(&ctx).await;
    
    assert!(result.is_err());
    assert_eq!(EXECUTED.load(Ordering::SeqCst), 2); // Both steps attempted
    assert_eq!(COMPENSATED.load(Ordering::SeqCst), 1); // Only step1 compensated
}

// ==================== Activity Error Tests ====================

#[test]
fn test_activity_error_types() {
    let app_error = ActivityError::application("Something went wrong");
    assert!(!app_error.retryable);
    
    let retryable_error = ActivityError::retryable("Network timeout");
    assert!(retryable_error.retryable);
    
    let timeout_error = ActivityError::timeout("Request timed out");
    assert!(timeout_error.retryable);
}

// ==================== Trading Activities Tests ====================

#[tokio::test]
async fn test_calculate_cq_activity() {
    use investor_os::temporal::activity::{CalculateCQ, CalculateCQInput};
    
    let activity = CalculateCQ;
    let ctx = ActivityContext::new();
    let input = CalculateCQInput {
        ticker: "AAPL".to_string(),
        quality_score: 0.8,
        insider_score: 0.7,
        sentiment_score: 0.6,
        regime_fit: 0.9,
        breakout_score: 0.75,
        atr_trend: 0.5,
    };
    
    let result = activity.execute(ctx, input).await;
    
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.ticker, "AAPL");
    // Expected: 0.8*0.20 + 0.7*0.20 + 0.6*0.15 + 0.9*0.20 + 0.75*0.15 + 0.5*0.10 = 0.7325
    assert!((output.cq - 0.7325).abs() < 0.001);
}

#[tokio::test]
async fn test_call_llm_activity() {
    use investor_os::temporal::activity::{CallLLM, CallLLMInput};
    
    let activity = CallLLM;
    let ctx = ActivityContext::new();
    let input = CallLLMInput {
        prompt: "Analyze AAPL".to_string(),
        model: "claude".to_string(),
        max_tokens: Some(100),
        temperature: Some(0.7),
    };
    
    let result = activity.execute(ctx, input).await;
    
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.response.is_empty());
}

#[tokio::test]
async fn test_place_order_activity() {
    use investor_os::temporal::activity::{PlaceOrder, PlaceOrderInput};
    
    let activity = PlaceOrder;
    let ctx = ActivityContext::new();
    let input = PlaceOrderInput {
        ticker: "AAPL".to_string(),
        action: "BUY".to_string(),
        quantity: rust_decimal::Decimal::from(100),
        order_type: "MARKET".to_string(),
        limit_price: None,
        stop_price: None,
    };
    
    let result = activity.execute(ctx, input).await;
    
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.order_id.is_empty());
    assert_eq!(output.status, "FILLED");
}
