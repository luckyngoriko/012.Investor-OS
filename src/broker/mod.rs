//! Broker Integration Module
//!
//! Sprint 6: Interactive Brokers Integration
//! - S6-D1: IB Client Setup (Connection to IB Gateway/Client Portal)
//! - S6-D2: Paper Trading Mode
//! - S6-D3: Order Management (Place, modify, cancel)
//! - S6-D4: Position Sync (Real-time reconciliation)
//! - S6-D5: Risk Pre-checks (Validate orders)
//! - S6-D6: Execution Engine (Auto-execute proposals)
//! - S6-D7: Order Journal (Log all broker interactions)
//! - S6-D8: Kill Switch (Immediate position flattening)

pub mod execution;
pub mod ib;
pub mod orders;
pub mod risk;

/// Sprint 11: Crypto Trading (Binance)
pub mod binance;

/// Sprint 11: Forex Trading (OANDA)
pub mod oanda;

/// Sprint 11: Multi-Asset Portfolio Management
pub mod multi_asset;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Broker-specific errors
#[derive(Error, Debug, Clone)]
pub enum BrokerError {
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Authentication failed: {0}")]
    Authentication(String),
    
    #[error("Order rejected: {0}")]
    OrderRejected(String),
    
    #[error("Invalid order: {0}")]
    InvalidOrder(String),
    
    #[error("Risk check failed: {0}")]
    RiskCheckFailed(String),
    
    #[error("Position not found: {0}")]
    PositionNotFound(String),
    
    #[error("External API error: {0}")]
    ExternalApi(String),
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Market closed")]
    MarketClosed,
    
    #[error("Insufficient funds")]
    InsufficientFunds,
}

/// Result type for broker operations
pub type Result<T> = std::result::Result<T, BrokerError>;

/// Broker account configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerConfig {
    /// Broker type
    pub broker_type: BrokerType,
    /// Account ID
    pub account_id: String,
    /// API base URL
    pub api_url: String,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Paper trading mode
    pub paper_trading: bool,
    /// Default order type
    pub default_order_type: OrderType,
    /// Maximum position size (in dollars)
    pub max_position_size: Decimal,
    /// Maximum order size
    pub max_order_size: Decimal,
    /// Daily loss limit
    pub daily_loss_limit: Decimal,
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            broker_type: BrokerType::InteractiveBrokers,
            account_id: String::new(),
            api_url: "https://localhost:5000/v1/api".to_string(), // IB Gateway default
            auth_token: None,
            paper_trading: true, // Default to paper trading for safety
            default_order_type: OrderType::Limit,
            max_position_size: Decimal::from(100000), // $100k default
            max_order_size: Decimal::from(50000),     // $50k default
            daily_loss_limit: Decimal::from(5000),    // $5k default
        }
    }
}

/// Supported broker types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrokerType {
    InteractiveBrokers,
    // Future: Alpaca, TD Ameritrade, etc.
}

/// Order side (buy/sell)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        }
    }
    
    /// Reverse the side (buy -> sell, sell -> buy)
    pub fn reverse(&self) -> Self {
        match self {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        }
    }
}

/// Order types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
    TrailingStop,
}

impl OrderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderType::Market => "MKT",
            OrderType::Limit => "LMT",
            OrderType::Stop => "STP",
            OrderType::StopLimit => "STP_LMT",
            OrderType::TrailingStop => "TRAIL",
        }
    }
}

/// Time in force for orders
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Default)]
pub enum TimeInForce {
    #[default]
    Day,
    Gtc, // Good Till Canceled
    Ioc, // Immediate or Cancel
    Fok, // Fill or Kill
    OpG, // Opening
    Cls, // Closing
}


/// Order status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    PendingSubmit,
    PreSubmitted,
    Submitted,
    Filled,
    PartiallyFilled,
    Cancelled,
    PendingCancel,
    ApiPending,
    ApiCancelled,
    Inactive,
    Rejected,
}

/// An order to be placed with the broker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub broker_order_id: Option<String>,
    pub ticker: String,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub order_type: OrderType,
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub time_in_force: TimeInForce,
    pub status: OrderStatus,
    pub filled_quantity: Decimal,
    pub avg_fill_price: Option<Decimal>,
    pub commission: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub proposal_id: Option<Uuid>, // Link to CQ proposal
    pub portfolio_id: Uuid,
    pub notes: Option<String>,
}

impl Order {
    /// Create a new order
    pub fn new(
        ticker: impl Into<String>,
        side: OrderSide,
        quantity: Decimal,
        order_type: OrderType,
        portfolio_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            broker_order_id: None,
            ticker: ticker.into(),
            side,
            quantity,
            order_type,
            limit_price: None,
            stop_price: None,
            time_in_force: TimeInForce::default(),
            status: OrderStatus::ApiPending,
            filled_quantity: Decimal::ZERO,
            avg_fill_price: None,
            commission: None,
            created_at: now,
            updated_at: now,
            proposal_id: None,
            portfolio_id,
            notes: None,
        }
    }
    
    /// Set limit price
    pub fn with_limit_price(mut self, price: Decimal) -> Self {
        self.limit_price = Some(price);
        self
    }
    
    /// Set stop price
    pub fn with_stop_price(mut self, price: Decimal) -> Self {
        self.stop_price = Some(price);
        self
    }
    
    /// Set time in force
    pub fn with_time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = tif;
        self
    }
    
    /// Link to proposal
    pub fn with_proposal(mut self, proposal_id: Uuid) -> Self {
        self.proposal_id = Some(proposal_id);
        self
    }
    
    /// Add notes
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
    
    /// Calculate remaining quantity to fill
    pub fn remaining_quantity(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }
    
    /// Check if order is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::PendingSubmit
                | OrderStatus::PreSubmitted
                | OrderStatus::Submitted
                | OrderStatus::PartiallyFilled
        )
    }
    
    /// Check if order is filled
    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::Filled
    }
}

/// Position in a security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: Uuid,
    pub ticker: String,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub market_price: Option<Decimal>,
    pub market_value: Option<Decimal>,
    pub unrealized_pnl: Option<Decimal>,
    pub realized_pnl: Decimal,
    pub portfolio_id: Uuid,
    pub opened_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Position {
    /// Calculate position value
    pub fn market_value(&self) -> Option<Decimal> {
        self.market_price.map(|price| price * self.quantity)
    }
    
    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self) -> Option<Decimal> {
        self.market_price.map(|price| {
            (price - self.avg_cost) * self.quantity
        })
    }
}

/// Broker account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub account_id: String,
    pub cash_balance: Decimal,
    pub buying_power: Decimal,
    pub equity_with_loan: Decimal,
    pub net_liquidation: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub currency: String,
    pub updated_at: DateTime<Utc>,
}

/// Trade execution details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub id: Uuid,
    pub order_id: Uuid,
    pub broker_execution_id: String,
    pub ticker: String,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub price: Decimal,
    pub commission: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Broker trait for abstraction
#[async_trait::async_trait]
pub trait Broker: Send + Sync {
    /// Connect to broker
    async fn connect(&mut self) -> Result<()>;
    
    /// Disconnect from broker
    async fn disconnect(&mut self) -> Result<()>;
    
    /// Check if connected
    fn is_connected(&self) -> bool;
    
    /// Get account information
    async fn get_account_info(&self) -> Result<AccountInfo>;
    
    /// Get positions
    async fn get_positions(&self) -> Result<Vec<Position>>;
    
    /// Get position for specific ticker
    async fn get_position(&self, ticker: &str) -> Result<Option<Position>>;
    
    /// Place an order
    async fn place_order(&self, order: &mut Order) -> Result<()>;
    
    /// Cancel an order
    async fn cancel_order(&self, order: &mut Order) -> Result<()>;
    
    /// Modify an order
    async fn modify_order(&self, order: &mut Order, new_quantity: Option<Decimal>, new_price: Option<Decimal>) -> Result<()>;
    
    /// Get order status
    async fn get_order_status(&self, order: &mut Order) -> Result<OrderStatus>;
    
    /// Get executions for an order
    async fn get_executions(&self, order_id: Uuid) -> Result<Vec<Execution>>;
    
    /// Get market price for ticker
    async fn get_market_price(&self, ticker: &str) -> Result<Decimal>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_creation() {
        let portfolio_id = Uuid::new_v4();
        let order = Order::new(
            "AAPL",
            OrderSide::Buy,
            Decimal::from(100),
            OrderType::Limit,
            portfolio_id,
        )
        .with_limit_price(Decimal::from(150));

        assert_eq!(order.ticker, "AAPL");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.quantity, Decimal::from(100));
        assert_eq!(order.limit_price, Some(Decimal::from(150)));
        assert_eq!(order.portfolio_id, portfolio_id);
    }

    #[test]
    fn test_order_side_reverse() {
        assert_eq!(OrderSide::Buy.reverse(), OrderSide::Sell);
        assert_eq!(OrderSide::Sell.reverse(), OrderSide::Buy);
    }

    #[test]
    fn test_order_is_active() {
        let portfolio_id = Uuid::new_v4();
        let mut order = Order::new("AAPL", OrderSide::Buy, Decimal::from(100), OrderType::Market, portfolio_id);
        
        order.status = OrderStatus::Submitted;
        assert!(order.is_active());
        
        order.status = OrderStatus::Filled;
        assert!(!order.is_active());
    }

    #[test]
    fn test_position_calculations() {
        let position = Position {
            id: Uuid::new_v4(),
            ticker: "AAPL".to_string(),
            quantity: Decimal::from(100),
            avg_cost: Decimal::from(150),
            market_price: Some(Decimal::from(160)),
            market_value: None,
            unrealized_pnl: None,
            realized_pnl: Decimal::ZERO,
            portfolio_id: Uuid::new_v4(),
            opened_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(position.market_value(), Some(Decimal::from(16000)));
        assert_eq!(position.unrealized_pnl(), Some(Decimal::from(1000)));
    }
}
