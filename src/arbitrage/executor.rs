//! Arbitrage execution engine

use rust_decimal::Decimal;
use tracing::{info, error};
use std::time::Instant;

use crate::execution::{ExecutionEngine, Order, OrderSide, Fill};
use crate::execution::venue::Venue;

use super::opportunity::{ArbitrageOpportunity, OpportunityTracker};
use super::error::{ArbitrageError, Result};

/// Arbitrage execution configuration
#[derive(Debug, Clone)]
pub struct ArbExecutorConfig {
    pub max_position_hold_ms: u64,
    pub max_slippage_bps: Decimal,
    pub simultaneous_arbs: usize,
    pub min_profit_bps: Decimal,
}

impl Default for ArbExecutorConfig {
    fn default() -> Self {
        Self {
            max_position_hold_ms: 500,     // 500ms max exposure
            max_slippage_bps: Decimal::from(5), // 5 bps slippage tolerance
            simultaneous_arbs: 3,          // Max 3 concurrent
            min_profit_bps: Decimal::from(3),   // 3 bps minimum
        }
    }
}

/// Arbitrage execution engine
#[derive(Debug)]
pub struct ArbitrageExecutor {
    config: ArbExecutorConfig,
    execution_engine: ExecutionEngine,
    tracker: OpportunityTracker,
    active_executions: usize,
}

impl ArbitrageExecutor {
    pub fn new(config: ArbExecutorConfig) -> Self {
        Self {
            config,
            execution_engine: ExecutionEngine::new(),
            tracker: OpportunityTracker::default(),
            active_executions: 0,
        }
    }
    
    /// Execute arbitrage opportunity
    pub async fn execute(&mut self, opp: &ArbitrageOpportunity) -> Result<ArbitrageResult> {
        // Pre-execution checks
        if self.active_executions >= self.config.simultaneous_arbs {
            return Err(ArbitrageError::RiskLimitExceeded(
                "Too many simultaneous executions".to_string()
            ));
        }
        
        if !opp.is_executable() {
            self.tracker.record_missed(opp.clone());
            return Err(ArbitrageError::OpportunityExpired {
                expected: opp.net_profit,
                actual: Decimal::ZERO,
            });
        }
        
        self.active_executions += 1;
        let start_time = Instant::now();
        
        info!("🎯 Executing arbitrage: Buy {} @ {}, Sell {} @ {}",
            opp.buy_venue.name(), opp.buy_price,
            opp.sell_venue.name(), opp.sell_price
        );
        
        // Execute both legs simultaneously
        let buy_order = Order::market(&opp.symbol, OrderSide::Buy, opp.quantity)
            .with_venue(opp.buy_venue.clone());
        
        let sell_order = Order::market(&opp.symbol, OrderSide::Sell, opp.quantity)
            .with_venue(opp.sell_venue.clone());
        
        // In production: these should be sent simultaneously
        // For now, execute sequentially
        let buy_result = self.execution_engine.submit_order(&buy_order).await;
        let sell_result = self.execution_engine.submit_order(&sell_order).await;
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        self.active_executions -= 1;
        
        match (buy_result, sell_result) {
            (Ok(buy_fills), Ok(sell_fills)) => {
                self.process_successful_execution(opp, buy_fills, sell_fills, execution_time_ms).await
            }
            (Err(e), _) | (_, Err(e)) => {
                error!("❌ Arbitrage execution failed: {}", e);
                Err(ArbitrageError::ExecutionFailed(e.to_string()))
            }
        }
    }
    
    /// Process successful execution
    async fn process_successful_execution(
        &mut self,
        opp: &ArbitrageOpportunity,
        buy_fills: Vec<Fill>,
        sell_fills: Vec<Fill>,
        execution_time_ms: u64,
    ) -> Result<ArbitrageResult> {
        let buy_price = self.calculate_avg_price(&buy_fills);
        let sell_price = self.calculate_avg_price(&sell_fills);
        let quantity = self.calculate_total_quantity(&buy_fills);
        
        let buy_cost = buy_price * quantity;
        let sell_revenue = sell_price * quantity;
        let gross_profit = sell_revenue - buy_cost;
        
        let total_fees: Decimal = buy_fills.iter().chain(sell_fills.iter())
            .map(|f| f.fees)
            .sum();
        
        let net_profit = gross_profit - total_fees;
        let profit_bps = (net_profit / buy_cost) * Decimal::from(10000);
        
        let result = ArbitrageResult {
            opportunity_id: opp.id,
            symbol: opp.symbol.clone(),
            quantity,
            buy_price,
            sell_price,
            buy_venue: opp.buy_venue.clone(),
            sell_venue: opp.sell_venue.clone(),
            gross_profit,
            fees: total_fees,
            net_profit,
            profit_bps,
            execution_time_ms,
            completed_at: chrono::Utc::now(),
        };
        
        self.tracker.record_executed(opp.clone(), net_profit);
        
        info!("✅ Arbitrage completed: Profit = ${} ({} bps) in {}ms",
            net_profit, profit_bps, execution_time_ms);
        
        Ok(result)
    }
    
    /// Calculate average price from fills
    fn calculate_avg_price(&self, fills: &[Fill]) -> Decimal {
        let total_notional: Decimal = fills.iter().map(|f| f.notional()).sum();
        let total_qty: Decimal = fills.iter().map(|f| f.quantity).sum();
        
        if total_qty.is_zero() {
            return Decimal::ZERO;
        }
        
        total_notional / total_qty
    }
    
    /// Calculate total quantity from fills
    fn calculate_total_quantity(&self, fills: &[Fill]) -> Decimal {
        fills.iter().map(|f| f.quantity).sum()
    }
    
    /// Check if we should execute this opportunity
    pub fn should_execute(&self, opp: &ArbitrageOpportunity) -> bool {
        opp.profit_bps >= self.config.min_profit_bps
            && opp.latency_ms <= self.config.max_position_hold_ms
            && opp.is_executable()
            && self.active_executions < self.config.simultaneous_arbs
    }
    
    /// Get execution statistics
    pub fn stats(&self) -> ArbExecutionStats {
        let tracker_stats = self.tracker.stats();
        
        ArbExecutionStats {
            total_opportunities: tracker_stats.total_detected,
            executed: tracker_stats.total_executed,
            missed: tracker_stats.total_missed,
            total_profit: tracker_stats.total_profit,
            avg_profit_per_trade: tracker_stats.avg_profit_per_trade,
            active_executions: self.active_executions,
        }
    }
}

/// Result of arbitrage execution
#[derive(Debug, Clone)]
pub struct ArbitrageResult {
    pub opportunity_id: uuid::Uuid,
    pub symbol: String,
    pub quantity: Decimal,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub buy_venue: Venue,
    pub sell_venue: Venue,
    pub gross_profit: Decimal,
    pub fees: Decimal,
    pub net_profit: Decimal,
    pub profit_bps: Decimal,
    pub execution_time_ms: u64,
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ArbExecutionStats {
    pub total_opportunities: usize,
    pub executed: usize,
    pub missed: usize,
    pub total_profit: Decimal,
    pub avg_profit_per_trade: Decimal,
    pub active_executions: usize,
}

/// Risk-managed arbitrage execution
#[derive(Debug)]
pub struct RiskManagedArbitrage {
    executor: ArbitrageExecutor,
    daily_profit_limit: Decimal,
    daily_loss_limit: Decimal,
    daily_pnl: Decimal,
    last_reset: chrono::DateTime<chrono::Utc>,
}

impl RiskManagedArbitrage {
    pub fn new(executor_config: ArbExecutorConfig) -> Self {
        Self {
            executor: ArbitrageExecutor::new(executor_config),
            daily_profit_limit: Decimal::from(10000), // $10k daily target
            daily_loss_limit: Decimal::from(-2000),   // $2k daily stop
            daily_pnl: Decimal::ZERO,
            last_reset: chrono::Utc::now(),
        }
    }
    
    /// Check if trading is allowed
    pub fn can_trade(&mut self) -> bool {
        self.check_reset();
        
        self.daily_pnl > self.daily_loss_limit 
            && self.daily_pnl < self.daily_profit_limit
    }
    
    /// Execute with risk checks
    pub async fn execute(&mut self, opp: &ArbitrageOpportunity) -> Result<ArbitrageResult> {
        if !self.can_trade() {
            return Err(ArbitrageError::RiskLimitExceeded(
                "Daily P&L limit reached".to_string()
            ));
        }
        
        let result = self.executor.execute(opp).await;
        
        if let Ok(ref r) = result {
            self.daily_pnl += r.net_profit;
        }
        
        result
    }
    
    fn check_reset(&mut self) {
        let now = chrono::Utc::now();
        if now.date_naive() != self.last_reset.date_naive() {
            self.daily_pnl = Decimal::ZERO;
            self.last_reset = now;
        }
    }
    
    pub fn stats(&self) -> ArbExecutionStats {
        self.executor.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    #[tokio::test]
    async fn test_arbitrage_executor_creation() {
        let config = ArbExecutorConfig::default();
        let executor = ArbitrageExecutor::new(config);
        
        assert_eq!(executor.active_executions, 0);
    }
    
    #[test]
    fn test_calculate_avg_price() {
        let config = ArbExecutorConfig::default();
        let executor = ArbitrageExecutor::new(config);
        
        let fills = vec![
            Fill {
                id: uuid::Uuid::new_v4(),
                order_id: uuid::Uuid::new_v4(),
                symbol: "BTC".to_string(),
                side: OrderSide::Buy,
                quantity: Decimal::from(1),
                price: Decimal::from(50000),
                venue: Venue::Binance,
                timestamp: Utc::now(),
                fees: Decimal::from(50),
            },
            Fill {
                id: uuid::Uuid::new_v4(),
                order_id: uuid::Uuid::new_v4(),
                symbol: "BTC".to_string(),
                side: OrderSide::Buy,
                quantity: Decimal::from(1),
                price: Decimal::from(50100),
                venue: Venue::Binance,
                timestamp: Utc::now(),
                fees: Decimal::from(50),
            },
        ];
        
        let avg = executor.calculate_avg_price(&fills);
        // (50000 + 50100) / 2 = 50050
        assert_eq!(avg, Decimal::from(50050));
    }
    
    #[test]
    fn test_should_execute() {
        let config = ArbExecutorConfig {
            min_profit_bps: Decimal::from(5),
            max_position_hold_ms: 500,
            ..Default::default()
        };
        let executor = ArbitrageExecutor::new(config);
        
        let good_opp = ArbitrageOpportunity {
            id: uuid::Uuid::new_v4(),
            arb_type: super::super::opportunity::ArbitrageType::CrossVenue,
            symbol: "BTC".to_string(),
            buy_venue: Venue::Binance,
            sell_venue: Venue::Coinbase,
            buy_price: Decimal::from(50000),
            sell_price: Decimal::from(50100),
            quantity: Decimal::ONE,
            gross_profit: Decimal::from(100),
            estimated_costs: Decimal::from(20),
            net_profit: Decimal::from(80),
            profit_bps: Decimal::from(10), // Above threshold
            confidence: Decimal::ONE,
            detected_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::seconds(5),
            latency_ms: 100, // Fast enough
        };
        
        assert!(executor.should_execute(&good_opp));
        
        let bad_opp = ArbitrageOpportunity {
            profit_bps: Decimal::from(2), // Below threshold
            ..good_opp.clone()
        };
        
        assert!(!executor.should_execute(&bad_opp));
    }
    
    #[tokio::test]
    async fn test_risk_managed_trading() {
        let config = ArbExecutorConfig::default();
        let mut risk_arb = RiskManagedArbitrage::new(config);
        
        assert!(risk_arb.can_trade());
        
        // Simulate hitting loss limit
        risk_arb.daily_pnl = Decimal::from(-2500);
        assert!(!risk_arb.can_trade());
    }
}
