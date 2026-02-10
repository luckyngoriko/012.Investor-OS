//! Shared state за LangGraph

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Основно състояние за trading граф
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedState {
    // Идентификация
    pub run_id: String,
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    
    // Input
    pub ticker: String,
    pub timestamp: DateTime<Utc>,
    
    // Market Data
    pub current_price: Option<Decimal>,
    pub ohlcv: Vec<OHLCV>,
    pub market_regime: MarketRegime,
    pub vix_level: Option<f64>,
    
    // Signals (всички 0.0 - 1.0)
    pub quality_score: Option<f64>,
    pub insider_score: Option<f64>,
    pub sentiment_score: Option<f64>,
    pub regime_fit: Option<f64>,
    pub breakout_score: Option<f64>,
    pub atr_trend: Option<f64>,
    
    // CQ
    pub conviction_quotient: Option<f64>,
    
    // Decision
    pub action: Option<TradingAction>,
    pub confidence: Option<f64>,
    pub position_size: Option<Decimal>,
    
    // Risk
    pub risk_approved: bool,
    pub risk_checks: Vec<RiskCheck>,
    
    // Execution
    pub order_id: Option<String>,
    pub execution_price: Option<Decimal>,
    pub execution_status: ExecutionStatus,
    
    // Meta
    pub current_node: String,
    pub node_history: Vec<NodeExecution>,
    pub llm_calls: u32,
    pub tool_calls: u32,
    pub errors: Vec<ExecutionError>,
    
    // User data
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SharedState {
    pub fn new(ticker: impl Into<String>) -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            session_id: String::new(),
            started_at: Utc::now(),
            ticker: ticker.into(),
            timestamp: Utc::now(),
            current_price: None,
            ohlcv: vec![],
            market_regime: MarketRegime::Unknown,
            vix_level: None,
            quality_score: None,
            insider_score: None,
            sentiment_score: None,
            regime_fit: None,
            breakout_score: None,
            atr_trend: None,
            conviction_quotient: None,
            action: None,
            confidence: None,
            position_size: None,
            risk_approved: false,
            risk_checks: vec![],
            order_id: None,
            execution_price: None,
            execution_status: ExecutionStatus::Pending,
            current_node: "start".to_string(),
            node_history: vec![],
            llm_calls: 0,
            tool_calls: 0,
            errors: vec![],
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }
    
    /// Изчислява CQ ако всички scores са налични
    pub fn calculate_cq(&mut self) -> Option<f64> {
        let cq = match (
            self.quality_score,
            self.insider_score,
            self.sentiment_score,
            self.regime_fit,
            self.breakout_score,
            self.atr_trend,
        ) {
            (Some(pegy), Some(insider), Some(sentiment), 
             Some(regime), Some(breakout), Some(atr)) => {
                // CQ v2.0 formula
                pegy * 0.20
                    + insider * 0.20
                    + sentiment * 0.15
                    + regime * 0.20
                    + breakout * 0.15
                    + atr * 0.10
            }
            _ => return None,
        };
        
        self.conviction_quotient = Some(cq);
        Some(cq)
    }
    
    /// Проверява дали трябва re-evaluation
    pub fn should_recheck(&self) -> bool {
        // Пример: ако цената се е променила значително
        // или имаме нови данни
        false
    }
    
    /// Snapshot на състоянието за logging/debugging
    pub fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            run_id: self.run_id.clone(),
            timestamp: Utc::now(),
            current_node: self.current_node.clone(),
            cq: self.conviction_quotient,
            action: self.action,
            risk_approved: self.risk_approved,
        }
    }
    
    /// Проверява дали графът е завършил успешно
    pub fn is_complete(&self) -> bool {
        matches!(self.execution_status, ExecutionStatus::Completed | ExecutionStatus::Skipped)
    }
    
    /// Добавя грешка
    pub fn add_error(&mut self, node: &str, error: &str) {
        self.errors.push(ExecutionError {
            node: node.to_string(),
            message: error.to_string(),
            timestamp: Utc::now(),
        });
    }
    
    /// Записва изпълнение на node
    pub fn record_node_execution(&mut self, node: &str, duration_ms: u64) {
        self.node_history.push(NodeExecution {
            node: node.to_string(),
            started_at: self.started_at, // Simplified
            duration_ms,
        });
        self.current_node = node.to_string();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketRegime {
    Unknown,
    Trending,      // Пазарът има ясна посока
    RangeBound,    // Странично движение
    Volatile,      // Висока волатилност
    LowLiquidity,  // Ниска ликвидност
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingAction {
    Buy,
    Sell,
    Hold,
    Reduce,
    Increase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCV {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskCheck {
    pub check_type: String,
    pub passed: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecution {
    pub node: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub node: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Snapshot за logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub run_id: String,
    pub timestamp: DateTime<Utc>,
    pub current_node: String,
    pub cq: Option<f64>,
    pub action: Option<TradingAction>,
    pub risk_approved: bool,
}

/// Builder за SharedState
pub struct StateBuilder {
    state: SharedState,
}

impl StateBuilder {
    pub fn new(ticker: impl Into<String>) -> Self {
        Self {
            state: SharedState::new(ticker),
        }
    }
    
    pub fn with_price(mut self, price: Decimal) -> Self {
        self.state.current_price = Some(price);
        self
    }
    
    pub fn with_regime(mut self, regime: MarketRegime) -> Self {
        self.state.market_regime = regime;
        self
    }
    
    pub fn with_quality_score(mut self, score: f64) -> Self {
        self.state.quality_score = Some(score);
        self
    }
    
    pub fn with_insider_score(mut self, score: f64) -> Self {
        self.state.insider_score = Some(score);
        self
    }
    
    pub fn with_sentiment_score(mut self, score: f64) -> Self {
        self.state.sentiment_score = Some(score);
        self
    }
    
    pub fn build(self) -> SharedState {
        self.state
    }
}
