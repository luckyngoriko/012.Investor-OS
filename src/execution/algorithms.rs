//! Algorithmic execution (TWAP, VWAP, Iceberg)

use rust_decimal::Decimal;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use chrono::Utc;

use super::order::{Order, OrderSlice, OrderType, Fill};
use super::error::{ExecutionError, Result};

/// TWAP Executor - Time-Weighted Average Price
#[derive(Debug)]
pub struct TWAPExecutor;

impl TWAPExecutor {
    pub fn new() -> Self {
        Self
    }
    
    /// Execute order using TWAP algorithm
    /// Splits order into equal slices over time
    pub async fn execute<F, Fut>(
        &self,
        order: &Order,
        execute_slice: F,
    ) -> Result<Vec<Fill>>
    where
        F: Fn(Decimal) -> Fut,
        Fut: std::future::Future<Output = Result<Fill>>,
    {
        let (duration_secs, slices) = match order.order_type {
            OrderType::TWAP { duration_secs, slices } => (duration_secs, slices),
            _ => return Err(ExecutionError::AlgorithmError(
                "Order is not TWAP type".to_string()
            )),
        };
        
        if slices == 0 {
            return Err(ExecutionError::AlgorithmError(
                "TWAP slices must be > 0".to_string()
            ));
        }
        
        let slice_qty = order.quantity / Decimal::from(slices as i64);
        let interval_ms = (duration_secs * 1000) / slices as u64;
        
        info!(
            "Starting TWAP execution: {} slices of {} every {}ms",
            slices, slice_qty, interval_ms
        );
        
        let mut fills = Vec::new();
        
        for i in 0..slices {
            // Wait for interval (skip on first slice)
            if i > 0 {
                sleep(Duration::from_millis(interval_ms)).await;
            }
            
            // Execute slice
            match execute_slice(slice_qty).await {
                Ok(fill) => {
                    info!("TWAP slice {}/{} filled: {} @ ${}", 
                        i + 1, slices, fill.quantity, fill.price);
                    fills.push(fill);
                }
                Err(e) => {
                    warn!("TWAP slice {}/{} failed: {}", i + 1, slices, e);
                    // Continue with next slice (TWAP is forgiving)
                }
            }
        }
        
        if fills.is_empty() {
            return Err(ExecutionError::ExecutionFailed(
                "No TWAP slices filled".to_string()
            ));
        }
        
        info!("TWAP completed: {}/{} slices filled", fills.len(), slices);
        Ok(fills)
    }
    
    /// Generate TWAP slices without executing
    pub fn generate_slices(&self, order: &Order) -> Result<Vec<OrderSlice>> {
        let (duration_secs, slices) = match order.order_type {
            OrderType::TWAP { duration_secs, slices } => (duration_secs, slices),
            _ => return Err(ExecutionError::AlgorithmError(
                "Order is not TWAP type".to_string()
            )),
        };
        
        let slice_qty = order.quantity / Decimal::from(slices as i64);
        let interval_secs = duration_secs / slices as u64;
        let now = Utc::now();
        
        let mut order_slices = Vec::new();
        for i in 0..slices {
            order_slices.push(OrderSlice {
                parent_order_id: order.id,
                slice_number: i + 1,
                total_slices: slices,
                quantity: slice_qty,
                target_time: now + chrono::Duration::seconds((i as u64 * interval_secs) as i64),
                executed: false,
            });
        }
        
        Ok(order_slices)
    }
}

impl Default for TWAPExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// VWAP Executor - Volume-Weighted Average Price
/// Executes more when market volume is higher
#[derive(Debug)]
pub struct VWAPExecutor;

impl VWAPExecutor {
    pub fn new() -> Self {
        Self
    }
    
    /// Execute order using VWAP algorithm
    /// Slices weighted by expected volume profile
    pub async fn execute<F, Fut>(
        &self,
        order: &Order,
        volume_profile: &[Decimal], // Percentage of daily volume per bucket
        execute_slice: F,
    ) -> Result<Vec<Fill>>
    where
        F: Fn(Decimal) -> Fut,
        Fut: std::future::Future<Output = Result<Fill>>,
    {
        let duration_secs = match order.order_type {
            OrderType::VWAP { duration_secs } => duration_secs,
            _ => return Err(ExecutionError::AlgorithmError(
                "Order is not VWAP type".to_string()
            )),
        };
        
        if volume_profile.is_empty() {
            return Err(ExecutionError::AlgorithmError(
                "Volume profile required for VWAP".to_string()
            ));
        }
        
        let total_weight: Decimal = volume_profile.iter().sum();
        let slices = volume_profile.len();
        let interval_ms = (duration_secs * 1000) / slices as u64;
        
        info!(
            "Starting VWAP execution: {} slices over {}s",
            slices, duration_secs
        );
        
        let mut fills = Vec::new();
        
        for (i, weight) in volume_profile.iter().enumerate() {
            if i > 0 {
                sleep(Duration::from_millis(interval_ms)).await;
            }
            
            let slice_qty = order.quantity * (*weight) / total_weight;
            if slice_qty.is_zero() {
                continue;
            }
            
            match execute_slice(slice_qty).await {
                Ok(fill) => {
                    info!("VWAP slice {}/{} filled: {}", i + 1, slices, fill.quantity);
                    fills.push(fill);
                }
                Err(e) => {
                    warn!("VWAP slice {}/{} failed: {}", i + 1, slices, e);
                }
            }
        }
        
        if fills.is_empty() {
            return Err(ExecutionError::ExecutionFailed(
                "No VWAP slices filled".to_string()
            ));
        }
        
        Ok(fills)
    }
    
    /// Generate default intraday volume profile
    /// U-shape: higher volume at open and close
    pub fn default_intraday_profile(buckets: usize) -> Vec<Decimal> {
        let mut profile = Vec::with_capacity(buckets);
        
        for i in 0..buckets {
            let bucket = i as f64 / buckets as f64;
            // U-shape: higher at start (0) and end (1)
            let weight = (bucket - 0.5).abs() * 2.0; // 0 at midday, 1 at open/close
            let weight = 0.5 + weight * 0.5; // Scale to 0.5-1.0 range
            profile.push(Decimal::try_from(weight).unwrap());
        }
        
        profile
    }
}

impl Default for VWAPExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Iceberg Executor - Hide large orders
pub struct IcebergExecutor {
    pub displayed_size: Decimal, // Visible portion
}

impl IcebergExecutor {
    pub fn new(displayed_size: Decimal) -> Self {
        Self { displayed_size }
    }
    
    /// Create child orders from iceberg parent
    pub fn slice_order(&self, order: &Order) -> Vec<Order> {
        let total = order.quantity;
        let mut slices = Vec::new();
        let mut remaining = total;
        
        while remaining > Decimal::ZERO {
            let slice_qty = remaining.min(self.displayed_size);
            
            let mut slice = Order::market(&order.symbol, order.side, slice_qty);
            slice.venue = order.venue.clone();
            slice.metadata.parent_order_id = Some(order.id);
            
            slices.push(slice);
            remaining -= slice_qty;
        }
        
        slices
    }
    
    /// Execute iceberg order
    pub async fn execute<F, Fut>(
        &self,
        order: &Order,
        execute_child: F,
    ) -> Result<Vec<Fill>>
    where
        F: Fn(Order) -> Fut,
        Fut: std::future::Future<Output = Result<Fill>>,
    {
        let slices = self.slice_order(order);
        info!("Iceberg order sliced into {} child orders", slices.len());
        
        let mut fills = Vec::new();
        
        for (i, slice) in slices.iter().enumerate() {
            // Only reveal next slice after previous fills
            match execute_child(slice.clone()).await {
                Ok(fill) => {
                    info!("Iceberg slice {}/{} filled", i + 1, slices.len());
                    fills.push(fill);
                }
                Err(e) => {
                    warn!("Iceberg slice failed: {}", e);
                    // For iceberg, we might want to retry or adjust
                }
            }
            
            // Small delay between slices to avoid detection
            sleep(Duration::from_millis(100)).await;
        }
        
        Ok(fills)
    }
}

/// Algorithm selector based on order characteristics
pub struct AlgorithmSelector;

impl AlgorithmSelector {
    /// Select best algorithm for order
    pub fn select(order: &Order) -> &'static str {
        match &order.order_type {
            OrderType::TWAP { .. } => "TWAP",
            OrderType::VWAP { .. } => "VWAP",
            OrderType::Iceberg { .. } => "Iceberg",
            _ => {
                // Auto-select based on size
                if order.quantity > Decimal::from(100) {
                    "TWAP" // Large order, use TWAP
                } else {
                    "Market" // Small order, execute immediately
                }
            }
        }
    }
    
    /// Recommend algorithm with reasoning
    pub fn recommend(order: &Order) -> (String, String) {
        let qty = order.quantity;
        
        if qty > Decimal::from(1000) {
            ("VWAP".to_string(), 
             "Large size - use volume-weighted execution to minimize impact".to_string())
        } else if qty > Decimal::from(100) {
            ("TWAP".to_string(),
             "Medium size - spread over time for price improvement".to_string())
        } else if qty > Decimal::from(10) {
            ("Iceberg".to_string(),
             "Moderate size - hide from market".to_string())
        } else {
            ("Market".to_string(),
             "Small size - immediate execution".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::execution::order::OrderSide;
    
    #[tokio::test]
    async fn test_twap_execution() {
        let executor = TWAPExecutor::new();
        let order = Order::twap("BTC", OrderSide::Buy, Decimal::from(100), 1, 5);
        
        let fills = executor.execute(&order, |qty| {
            async move {
                Ok(Fill {
                    id: uuid::Uuid::new_v4(),
                    order_id: order.id,
                    symbol: "BTC".to_string(),
                    side: OrderSide::Buy,
                    quantity: qty,
                    price: Decimal::from(50000),
                    venue: super::super::venue::Venue::Binance,
                    timestamp: Utc::now(),
                    fees: Decimal::from(50),
                })
            }
        }).await.unwrap();
        
        assert_eq!(fills.len(), 5);
    }
    
    #[test]
    fn test_twap_slice_generation() {
        let executor = TWAPExecutor::new();
        let order = Order::twap("BTC", OrderSide::Buy, Decimal::from(100), 60, 4);
        
        let slices = executor.generate_slices(&order).unwrap();
        
        assert_eq!(slices.len(), 4);
        assert_eq!(slices[0].quantity, Decimal::from(25)); // 100/4
        assert_eq!(slices[0].total_slices, 4);
    }
    
    #[test]
    fn test_iceberg_slicing() {
        let executor = IcebergExecutor::new(Decimal::from(10));
        let order = Order::market("BTC", OrderSide::Buy, Decimal::from(45));
        
        let slices = executor.slice_order(&order);
        
        assert_eq!(slices.len(), 5); // 45/10 = 4.5 → 5 slices
        assert_eq!(slices[0].quantity, Decimal::from(10));
        assert_eq!(slices[4].quantity, Decimal::from(5)); // Remainder
    }
    
    #[test]
    fn test_vwap_profile_generation() {
        let profile = VWAPExecutor::default_intraday_profile(24);
        
        assert_eq!(profile.len(), 24);
        
        // First bucket (open) should be high
        let first = profile.first().unwrap();
        let middle = profile.get(12).unwrap(); // Midday
        let last = profile.last().unwrap(); // Close
        
        assert!(first > middle);
        assert!(last > middle);
    }
    
    #[test]
    fn test_algorithm_selection() {
        let twap_order = Order::twap("BTC", OrderSide::Buy, Decimal::from(10), 60, 5);
        assert_eq!(AlgorithmSelector::select(&twap_order), "TWAP");
        
        let market_order = Order::market("BTC", OrderSide::Buy, Decimal::from(5));
        assert_eq!(AlgorithmSelector::select(&market_order), "Market");
        
        let large_order = Order::market("BTC", OrderSide::Buy, Decimal::from(200));
        assert_eq!(AlgorithmSelector::select(&large_order), "TWAP");
    }
    
    #[test]
    fn test_algorithm_recommendation() {
        let small = Order::market("BTC", OrderSide::Buy, Decimal::from(5));
        let (algo, reason) = AlgorithmSelector::recommend(&small);
        assert_eq!(algo, "Market");
        assert!(reason.contains("Small"));
        
        let large = Order::market("BTC", OrderSide::Buy, Decimal::from(2000));
        let (algo, reason) = AlgorithmSelector::recommend(&large);
        assert_eq!(algo, "VWAP");
        assert!(reason.contains("Large"));
    }
}
