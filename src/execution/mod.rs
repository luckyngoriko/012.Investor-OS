//! Execution Module - Smart Order Routing & Algorithmic Trading
//!
//! Sprint 18: Smart Order Routing (SOR)
//! - Venue analysis and comparison
//! - Cost calculation with market impact
//! - Smart routing to optimal venues
//! - TWAP/VWAP/Iceberg algorithms
//! - Multi-venue execution

pub mod algorithms;
pub mod cost;
pub mod error;
pub mod order;
pub mod router;
pub mod venue;

pub use algorithms::{TWAPExecutor, VWAPExecutor, IcebergExecutor, AlgorithmSelector};
pub use cost::{CostCalculator, ExecutionCost, ImpactModel};
pub use error::{ExecutionError, Result};
pub use order::{Order, OrderSide, OrderType, OrderStatus, TimeInForce, Fill, OrderSlice};
pub use router::{SmartRouter, RouteDecision};
pub use venue::{Venue, VenueType, VenueQuote, VenueAnalyzer, VenueScore, FeeStructure};

use rust_decimal::Decimal;
use uuid::Uuid;
use tracing::info;

/// Main execution engine
#[derive(Debug)]
pub struct ExecutionEngine {
    router: SmartRouter,
    twap_executor: TWAPExecutor,
    vwap_executor: VWAPExecutor,
}

impl ExecutionEngine {
    /// Create new execution engine
    pub fn new() -> Self {
        Self {
            router: SmartRouter::new(),
            twap_executor: TWAPExecutor::new(),
            vwap_executor: VWAPExecutor::new(),
        }
    }
    
    /// Submit order for execution
    pub async fn submit_order(&self, order: &Order) -> Result<Vec<Fill>> {
        info!("Submitting order: {:?} {} {} {:?}", 
            order.side, order.quantity, order.symbol, order.order_type);
        
        match &order.order_type {
            OrderType::Market | OrderType::Limit(_) | OrderType::Stop(_) => {
                // Single execution
                let decision = self.router.route(order)?;
                self.execute_single(order, &decision).await
            }
            OrderType::TWAP { .. } => {
                // TWAP execution
                self.twap_executor.execute(order, |qty| async move {
                    // Mock execution - in real implementation would call venue API
                    Ok(Fill {
                        id: Uuid::new_v4(),
                        order_id: order.id,
                        symbol: order.symbol.clone(),
                        side: order.side,
                        quantity: qty,
                        price: Decimal::from(50000), // Mock price
                        venue: Venue::Binance,
                        timestamp: chrono::Utc::now(),
                        fees: qty * Decimal::from(50000) * Decimal::try_from(0.001).unwrap(),
                    })
                }).await
            }
            OrderType::VWAP { .. } => {
                // VWAP execution with default profile
                let profile = VWAPExecutor::default_intraday_profile(24);
                self.vwap_executor.execute(order, &profile, |qty| async move {
                    Ok(Fill {
                        id: Uuid::new_v4(),
                        order_id: order.id,
                        symbol: order.symbol.clone(),
                        side: order.side,
                        quantity: qty,
                        price: Decimal::from(50000),
                        venue: Venue::Binance,
                        timestamp: chrono::Utc::now(),
                        fees: qty * Decimal::from(50000) * Decimal::try_from(0.001).unwrap(),
                    })
                }).await
            }
            OrderType::Iceberg { .. } => {
                // Iceberg execution
                let iceberg = IcebergExecutor::new(order.quantity / Decimal::from(10));
                iceberg.execute(order, |child| async move {
                    Ok(Fill {
                        id: Uuid::new_v4(),
                        order_id: child.id,
                        symbol: child.symbol.clone(),
                        side: child.side,
                        quantity: child.quantity,
                        price: Decimal::from(50000),
                        venue: Venue::Binance,
                        timestamp: chrono::Utc::now(),
                        fees: child.quantity * Decimal::from(50000) * Decimal::try_from(0.001).unwrap(),
                    })
                }).await
            }
            OrderType::StopLimit(_, _) => {
                Err(ExecutionError::ExecutionFailed(
                    "Stop-limit orders not yet implemented".to_string()
                ))
            }
        }
    }
    
    /// Execute single order at chosen venue
    async fn execute_single(&self, order: &Order, decision: &RouteDecision) -> Result<Vec<Fill>> {
        info!("Executing at {}: expected cost ${}", 
            decision.primary_venue.name(), decision.expected_cost);
        
        // Mock execution - in production this would call the venue's API
        let fill = Fill {
            id: Uuid::new_v4(),
            order_id: order.id,
            symbol: order.symbol.clone(),
            side: order.side,
            quantity: order.quantity,
            price: Decimal::from(50000), // Mock
            venue: decision.primary_venue.clone(),
            timestamp: chrono::Utc::now(),
            fees: order.quantity * Decimal::from(50000) * Decimal::try_from(0.001).unwrap(),
        };
        
        Ok(vec![fill])
    }
    
    /// Get best quote for symbol
    pub fn get_best_quote(&self, symbol: &str, side: OrderSide) -> Option<&VenueQuote> {
        self.router.venue_analyzer().get_best_quote(symbol, side)
    }
    
    /// Update venue quote
    pub fn update_quote(&mut self, quote: VenueQuote) {
        self.router.venue_analyzer_mut().update_quote(quote);
    }
    
    /// Calculate execution cost estimate
    pub fn estimate_cost(&self, order: &Order, _venue: &Venue) -> Option<ExecutionCost> {
        // Find quote for venue
        let quote = self.router.venue_analyzer()
            .get_best_quote(&order.symbol, order.side)?;
        
        // Use default ADV for estimation
        let adv = Decimal::from(1000000);
        let cost_calc = CostCalculator::new();
        
        Some(cost_calc.calculate_cost(order, quote, adv))
    }
    
    /// Recommend best algorithm for order
    pub fn recommend_algorithm(&self, order: &Order) -> (String, String) {
        AlgorithmSelector::recommend(order)
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_execution_engine_creation() {
        let engine = ExecutionEngine::new();
        // Just verify it compiles and runs
        assert!(true);
    }
    
    #[tokio::test]
    async fn test_submit_market_order() {
        let engine = ExecutionEngine::new();
        let order = Order::market("BTC", OrderSide::Buy, Decimal::from(1))
            .with_venue(venue::Venue::Binance);
        
        let fills = engine.submit_order(&order).await.unwrap();
        
        assert!(!fills.is_empty());
        assert_eq!(fills[0].quantity, Decimal::from(1));
    }
    
    #[tokio::test]
    async fn test_submit_twap_order() {
        let engine = ExecutionEngine::new();
        let order = Order::twap("BTC", OrderSide::Buy, Decimal::from(10), 1, 3);
        
        let fills = engine.submit_order(&order).await.unwrap();
        
        // Should have multiple fills (one per slice)
        assert_eq!(fills.len(), 3);
    }
    
    #[tokio::test]
    async fn test_full_execution_lifecycle() {
        println!("\n🚀 Testing Full Execution Lifecycle");
        
        let mut engine = ExecutionEngine::new();
        
        // 1. Update market quotes
        engine.update_quote(VenueQuote {
            venue: Venue::Binance,
            symbol: "BTC".to_string(),
            bid: Decimal::from(49900),
            ask: Decimal::from(50100),
            bid_size: Decimal::from(100),
            ask_size: Decimal::from(100),
            timestamp: chrono::Utc::now(),
            latency_ms: 20,
        });
        
        engine.update_quote(VenueQuote {
            venue: Venue::Coinbase,
            symbol: "BTC".to_string(),
            bid: Decimal::from(49950),
            ask: Decimal::from(50050),
            bid_size: Decimal::from(80),
            ask_size: Decimal::from(80),
            timestamp: chrono::Utc::now(),
            latency_ms: 25,
        });
        
        println!("✅ Updated venue quotes");
        
        // 2. Get best quote
        let best = engine.get_best_quote("BTC", OrderSide::Buy);
        assert!(best.is_some());
        println!("📊 Best ask: {} @ ${}", best.unwrap().venue.name(), best.unwrap().ask);
        
        // 3. Get algorithm recommendation
        let large_order = Order::market("BTC", OrderSide::Buy, Decimal::from(500));
        let (algo, reason) = engine.recommend_algorithm(&large_order);
        println!("🤖 Recommended algorithm: {} ({})", algo, reason);
        
        // 4. Estimate cost
        let cost = engine.estimate_cost(&large_order, &Venue::Binance);
        if let Some(c) = cost {
            println!("💰 Estimated cost: ${} ({} bps)", c.total_cost, c.total_cost_bps);
        }
        
        // 5. Execute with TWAP
        let twap_order = Order::twap("BTC", OrderSide::Buy, Decimal::from(10), 1, 3);
        let fills = engine.submit_order(&twap_order).await.unwrap();
        
        let total_filled: Decimal = fills.iter().map(|f| f.quantity).sum();
        let total_fees: Decimal = fills.iter().map(|f| f.fees).sum();
        
        println!("📈 TWAP execution completed:");
        println!("   Slices filled: {}", fills.len());
        println!("   Total quantity: {}", total_filled);
        println!("   Total fees: ${}", total_fees);
        
        assert_eq!(total_filled, Decimal::from(10));
        
        println!("\n✅ Full execution lifecycle completed!");
    }
}
