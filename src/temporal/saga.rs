//! Saga pattern for distributed transactions

use async_trait::async_trait;

use super::{TemporalError, WorkflowContext};

/// A saga is a long-running transaction with compensation
pub struct Saga {
    steps: Vec<Box<dyn SagaStep>>,
}

impl Saga {
    pub fn new() -> Self {
        Self { steps: vec![] }
    }

    pub fn add_step<S: SagaStep + 'static>(&mut self, step: S) {
        self.steps.push(Box::new(step));
    }

    pub async fn execute(&self, ctx: &WorkflowContext) -> Result<SagaResult, TemporalError> {
        let mut executed = vec![];

        for (i, step) in self.steps.iter().enumerate() {
            match step.execute(ctx).await {
                Ok(_) => {
                    executed.push(i);
                }
                Err(e) => {
                    // Compensate executed steps in reverse order
                    for &idx in executed.iter().rev() {
                        if let Err(comp_err) = self.steps[idx].compensate(ctx).await {
                            return Err(TemporalError::CompensationFailed(format!(
                                "Step {} failed: {}, compensation failed: {}",
                                i, e, comp_err
                            )));
                        }
                    }
                    return Err(TemporalError::ActivityFailed(format!(
                        "Step {} failed: {}",
                        i, e
                    )));
                }
            }
        }

        Ok(SagaResult {
            completed_steps: executed.len(),
        })
    }
}

impl Default for Saga {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of saga execution
#[derive(Debug, Clone)]
pub struct SagaResult {
    pub completed_steps: usize,
}

/// A step in a saga
#[async_trait]
pub trait SagaStep: Send + Sync {
    async fn execute(&self, ctx: &WorkflowContext) -> Result<(), String>;
    async fn compensate(&self, ctx: &WorkflowContext) -> Result<(), String>;
}

/// Builder for creating sagas
pub struct SagaBuilder {
    saga: Saga,
}

impl SagaBuilder {
    pub fn new() -> Self {
        Self { saga: Saga::new() }
    }

    pub fn step<S: SagaStep + 'static>(mut self, step: S) -> Self {
        self.saga.add_step(step);
        self
    }

    pub fn build(self) -> Saga {
        self.saga
    }
}

impl Default for SagaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Compensation function type
pub type CompensationFn = Box<
    dyn Fn(
            &WorkflowContext,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>>
        + Send
        + Sync,
>;

/// Concrete saga step with action and compensation
pub struct ConcreteStep {
    name: String,
    action: Box<
        dyn Fn(
                &WorkflowContext,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>,
            > + Send
            + Sync,
    >,
    compensation: CompensationFn,
}

impl ConcreteStep {
    pub fn new<F, Fut, C, CompFut>(name: impl Into<String>, action: F, compensation: C) -> Self
    where
        F: Fn(&WorkflowContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
        C: Fn(&WorkflowContext) -> CompFut + Send + Sync + 'static,
        CompFut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        Self {
            name: name.into(),
            action: Box::new(move |ctx| Box::pin(action(ctx))),
            compensation: Box::new(move |ctx| Box::pin(compensation(ctx))),
        }
    }
}

#[async_trait]
impl SagaStep for ConcreteStep {
    async fn execute(&self, ctx: &WorkflowContext) -> Result<(), String> {
        (self.action)(ctx).await
    }

    async fn compensate(&self, ctx: &WorkflowContext) -> Result<(), String> {
        (self.compensation)(ctx).await
    }
}

/// Example trading saga steps
pub mod trading_saga {
    use super::*;

    /// Reserve funds step
    pub struct ReserveFundsStep {
        amount: rust_decimal::Decimal,
    }

    impl ReserveFundsStep {
        pub fn new(amount: rust_decimal::Decimal) -> Self {
            Self { amount }
        }
    }

    #[async_trait]
    impl SagaStep for ReserveFundsStep {
        async fn execute(&self, _ctx: &WorkflowContext) -> Result<(), String> {
            // Reserve funds logic
            tracing::info!("Reserving funds: {}", self.amount);
            Ok(())
        }

        async fn compensate(&self, _ctx: &WorkflowContext) -> Result<(), String> {
            // Release reserved funds
            tracing::info!("Releasing reserved funds: {}", self.amount);
            Ok(())
        }
    }

    /// Place order step
    pub struct PlaceOrderStep {
        order_request: PlaceOrderRequest,
    }

    #[derive(Clone)]
    pub struct PlaceOrderRequest {
        pub ticker: String,
        pub action: String,
        pub quantity: rust_decimal::Decimal,
    }

    impl PlaceOrderStep {
        pub fn new(request: PlaceOrderRequest) -> Self {
            Self {
                order_request: request,
            }
        }
    }

    #[async_trait]
    impl SagaStep for PlaceOrderStep {
        async fn execute(&self, _ctx: &WorkflowContext) -> Result<(), String> {
            tracing::info!(
                "Placing order: {} {}",
                self.order_request.action,
                self.order_request.ticker
            );
            Ok(())
        }

        async fn compensate(&self, _ctx: &WorkflowContext) -> Result<(), String> {
            tracing::info!("Cancelling order for {}", self.order_request.ticker);
            Ok(())
        }
    }
}
