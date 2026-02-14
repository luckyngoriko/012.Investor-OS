//! Risk management for market making

use rust_decimal::Decimal;
use chrono::{DateTime, Utc, Duration};

use super::quoting::TwoSidedQuote;
use super::error::{MarketMakingError, Result};

/// Risk limits for market making
#[derive(Debug, Clone)]
pub struct RiskLimits {
    pub max_position_usd: Decimal,
    pub max_inventory_ratio: Decimal,    // Max inventory as % of total
    pub max_daily_loss: Decimal,
    pub max_drawdown_pct: Decimal,
    pub min_spread_bps: Decimal,
    pub max_orders_per_minute: u32,
    pub pause_after_loss_streak: u32,    // Consecutive losses before pause
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_usd: Decimal::from(1000000),  // $1M
            max_inventory_ratio: Decimal::try_from(0.5).unwrap(), // 50%
            max_daily_loss: Decimal::from(50000),      // $50k
            max_drawdown_pct: Decimal::try_from(0.05).unwrap(), // 5%
            min_spread_bps: Decimal::from(5),
            max_orders_per_minute: 600,
            pause_after_loss_streak: 5,
        }
    }
}

/// Trading pause state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradingPause {
    Active,           // Normal trading
    Paused(u64),      // Paused for N seconds
    EmergencyStop,    // Manual stop required
}

/// Risk manager for market making
#[derive(Debug)]
pub struct MarketMakingRiskManager {
    limits: RiskLimits,
    daily_pnl: Decimal,
    peak_pnl: Decimal,
    current_drawdown: Decimal,
    loss_streak: u32,
    orders_this_minute: u32,
    last_order_time: DateTime<Utc>,
    pause_state: TradingPause,
    pause_until: Option<DateTime<Utc>>,
    trade_history: Vec<(DateTime<Utc>, Decimal)>, // (time, pnl)
}

impl MarketMakingRiskManager {
    pub fn new(limits: RiskLimits) -> Self {
        Self {
            limits,
            daily_pnl: Decimal::ZERO,
            peak_pnl: Decimal::ZERO,
            current_drawdown: Decimal::ZERO,
            loss_streak: 0,
            orders_this_minute: 0,
            last_order_time: Utc::now(),
            pause_state: TradingPause::Active,
            pause_until: None,
            trade_history: Vec::new(),
        }
    }
    
    /// Check if trading is allowed
    pub fn can_trade(&mut self) -> bool {
        // Check pause state
        match self.pause_state {
            TradingPause::Active => {}
            TradingPause::Paused(_) => {
                if let Some(until) = self.pause_until {
                    if Utc::now() < until {
                        return false;
                    }
                    // Resume trading
                    self.pause_state = TradingPause::Active;
                    self.loss_streak = 0;
                }
            }
            TradingPause::EmergencyStop => return false,
        }
        
        // Check daily loss limit
        if self.daily_pnl < -self.limits.max_daily_loss {
            self.pause(300); // Pause 5 minutes
            return false;
        }
        
        // Check drawdown
        if self.current_drawdown > self.limits.max_drawdown_pct {
            self.pause(600); // Pause 10 minutes
            return false;
        }
        
        // Check order rate
        self.check_order_rate();
        if self.orders_this_minute >= self.limits.max_orders_per_minute {
            return false;
        }
        
        true
    }
    
    /// Validate a quote
    pub fn validate_quote(&self, quote: &TwoSidedQuote) -> Result<()> {
        // Check spread
        if quote.spread_bps < self.limits.min_spread_bps {
            return Err(MarketMakingError::SpreadTooTight {
                current: quote.spread_bps,
                minimum: self.limits.min_spread_bps,
            });
        }
        
        // Check if bid < ask
        if let (Some(bid), Some(ask)) = (&quote.bid, &quote.ask) {
            if bid.price >= ask.price {
                return Err(MarketMakingError::QuoteRejected(
                    "Bid price must be below ask price".to_string()
                ));
            }
        }
        
        Ok(())
    }
    
    /// Record trade for risk tracking
    pub fn record_trade(&mut self, notional: Decimal) {
        let now = Utc::now();
        
        // Update order rate
        self.check_order_rate();
        self.orders_this_minute += 1;
        self.last_order_time = now;
        
        self.trade_history.push((now, notional));
        
        // Clean old history (> 1 hour)
        self.trade_history.retain(|(t, _)| now - *t < Duration::hours(1));
    }
    
    /// Record P&L
    pub fn record_pnl(&mut self, pnl: Decimal) {
        self.daily_pnl += pnl;
        
        // Track peak and drawdown
        if self.daily_pnl > self.peak_pnl {
            self.peak_pnl = self.daily_pnl;
        }
        
        if self.peak_pnl > Decimal::ZERO {
            self.current_drawdown = (self.peak_pnl - self.daily_pnl) / self.peak_pnl;
        }
        
        // Track loss streak
        if pnl < Decimal::ZERO {
            self.loss_streak += 1;
            
            if self.loss_streak >= self.limits.pause_after_loss_streak {
                self.pause(60); // Pause 1 minute after loss streak
            }
        } else {
            self.loss_streak = 0;
        }
    }
    
    /// Pause trading
    pub fn pause(&mut self, seconds: u64) {
        self.pause_state = TradingPause::Paused(seconds);
        self.pause_until = Some(Utc::now() + Duration::seconds(seconds as i64));
    }
    
    /// Emergency stop
    pub fn emergency_stop(&mut self) {
        self.pause_state = TradingPause::EmergencyStop;
    }
    
    /// Resume trading (only from pause, not emergency)
    pub fn resume(&mut self) -> Result<()> {
        match self.pause_state {
            TradingPause::EmergencyStop => {
                Err(MarketMakingError::QuoteRejected(
                    "Cannot resume from emergency stop without manual reset".to_string()
                ))
            }
            _ => {
                self.pause_state = TradingPause::Active;
                self.pause_until = None;
                self.loss_streak = 0;
                Ok(())
            }
        }
    }
    
    /// Check and update order rate
    fn check_order_rate(&mut self) {
        let now = Utc::now();
        if now - self.last_order_time >= Duration::minutes(1) {
            self.orders_this_minute = 0;
        }
    }
    
    /// Get current risk metrics
    pub fn metrics(&self) -> RiskMetrics {
        RiskMetrics {
            daily_pnl: self.daily_pnl,
            peak_pnl: self.peak_pnl,
            current_drawdown: self.current_drawdown,
            loss_streak: self.loss_streak,
            orders_per_minute: self.orders_this_minute,
            pause_state: self.pause_state,
            trade_count: self.trade_history.len(),
        }
    }
    
    /// Reset daily stats (call at midnight)
    pub fn reset_daily(&mut self) {
        self.daily_pnl = Decimal::ZERO;
        self.peak_pnl = Decimal::ZERO;
        self.current_drawdown = Decimal::ZERO;
        self.loss_streak = 0;
    }
}

/// Risk metrics snapshot
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub daily_pnl: Decimal,
    pub peak_pnl: Decimal,
    pub current_drawdown: Decimal,
    pub loss_streak: u32,
    pub orders_per_minute: u32,
    pub pause_state: TradingPause,
    pub trade_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::order::OrderSide;
    use crate::execution::venue::Venue;
    use crate::market_making::quoting::Quote;
    
    fn create_test_quote(spread_bps: Decimal) -> TwoSidedQuote {
        let mid = Decimal::from(50000);
        let half_spread = (mid * spread_bps) / Decimal::from(20000);
        
        TwoSidedQuote {
            symbol: "BTC".to_string(),
            bid: Some(Quote {
                symbol: "BTC".to_string(),
                side: OrderSide::Buy,
                price: mid - half_spread,
                quantity: Decimal::ONE,
                venue: Venue::Binance,
            }),
            ask: Some(Quote {
                symbol: "BTC".to_string(),
                side: OrderSide::Sell,
                price: mid + half_spread,
                quantity: Decimal::ONE,
                venue: Venue::Binance,
            }),
            spread_bps,
            mid_price: mid,
            timestamp: Utc::now(),
        }
    }
    
    #[test]
    fn test_validate_quote_spread() {
        let limits = RiskLimits {
            min_spread_bps: Decimal::from(10),
            ..Default::default()
        };
        let manager = MarketMakingRiskManager::new(limits);
        
        // Too tight
        let tight_quote = create_test_quote(Decimal::from(5));
        assert!(manager.validate_quote(&tight_quote).is_err());
        
        // OK
        let good_quote = create_test_quote(Decimal::from(20));
        assert!(manager.validate_quote(&good_quote).is_ok());
    }
    
    #[test]
    fn test_daily_loss_limit() {
        let limits = RiskLimits {
            max_daily_loss: Decimal::from(1000),
            ..Default::default()
        };
        let mut manager = MarketMakingRiskManager::new(limits);
        
        // Initially can trade
        assert!(manager.can_trade());
        
        // Hit loss limit
        manager.record_pnl(Decimal::from(-1500));
        
        // Should be paused
        assert!(!manager.can_trade());
        assert!(matches!(manager.pause_state, TradingPause::Paused(_)));
    }
    
    #[test]
    fn test_loss_streak_pause() {
        let limits = RiskLimits {
            pause_after_loss_streak: 3,
            ..Default::default()
        };
        let mut manager = MarketMakingRiskManager::new(limits);
        
        // 3 losses in a row
        manager.record_pnl(Decimal::from(-100));
        manager.record_pnl(Decimal::from(-100));
        manager.record_pnl(Decimal::from(-100));
        
        // Should be paused
        assert!(!manager.can_trade());
    }
    
    #[test]
    fn test_emergency_stop() {
        let limits = RiskLimits::default();
        let mut manager = MarketMakingRiskManager::new(limits);
        
        manager.emergency_stop();
        
        assert!(!manager.can_trade());
        assert!(matches!(manager.pause_state, TradingPause::EmergencyStop));
        
        // Cannot resume from emergency
        assert!(manager.resume().is_err());
    }
    
    #[test]
    fn test_drawdown_tracking() {
        let limits = RiskLimits::default();
        let mut manager = MarketMakingRiskManager::new(limits);
        
        // Profits first
        manager.record_pnl(Decimal::from(1000));
        manager.record_pnl(Decimal::from(500));
        
        assert_eq!(manager.peak_pnl, Decimal::from(1500));
        
        // Drawdown
        manager.record_pnl(Decimal::from(-300));
        
        // Drawdown = (1500 - 1200) / 1500 = 20%
        assert!(manager.current_drawdown > Decimal::ZERO);
    }
    
    #[test]
    fn test_order_rate_limit() {
        let limits = RiskLimits {
            max_orders_per_minute: 3,
            ..Default::default()
        };
        let mut manager = MarketMakingRiskManager::new(limits);
        
        // Initially can trade
        assert!(manager.can_trade());
        
        // Record 2 trades (below limit)
        manager.record_trade(Decimal::from(1000));
        manager.record_trade(Decimal::from(1000));
        
        // Can still trade
        assert!(manager.can_trade());
        
        // Third trade reaches limit
        manager.record_trade(Decimal::from(1000));
        
        // At limit, cannot trade more
        assert!(!manager.can_trade());
    }
    
    #[test]
    fn test_metrics() {
        let limits = RiskLimits::default();
        let mut manager = MarketMakingRiskManager::new(limits);
        
        manager.record_pnl(Decimal::from(500));
        manager.record_trade(Decimal::from(10000));
        
        let metrics = manager.metrics();
        
        assert_eq!(metrics.daily_pnl, Decimal::from(500));
        assert_eq!(metrics.trade_count, 1);
        assert!(matches!(metrics.pause_state, TradingPause::Active));
    }
}
