//! Order types and structures

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use super::venue::Venue;

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,              // Execute immediately at best price
    Limit(Decimal),      // Execute at specified price or better
    Stop(Decimal),       // Trigger at price, execute as market
    StopLimit(Decimal, Decimal), // Trigger at price, execute as limit
    TWAP { duration_secs: u64, slices: usize }, // Time-Weighted Average Price
    VWAP { duration_secs: u64 }, // Volume-Weighted Average Price
    Iceberg { displayed: Decimal }, // Show only part of order
}

/// Time in force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum TimeInForce {
    #[default]
    GTC, // Good Till Canceled
    IOC, // Immediate Or Cancel
    FOK, // Fill Or Kill
    DAY, // Day order
}


/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Open,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

/// Trading order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub time_in_force: TimeInForce,
    pub venue: Option<Venue>, // None = smart route
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: OrderMetadata,
}

/// Order metadata for tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderMetadata {
    pub strategy_id: Option<String>,
    pub parent_order_id: Option<Uuid>, // For child orders (TWAP slices)
    pub client_order_id: Option<String>,
    pub notes: Option<String>,
}

impl Order {
    /// Create new market order
    pub fn market(symbol: impl Into<String>, side: OrderSide, quantity: Decimal) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            symbol: symbol.into(),
            side,
            order_type: OrderType::Market,
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Pending,
            time_in_force: TimeInForce::IOC,
            venue: None,
            created_at: now,
            updated_at: now,
            metadata: OrderMetadata::default(),
        }
    }
    
    /// Create new limit order
    pub fn limit(
        symbol: impl Into<String>,
        side: OrderSide,
        quantity: Decimal,
        price: Decimal,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            symbol: symbol.into(),
            side,
            order_type: OrderType::Limit(price),
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Pending,
            time_in_force: TimeInForce::GTC,
            venue: None,
            created_at: now,
            updated_at: now,
            metadata: OrderMetadata::default(),
        }
    }
    
    /// Create TWAP order
    pub fn twap(
        symbol: impl Into<String>,
        side: OrderSide,
        quantity: Decimal,
        duration_secs: u64,
        slices: usize,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            symbol: symbol.into(),
            side,
            order_type: OrderType::TWAP { duration_secs, slices },
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Pending,
            time_in_force: TimeInForce::DAY,
            venue: None,
            created_at: now,
            updated_at: now,
            metadata: OrderMetadata::default(),
        }
    }
    
    /// Create VWAP order
    pub fn vwap(
        symbol: impl Into<String>,
        side: OrderSide,
        quantity: Decimal,
        duration_secs: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            symbol: symbol.into(),
            side,
            order_type: OrderType::VWAP { duration_secs },
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Pending,
            time_in_force: TimeInForce::DAY,
            venue: None,
            created_at: now,
            updated_at: now,
            metadata: OrderMetadata::default(),
        }
    }
    
    /// Remaining quantity to fill
    pub fn remaining(&self) -> Decimal {
        self.quantity - self.filled_quantity
    }
    
    /// Check if order is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.status, OrderStatus::Filled | OrderStatus::Canceled | OrderStatus::Rejected | OrderStatus::Expired)
    }
    
    /// Update filled quantity
    pub fn fill(&mut self, amount: Decimal) {
        self.filled_quantity += amount;
        self.updated_at = Utc::now();
        
        if self.filled_quantity >= self.quantity {
            self.status = OrderStatus::Filled;
            self.filled_quantity = self.quantity; // Cap at quantity
        } else if self.filled_quantity > Decimal::ZERO {
            self.status = OrderStatus::PartiallyFilled;
        }
    }
    
    /// Set venue preference
    pub fn with_venue(mut self, venue: Venue) -> Self {
        self.venue = Some(venue);
        self
    }
    
    /// Set metadata
    pub fn with_metadata(mut self, metadata: OrderMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Fill/execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub id: Uuid,
    pub order_id: Uuid,
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub price: Decimal,
    pub venue: Venue,
    pub timestamp: DateTime<Utc>,
    pub fees: Decimal,
}

impl Fill {
    pub fn notional(&self) -> Decimal {
        self.quantity * self.price
    }
}

/// Order slice (for algorithmic execution)
#[derive(Debug, Clone)]
pub struct OrderSlice {
    pub parent_order_id: Uuid,
    pub slice_number: usize,
    pub total_slices: usize,
    pub quantity: Decimal,
    pub target_time: DateTime<Utc>,
    pub executed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_market_order_creation() {
        let order = Order::market("BTC", OrderSide::Buy, Decimal::from(10));
        
        assert_eq!(order.symbol, "BTC");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.quantity, Decimal::from(10));
        assert!(matches!(order.order_type, OrderType::Market));
        assert_eq!(order.status, OrderStatus::Pending);
    }
    
    #[test]
    fn test_limit_order_creation() {
        let order = Order::limit("ETH", OrderSide::Sell, Decimal::from(5), Decimal::from(3000));
        
        assert_eq!(order.symbol, "ETH");
        assert!(matches!(order.order_type, OrderType::Limit(price) if price == Decimal::from(3000)));
    }
    
    #[test]
    fn test_order_fill() {
        let mut order = Order::market("BTC", OrderSide::Buy, Decimal::from(10));
        
        assert_eq!(order.remaining(), Decimal::from(10));
        
        order.fill(Decimal::from(3));
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.remaining(), Decimal::from(7));
        
        order.fill(Decimal::from(7));
        assert_eq!(order.status, OrderStatus::Filled);
        assert_eq!(order.remaining(), Decimal::ZERO);
    }
    
    #[test]
    fn test_twap_order() {
        let order = Order::twap("BTC", OrderSide::Buy, Decimal::from(100), 3600, 10);
        
        assert!(matches!(order.order_type, OrderType::TWAP { duration_secs: 3600, slices: 10 }));
        assert_eq!(order.time_in_force, TimeInForce::DAY);
    }
    
    #[test]
    fn test_fill_notional() {
        let fill = Fill {
            id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            symbol: "BTC".to_string(),
            side: OrderSide::Buy,
            quantity: Decimal::from(2),
            price: Decimal::from(50000),
            venue: Venue::Binance,
            timestamp: Utc::now(),
            fees: Decimal::from(100),
        };
        
        assert_eq!(fill.notional(), Decimal::from(100000));
    }
}
