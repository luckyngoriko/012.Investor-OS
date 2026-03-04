//! Node implementations

use super::state::SharedState;
use async_trait::async_trait;
use rust_decimal::prelude::ToPrimitive;
use std::fmt;

/// Резултат от изпълнение на node
#[derive(Debug, Clone)]
pub enum NodeOutput {
    /// Продължаваме с обновено състояние
    Continue(SharedState),
    /// Завършваме с финално състояние
    End(SharedState),
    /// Скок към друг node
    Jump(String, SharedState),
}

/// Грешка при изпълнение
#[derive(Debug, Clone)]
pub struct NodeError {
    pub message: String,
    pub recoverable: bool,
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for NodeError {}

impl NodeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: false,
        }
    }

    pub fn recoverable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            recoverable: true,
        }
    }
}

/// Trait за node
#[async_trait]
pub trait Node: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, state: SharedState) -> Result<NodeOutput, NodeError>;
}

// ==================== Trading Nodes ====================

/// Стартов node
pub struct StartNode;

#[async_trait]
impl Node for StartNode {
    fn name(&self) -> &str {
        "start"
    }

    async fn execute(&self, state: SharedState) -> Result<NodeOutput, NodeError> {
        tracing::info!("Starting trading decision for {}", state.ticker);
        Ok(NodeOutput::Continue(state))
    }
}

/// Събиране на данни
pub struct DataCollectionNode {
    market_service: Arc<dyn MarketDataService>,
}

#[async_trait]
pub trait MarketDataService: Send + Sync {
    async fn fetch_price(&self, ticker: &str) -> Option<rust_decimal::Decimal>;
    async fn fetch_ohlcv(&self, ticker: &str, days: u32) -> Vec<super::state::OHLCV>;
}

use std::sync::Arc;

impl DataCollectionNode {
    pub fn new(service: Arc<dyn MarketDataService>) -> Self {
        Self {
            market_service: service,
        }
    }
}

#[async_trait]
impl Node for DataCollectionNode {
    fn name(&self) -> &str {
        "collect_data"
    }

    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        let price = self.market_service.fetch_price(&state.ticker).await;
        let ohlcv = self.market_service.fetch_ohlcv(&state.ticker, 30).await;

        state.current_price = price;
        state.ohlcv = ohlcv;

        if state.current_price.is_none() {
            return Err(NodeError::new("Failed to fetch price"));
        }

        Ok(NodeOutput::Continue(state))
    }
}

/// Определяне на market regime
pub struct RegimeDetectionNode {
    ml_service: Arc<dyn RegimeMLService>,
}

#[async_trait]
pub trait RegimeMLService: Send + Sync {
    async fn detect(&self, ohlcv: &[super::state::OHLCV]) -> super::state::MarketRegime;
}

impl RegimeDetectionNode {
    pub fn new(service: Arc<dyn RegimeMLService>) -> Self {
        Self {
            ml_service: service,
        }
    }
}

#[async_trait]
impl Node for RegimeDetectionNode {
    fn name(&self) -> &str {
        "detect_regime"
    }

    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        if state.ohlcv.is_empty() {
            return Err(NodeError::new("No OHLCV data"));
        }

        let regime = self.ml_service.detect(&state.ohlcv).await;
        state.market_regime = regime;

        tracing::info!("Detected regime: {:?}", regime);

        Ok(NodeOutput::Continue(state))
    }
}

/// Breakout стратегия node
pub struct BreakoutStrategyNode {
    breakout_threshold: f64,
}

impl BreakoutStrategyNode {
    pub fn new(threshold: f64) -> Self {
        Self {
            breakout_threshold: threshold,
        }
    }
}

#[async_trait]
impl Node for BreakoutStrategyNode {
    fn name(&self) -> &str {
        "breakout_strategy"
    }

    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        // Изчисляваме breakout score
        let score = calculate_breakout_score(&state.ohlcv, self.breakout_threshold);
        state.breakout_score = Some(score);

        // Trend following също
        let atr = calculate_atr_trend(&state.ohlcv);
        state.atr_trend = Some(atr);

        // За trending режим, regime fit е висок
        state.regime_fit = Some(0.9);

        Ok(NodeOutput::Continue(state))
    }
}

/// Mean reversion стратегия node
pub struct MeanReversionStrategyNode;

#[async_trait]
impl Node for MeanReversionStrategyNode {
    fn name(&self) -> &str {
        "mean_reversion"
    }

    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        // Изчисляваме mean reversion signals
        let score = calculate_mean_reversion_score(&state.ohlcv);
        state.breakout_score = Some(1.0 - score); // Инвертираме

        // ATR trend е по-нисък в range-bound
        state.atr_trend = Some(0.3);

        // Regime fit за mean reversion
        state.regime_fit = Some(0.85);

        Ok(NodeOutput::Continue(state))
    }
}

/// CQ Calculation node
pub struct CQCalculationNode;

#[async_trait]
impl Node for CQCalculationNode {
    fn name(&self) -> &str {
        "cq_calculation"
    }

    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        // Проверяваме дали имаме всички необходими scores
        let required = [
            state.quality_score,
            state.insider_score,
            state.sentiment_score,
            state.regime_fit,
            state.breakout_score,
            state.atr_trend,
        ];

        if required.iter().any(|s| s.is_none()) {
            return Err(NodeError::new("Missing required scores for CQ calculation"));
        }

        state.calculate_cq();

        tracing::info!(
            "CQ calculated: {:.2} for {}",
            state.conviction_quotient.unwrap_or(0.0),
            state.ticker
        );

        Ok(NodeOutput::Continue(state))
    }
}

/// Risk check node
pub struct RiskCheckNode {
    risk_service: Arc<dyn RiskService>,
    min_cq: f64,
}

#[async_trait]
pub trait RiskService: Send + Sync {
    async fn check_risk(&self, state: &SharedState) -> RiskResult;
}

pub struct RiskResult {
    pub approved: bool,
    pub checks: Vec<super::state::RiskCheck>,
    pub suggested_size: rust_decimal::Decimal,
}

impl RiskCheckNode {
    pub fn new(service: Arc<dyn RiskService>, min_cq: f64) -> Self {
        Self {
            risk_service: service,
            min_cq,
        }
    }
}

#[async_trait]
impl Node for RiskCheckNode {
    fn name(&self) -> &str {
        "risk_check"
    }

    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        let cq = state.conviction_quotient.unwrap_or(0.0);

        // Проверка за min CQ
        if cq < self.min_cq {
            state.risk_approved = false;
            state.execution_status = super::state::ExecutionStatus::Rejected;
            return Ok(NodeOutput::End(state));
        }

        // Risk service checks
        let risk_result = self.risk_service.check_risk(&state).await;

        state.risk_approved = risk_result.approved;
        state.risk_checks = risk_result.checks;
        state.position_size = Some(risk_result.suggested_size);

        if !risk_result.approved {
            state.execution_status = super::state::ExecutionStatus::Rejected;
            return Ok(NodeOutput::End(state));
        }

        Ok(NodeOutput::Continue(state))
    }
}

/// Execution node
pub struct ExecutionNode {
    broker_service: Arc<dyn BrokerService>,
}

#[async_trait]
pub trait BrokerService: Send + Sync {
    async fn place_order(&self, order: OrderRequest) -> Result<OrderResult, String>;
}

pub struct OrderRequest {
    pub ticker: String,
    pub action: super::state::TradingAction,
    pub quantity: rust_decimal::Decimal,
}

pub struct OrderResult {
    pub order_id: String,
    pub executed_price: rust_decimal::Decimal,
    pub status: String,
}

impl ExecutionNode {
    pub fn new(service: Arc<dyn BrokerService>) -> Self {
        Self {
            broker_service: service,
        }
    }
}

#[async_trait]
impl Node for ExecutionNode {
    fn name(&self) -> &str {
        "execute"
    }

    async fn execute(&self, mut state: SharedState) -> Result<NodeOutput, NodeError> {
        let action = state
            .action
            .ok_or_else(|| NodeError::new("No trading action specified"))?;

        let size = state
            .position_size
            .ok_or_else(|| NodeError::new("No position size specified"))?;

        let order = OrderRequest {
            ticker: state.ticker.clone(),
            action,
            quantity: size,
        };

        match self.broker_service.place_order(order).await {
            Ok(result) => {
                state.order_id = Some(result.order_id);
                state.execution_price = Some(result.executed_price);
                state.execution_status = super::state::ExecutionStatus::Completed;

                tracing::info!(
                    "Order executed: {} {} @ {}",
                    state.ticker,
                    size,
                    result.executed_price
                );

                Ok(NodeOutput::End(state))
            }
            Err(e) => {
                state.execution_status = super::state::ExecutionStatus::Failed;
                Err(NodeError::new(format!("Order failed: {}", e)))
            }
        }
    }
}

// ==================== Helper Functions ====================

fn calculate_breakout_score(ohlcv: &[super::state::OHLCV], threshold: f64) -> f64 {
    if ohlcv.len() < 20 {
        return 0.5;
    }

    // Simplified breakout detection
    let recent = &ohlcv[ohlcv.len() - 5..];
    let highs: Vec<_> = recent.iter().map(|c| c.high).collect();
    let max_high = highs.iter().max().unwrap();
    let min_high = highs.iter().min().unwrap();

    let range = (max_high - min_high).to_f64().unwrap_or(0.0);

    (range / threshold).min(1.0)
}

fn calculate_atr_trend(ohlcv: &[super::state::OHLCV]) -> f64 {
    if ohlcv.len() < 14 {
        return 0.5;
    }

    // Simplified ATR trend
    let atr = calculate_atr(ohlcv, 14);
    let current_price = ohlcv.last().unwrap().close;

    let atr_percent = (atr / current_price).to_f64().unwrap_or(0.0);

    // Нормализираме към 0-1
    (atr_percent * 10.0).min(1.0)
}

fn calculate_atr(ohlcv: &[super::state::OHLCV], period: usize) -> rust_decimal::Decimal {
    let mut tr_values = vec![];

    for i in 1..ohlcv.len() {
        let high_low = ohlcv[i].high - ohlcv[i].low;
        let high_close = (ohlcv[i].high - ohlcv[i - 1].close).abs();
        let low_close = (ohlcv[i].low - ohlcv[i - 1].close).abs();

        let tr = high_low.max(high_close).max(low_close);
        tr_values.push(tr);
    }

    // Simple average
    let sum: rust_decimal::Decimal = tr_values.iter().rev().take(period).sum();
    sum / rust_decimal::Decimal::from(period)
}

fn calculate_mean_reversion_score(ohlcv: &[super::state::OHLCV]) -> f64 {
    if ohlcv.len() < 20 {
        return 0.5;
    }

    // Calculate distance from SMA
    let sma = ohlcv
        .iter()
        .rev()
        .take(20)
        .map(|c| c.close)
        .sum::<rust_decimal::Decimal>()
        / rust_decimal::Decimal::from(20);

    let current = ohlcv.last().unwrap().close;
    let deviation = ((current - sma) / sma).to_f64().unwrap_or(0.0);

    // Нормализираме
    (deviation.abs() * 5.0).min(1.0)
}
