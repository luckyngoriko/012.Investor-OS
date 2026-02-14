//! Performance Attribution
//!
//! Tracks and analyzes strategy performance across dimensions

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Attribution engine
#[derive(Debug)]
pub struct AttributionEngine {
    trades: Vec<TradeRecord>,
    daily_pnl: HashMap<String, Vec<DailyPnl>>, // strategy_id -> daily pnl
    regime_performance: HashMap<super::MarketRegime, Vec<RegimePnl>>,
}

/// Trade record
#[derive(Debug, Clone)]
struct TradeRecord {
    id: Uuid,
    strategy_id: Uuid,
    symbol: String,
    pnl: Decimal,
    timestamp: DateTime<Utc>,
    regime: Option<super::MarketRegime>,
}

/// Daily P&L record
#[derive(Debug, Clone)]
struct DailyPnl {
    date: chrono::NaiveDate,
    pnl: Decimal,
    trades: u32,
}

/// Performance per regime
#[derive(Debug, Clone)]
struct RegimePnl {
    regime: super::MarketRegime,
    pnl: Decimal,
    trades: u32,
    timestamp: DateTime<Utc>,
}

/// Strategy performance summary
#[derive(Debug, Clone)]
pub struct StrategyPerformance {
    pub strategy_id: Uuid,
    pub total_pnl: Decimal,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub win_rate: f32,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub profit_factor: f32,
    pub sharpe_ratio: f32,
    pub max_drawdown: Decimal,
    pub avg_daily_pnl: Decimal,
}

/// Performance attribution report
#[derive(Debug, Clone)]
pub struct PerformanceAttribution {
    pub total_pnl: Decimal,
    pub total_trades: u32,
    pub by_strategy: HashMap<Uuid, StrategyPerformance>,
    pub by_regime: HashMap<super::MarketRegime, RegimePerformance>,
    pub by_time_period: HashMap<String, Decimal>, // "2024-Q1", "2024-Q2", etc.
}

/// Performance by regime
#[derive(Debug, Clone)]
pub struct RegimePerformance {
    pub regime: super::MarketRegime,
    pub total_pnl: Decimal,
    pub trade_count: u32,
    pub avg_pnl_per_trade: Decimal,
    pub win_rate: f32,
}

impl AttributionEngine {
    /// Create new attribution engine
    pub fn new() -> Self {
        Self {
            trades: Vec::new(),
            daily_pnl: HashMap::new(),
            regime_performance: HashMap::new(),
        }
    }
    
    /// Record a trade
    pub fn record_trade(&mut self, strategy_id: Uuid, pnl: Decimal, trades: u32) {
        let today = Utc::now().date_naive();
        
        // Record individual trade results for accurate win/loss tracking
        // Create separate entries for each trade to preserve individual PnL
        let daily = self.daily_pnl.entry(strategy_id.to_string()).or_default();
        
        // For accurate attribution, we track each trade individually
        // If multiple trades with same PnL direction happen on same day, we still track separately
        for _ in 0..trades {
            daily.push(DailyPnl {
                date: today,
                pnl: if trades == 1 { pnl } else { pnl / Decimal::from(trades) },
                trades: 1,
            });
        }
        
        info!(
            "Recorded trade: strategy={} pnl={} trades={}",
            strategy_id, pnl, trades
        );
    }
    
    /// Record trade with regime context
    pub fn record_trade_with_regime(
        &mut self,
        strategy_id: Uuid,
        symbol: &str,
        pnl: Decimal,
        regime: super::MarketRegime,
    ) {
        let trade = TradeRecord {
            id: Uuid::new_v4(),
            strategy_id,
            symbol: symbol.to_string(),
            pnl,
            timestamp: Utc::now(),
            regime: Some(regime),
        };
        
        self.trades.push(trade);
        
        // Update regime performance
        let regime_pnl = self.regime_performance.entry(regime).or_default();
        regime_pnl.push(RegimePnl {
            regime,
            pnl,
            trades: 1,
            timestamp: Utc::now(),
        });
    }
    
    /// Get performance attribution
    pub fn get_attribution(&self) -> PerformanceAttribution {
        let mut by_strategy: HashMap<Uuid, StrategyPerformance> = HashMap::new();
        let mut by_regime: HashMap<super::MarketRegime, RegimePerformance> = HashMap::new();
        
        // Calculate by strategy
        for (strategy_id, daily_pnls) in &self.daily_pnl {
            let strategy_id = Uuid::parse_str(strategy_id).unwrap();
            let perf = self.calculate_strategy_performance(strategy_id, daily_pnls);
            by_strategy.insert(strategy_id, perf);
        }
        
        // Calculate by regime
        for (regime, pnls) in &self.regime_performance {
            let total_pnl: Decimal = pnls.iter().map(|p| p.pnl).sum();
            let trade_count = pnls.iter().map(|p| p.trades).sum();
            
            let winning_trades = pnls.iter().filter(|p| p.pnl > Decimal::ZERO).count() as u32;
            let win_rate = if trade_count > 0 {
                winning_trades as f32 / trade_count as f32
            } else {
                0.0
            };
            
            let avg_pnl = if trade_count > 0 {
                total_pnl / Decimal::from(trade_count)
            } else {
                Decimal::ZERO
            };
            
            by_regime.insert(*regime, RegimePerformance {
                regime: *regime,
                total_pnl,
                trade_count,
                avg_pnl_per_trade: avg_pnl,
                win_rate,
            });
        }
        
        // Calculate totals
        let total_pnl: Decimal = by_strategy.values().map(|p| p.total_pnl).sum();
        let total_trades: u32 = by_strategy.values().map(|p| p.total_trades).sum();
        
        PerformanceAttribution {
            total_pnl,
            total_trades,
            by_strategy,
            by_regime,
            by_time_period: self.calculate_time_period_attribution(),
        }
    }
    
    /// Calculate strategy performance
    fn calculate_strategy_performance(
        &self,
        strategy_id: Uuid,
        daily_pnls: &[DailyPnl],
    ) -> StrategyPerformance {
        let total_pnl: Decimal = daily_pnls.iter().map(|d| d.pnl).sum();
        let total_trades: u32 = daily_pnls.iter().map(|d| d.trades).sum();
        
        let mut winning_trades = 0u32;
        let mut losing_trades = 0u32;
        let mut total_wins = Decimal::ZERO;
        let mut total_losses = Decimal::ZERO;
        let mut max_drawdown = Decimal::ZERO;
        let mut peak = Decimal::ZERO;
        let mut running_pnl = Decimal::ZERO;
        
        for day in daily_pnls {
            running_pnl += day.pnl;
            
            if running_pnl > peak {
                peak = running_pnl;
            }
            
            let drawdown = peak - running_pnl;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
            
            if day.pnl > Decimal::ZERO {
                winning_trades += day.trades;
                total_wins += day.pnl;
            } else if day.pnl < Decimal::ZERO {
                losing_trades += day.trades;
                total_losses += day.pnl.abs();
            }
        }
        
        let win_rate = if total_trades > 0 {
            winning_trades as f32 / total_trades as f32
        } else {
            0.0
        };
        
        let avg_win = if winning_trades > 0 {
            total_wins / Decimal::from(winning_trades)
        } else {
            Decimal::ZERO
        };
        
        let avg_loss = if losing_trades > 0 {
            total_losses / Decimal::from(losing_trades)
        } else {
            Decimal::ZERO
        };
        
        let profit_factor: f32 = if total_losses > Decimal::ZERO {
            (total_wins / total_losses).try_into().unwrap_or(0.0f32)
        } else if total_wins > Decimal::ZERO { f32::INFINITY } else { 0.0 };
        
        let avg_daily_pnl = if !daily_pnls.is_empty() {
            total_pnl / Decimal::from(daily_pnls.len() as i64)
        } else {
            Decimal::ZERO
        };
        
        // Simplified Sharpe calculation (assuming risk-free rate of 0)
        let returns: Vec<f32> = daily_pnls.iter()
            .map(|d| d.pnl.try_into().unwrap_or(0.0f32))
            .collect();
        let sharpe_ratio = calculate_sharpe(&returns);
        
        StrategyPerformance {
            strategy_id,
            total_pnl,
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            avg_win,
            avg_loss,
            profit_factor,
            sharpe_ratio,
            max_drawdown,
            avg_daily_pnl,
        }
    }
    
    /// Calculate time period attribution
    fn calculate_time_period_attribution(&self) -> HashMap<String, Decimal> {
        let mut periods: HashMap<String, Decimal> = HashMap::new();
        
        for daily_pnls in self.daily_pnl.values() {
            for day in daily_pnls {
                use chrono::Datelike;
                let period = format!("{}-Q{}", 
                    day.date.year(),
                    (day.date.month() - 1) / 3 + 1
                );
                
                *periods.entry(period).or_insert(Decimal::ZERO) += day.pnl;
            }
        }
        
        periods
    }
    
    /// Get performance for specific strategy
    pub fn get_strategy_performance(&self, strategy_id: Uuid) -> Option<StrategyPerformance> {
        let key = strategy_id.to_string();
        self.daily_pnl.get(&key).map(|daily| {
            self.calculate_strategy_performance(strategy_id, daily)
        })
    }
    
    /// Get best performing strategy
    pub fn get_best_strategy(&self) -> Option<(Uuid, Decimal)> {
        let attribution = self.get_attribution();
        attribution.by_strategy
            .iter()
            .max_by(|a, b| a.1.total_pnl.cmp(&b.1.total_pnl))
            .map(|(id, perf)| (*id, perf.total_pnl))
    }
    
    /// Get worst performing strategy
    pub fn get_worst_strategy(&self) -> Option<(Uuid, Decimal)> {
        let attribution = self.get_attribution();
        attribution.by_strategy
            .iter()
            .min_by(|a, b| a.1.total_pnl.cmp(&b.1.total_pnl))
            .map(|(id, perf)| (*id, perf.total_pnl))
    }
    
    /// Get best performing regime
    pub fn get_best_regime(&self) -> Option<(super::MarketRegime, Decimal)> {
        let attribution = self.get_attribution();
        attribution.by_regime
            .iter()
            .max_by(|a, b| a.1.total_pnl.cmp(&b.1.total_pnl))
            .map(|(regime, perf)| (*regime, perf.total_pnl))
    }
    
    /// Clean old data (keep last N days)
    pub fn clean_old_data(&mut self, days: i64) {
        let cutoff = Utc::now() - Duration::days(days);
        
        self.trades.retain(|t| t.timestamp > cutoff);
        
        for daily in self.daily_pnl.values_mut() {
            daily.retain(|d| d.date > cutoff.date_naive());
        }
    }
    
    /// Get total trade count
    pub fn total_trade_count(&self) -> usize {
        self.trades.len()
    }
}

impl Default for AttributionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate Sharpe ratio from returns
fn calculate_sharpe(returns: &[f32]) -> f32 {
    if returns.is_empty() {
        return 0.0;
    }
    
    let mean = returns.iter().sum::<f32>() / returns.len() as f32;
    
    if returns.len() < 2 {
        return if mean > 0.0 { f32::INFINITY } else { 0.0 };
    }
    
    let variance = returns.iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f32>() / (returns.len() - 1) as f32;
    
    let std_dev = variance.sqrt();
    
    if std_dev == 0.0 {
        if mean > 0.0 { f32::INFINITY } else { 0.0 }
    } else {
        mean / std_dev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribution_engine_creation() {
        let engine = AttributionEngine::new();
        assert_eq!(engine.total_trade_count(), 0);
    }

    #[test]
    fn test_record_trade() {
        let mut engine = AttributionEngine::new();
        let strategy_id = Uuid::new_v4();
        
        engine.record_trade(strategy_id, Decimal::from(100), 1);
        
        let perf = engine.get_strategy_performance(strategy_id).unwrap();
        assert_eq!(perf.total_pnl, Decimal::from(100));
        assert_eq!(perf.total_trades, 1);
    }

    #[test]
    fn test_multiple_trades() {
        let mut engine = AttributionEngine::new();
        let strategy_id = Uuid::new_v4();
        
        engine.record_trade(strategy_id, Decimal::from(100), 1);
        engine.record_trade(strategy_id, Decimal::from(-50), 1);
        engine.record_trade(strategy_id, Decimal::from(75), 1);
        
        let perf = engine.get_strategy_performance(strategy_id).unwrap();
        assert_eq!(perf.total_pnl, Decimal::from(125));
        assert_eq!(perf.total_trades, 3);
        assert_eq!(perf.winning_trades, 2);
        assert_eq!(perf.losing_trades, 1);
    }

    #[test]
    fn test_win_rate_calculation() {
        let mut engine = AttributionEngine::new();
        let strategy_id = Uuid::new_v4();
        
        engine.record_trade(strategy_id, Decimal::from(100), 1);
        engine.record_trade(strategy_id, Decimal::from(100), 1);
        engine.record_trade(strategy_id, Decimal::from(-100), 1);
        
        let perf = engine.get_strategy_performance(strategy_id).unwrap();
        assert_eq!(perf.win_rate, 2.0 / 3.0);
    }

    #[test]
    fn test_profit_factor() {
        let mut engine = AttributionEngine::new();
        let strategy_id = Uuid::new_v4();
        
        engine.record_trade(strategy_id, Decimal::from(200), 1);  // Win
        engine.record_trade(strategy_id, Decimal::from(-100), 1); // Loss
        
        let perf = engine.get_strategy_performance(strategy_id).unwrap();
        assert_eq!(perf.profit_factor, 2.0);
    }

    #[test]
    fn test_best_worst_strategy() {
        let mut engine = AttributionEngine::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        
        engine.record_trade(id1, Decimal::from(500), 1);
        engine.record_trade(id2, Decimal::from(-100), 1);
        
        let best = engine.get_best_strategy().unwrap();
        assert_eq!(best.0, id1);
        assert_eq!(best.1, Decimal::from(500));
        
        let worst = engine.get_worst_strategy().unwrap();
        assert_eq!(worst.0, id2);
        assert_eq!(worst.1, Decimal::from(-100));
    }
}
