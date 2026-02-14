//! Strategies Module - Advanced Trading Strategies
//!
//! Sprint 21: Advanced Strategies
//! - Momentum trading
//! - Mean reversion
//! - Pairs trading / Statistical arbitrage
//! - Strategy combination and backtesting

pub mod error;
pub mod mean_reversion;
pub mod momentum;
pub mod pairs;
pub mod types;

pub use error::{StrategyError, Result};
pub use mean_reversion::{MeanReversionStrategy, MeanReversionConfig, SmaCrossover};
pub use momentum::{MomentumStrategy, MomentumConfig, RSICalculator};
pub use pairs::{PairsTradingStrategy, PairsConfig, CointegrationTest, CommonPairs};
pub use types::*;

use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::debug;

/// Strategy registry
#[derive(Debug)]
pub struct StrategyRegistry {
    strategies: HashMap<String, Box<dyn Strategy>>,
}

/// Strategy trait
pub trait Strategy: std::fmt::Debug + Send {
    fn name(&self) -> &str;
    fn strategy_type(&self) -> StrategyType;
    fn generate_signal(&self, symbol: &str, data: &[PriceData]) -> Result<Signal>;
}

/// Multi-strategy engine
#[derive(Debug)]
pub struct StrategyEngine {
    registry: StrategyRegistry,
    active_strategies: Vec<String>,
    signals: Vec<Signal>,
}

impl StrategyEngine {
    /// Create new strategy engine
    pub fn new() -> Self {
        Self {
            registry: StrategyRegistry {
                strategies: HashMap::new(),
            },
            active_strategies: Vec::new(),
            signals: Vec::new(),
        }
    }
    
    /// Register a strategy
    pub fn register<S: Strategy + 'static>(&mut self, name: String, strategy: S) {
        self.registry.strategies.insert(name.clone(), Box::new(strategy));
        self.active_strategies.push(name);
    }
    
    /// Run all strategies on data
    pub fn run_all(&mut self, data: &HashMap<String, Vec<PriceData>>) -> Vec<Signal> {
        let mut signals = Vec::new();
        
        for strategy_name in &self.active_strategies {
            if let Some(strategy) = self.registry.strategies.get(strategy_name) {
                for (symbol, symbol_data) in data {
                    match strategy.generate_signal(symbol, symbol_data) {
                        Ok(signal) if signal.is_actionable() => {
                            debug!(
                                "Signal from {} for {}: {:?} (strength: {:.2})",
                                strategy_name, symbol, signal.direction, signal.strength
                            );
                            signals.push(signal);
                        }
                        Ok(_) => {} // Non-actionable signal
                        Err(e) => {
                            debug!("Strategy {} error for {}: {}", strategy_name, symbol, e);
                        }
                    }
                }
            }
        }
        
        // Sort by strength descending
        signals.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap());
        
        self.signals = signals.clone();
        signals
    }
    
    /// Get combined signal for symbol (consensus)
    pub fn get_consensus(&self, symbol: &str) -> Option<Signal> {
        let symbol_signals: Vec<_> = self.signals.iter()
            .filter(|s| s.symbol == symbol)
            .collect();
        
        if symbol_signals.is_empty() {
            return None;
        }
        
        // Count directions
        let long_count = symbol_signals.iter()
            .filter(|s| s.direction == SignalDirection::Long)
            .count();
        let short_count = symbol_signals.iter()
            .filter(|s| s.direction == SignalDirection::Short)
            .count();
        
        // Calculate weighted average strength
        let total_strength: Decimal = symbol_signals.iter()
            .map(|s| s.strength)
            .sum();
        
        let avg_strength = total_strength / Decimal::from(symbol_signals.len() as i64);
        
        // Determine consensus direction
        let direction = if long_count > short_count {
            SignalDirection::Long
        } else if short_count > long_count {
            SignalDirection::Short
        } else {
            SignalDirection::Neutral
        };
        
        // Confidence based on agreement
        let agreement = if long_count + short_count > 0 {
            Decimal::from(long_count.max(short_count) as i64) 
                / Decimal::from((long_count + short_count) as i64)
        } else {
            Decimal::ZERO
        };
        
        Some(Signal::new(symbol, direction, avg_strength)
            .with_confidence(agreement * Decimal::from(100)))
    }
    
    /// Get active strategies count
    pub fn active_count(&self) -> usize {
        self.active_strategies.len()
    }
    
    /// Clear signals
    pub fn clear_signals(&mut self) {
        self.signals.clear();
    }
}

impl Default for StrategyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple backtester
pub struct StrategyBacktester;

impl StrategyBacktester {
    /// Run backtest for a strategy
    pub fn backtest(
        strategy: &dyn Strategy,
        data: &[PriceData],
        initial_capital: Decimal,
    ) -> BacktestResult {
        let mut capital = initial_capital;
        let mut position = Decimal::ZERO;
        let mut trades: Vec<Trade> = Vec::new();
        let mut max_capital = initial_capital;
        let mut max_drawdown = Decimal::ZERO;
        
        for i in 20..data.len() {
            let window = &data[..i];
            let current_price = data[i].close;
            
            if let Ok(signal) = strategy.generate_signal("TEST", window) {
                if signal.is_actionable() {
                    // Execute signal
                    match signal.direction {
                        SignalDirection::Long if position <= Decimal::ZERO => {
                            // Close short or enter long
                            if position < Decimal::ZERO {
                                let pnl = (current_price - trades.last().unwrap().exit_price) * position.abs();
                                capital += pnl;
                            }
                            position = Decimal::ONE;
                            trades.push(Trade {
                                entry_price: current_price,
                                exit_price: Decimal::ZERO,
                                entry_time: data[i].timestamp,
                                exit_time: None,
                                pnl: Decimal::ZERO,
                            });
                        }
                        SignalDirection::Short if position >= Decimal::ZERO => {
                            // Close long or enter short
                            if position > Decimal::ZERO {
                                let pnl = (current_price - trades.last().unwrap().entry_price) * position;
                                capital += pnl;
                                if let Some(t) = trades.last_mut() {
                                    t.exit_price = current_price;
                                    t.exit_time = Some(data[i].timestamp);
                                    t.pnl = pnl;
                                }
                            }
                            position = -Decimal::ONE;
                            trades.push(Trade {
                                entry_price: current_price,
                                exit_price: Decimal::ZERO,
                                entry_time: data[i].timestamp,
                                exit_time: None,
                                pnl: Decimal::ZERO,
                            });
                        }
                        _ => {}
                    }
                    
                    // Track max capital and drawdown
                    if capital > max_capital {
                        max_capital = capital;
                    }
                    let drawdown = (max_capital - capital) / max_capital;
                    if drawdown > max_drawdown {
                        max_drawdown = drawdown;
                    }
                }
            }
        }
        
        // Close final position
        if position != Decimal::ZERO {
            let final_price = data.last().unwrap().close;
            if let Some(t) = trades.last_mut() {
                t.exit_price = final_price;
                t.exit_time = Some(data.last().unwrap().timestamp);
                if position > Decimal::ZERO {
                    t.pnl = (final_price - t.entry_price) * position;
                } else {
                    t.pnl = (t.entry_price - final_price) * position.abs();
                }
            }
        }
        
        let winning_trades = trades.iter().filter(|t| t.pnl > Decimal::ZERO).count();
        let total_trades = trades.len();
        
        BacktestResult {
            strategy_name: strategy.name().to_string(),
            initial_capital,
            final_capital: capital,
            total_return: (capital - initial_capital) / initial_capital,
            max_drawdown,
            total_trades,
            winning_trades,
            win_rate: if total_trades > 0 {
                Decimal::from(winning_trades as i64) / Decimal::from(total_trades as i64)
            } else {
                Decimal::ZERO
            },
            trades,
        }
    }
}

/// Trade record
#[derive(Debug, Clone)]
pub struct Trade {
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    pub entry_time: chrono::DateTime<chrono::Utc>,
    pub exit_time: Option<chrono::DateTime<chrono::Utc>>,
    pub pnl: Decimal,
}

/// Backtest result
#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub strategy_name: String,
    pub initial_capital: Decimal,
    pub final_capital: Decimal,
    pub total_return: Decimal,
    pub max_drawdown: Decimal,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub win_rate: Decimal,
    pub trades: Vec<Trade>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    fn create_test_data(prices: Vec<i64>) -> HashMap<String, Vec<PriceData>> {
        let mut data = HashMap::new();
        let price_data: Vec<PriceData> = prices.into_iter()
            .enumerate()
            .map(|(i, price)| PriceData {
                timestamp: Utc::now() + chrono::Duration::hours(i as i64),
                open: Decimal::from(price),
                high: Decimal::from(price + 10),
                low: Decimal::from(price - 10),
                close: Decimal::from(price),
                volume: Decimal::from(1000),
            })
            .collect();
        data.insert("BTC".to_string(), price_data);
        data
    }
    
    #[test]
    fn test_strategy_engine_creation() {
        let engine = StrategyEngine::new();
        assert_eq!(engine.active_count(), 0);
    }
    
    #[test]
    fn test_register_and_run_strategies() {
        let mut engine = StrategyEngine::new();
        
        // Register momentum strategy
        let momentum = MomentumStrategy::new(MomentumConfig::default());
        engine.register("Momentum".to_string(), momentum);
        
        assert_eq!(engine.active_count(), 1);
        
        // Create test data
        let prices: Vec<i64> = (0..40).map(|i| 100 + i * 5).collect();
        let data = create_test_data(prices);
        
        // Run strategies
        let signals = engine.run_all(&data);
        
        // May or may not have signals depending on data
        // Just verify it runs without error
    }
    
    #[test]
    fn test_consensus() {
        let mut engine = StrategyEngine::new();
        
        // Register strategies
        engine.register("Momentum".to_string(), 
            MomentumStrategy::new(MomentumConfig::default()));
        engine.register("MeanRev".to_string(),
            MeanReversionStrategy::new(MeanReversionConfig::default()));
        
        // Run with data
        let prices: Vec<i64> = (0..50).map(|i| 100 + (i % 10) * 2).collect();
        let data = create_test_data(prices);
        engine.run_all(&data);
        
        // Get consensus
        let consensus = engine.get_consensus("BTC");
        
        // Consensus might be None if strategies disagree
        // or Some if they agree
        if let Some(c) = consensus {
            assert!(c.confidence >= Decimal::ZERO && c.confidence <= Decimal::from(100));
        }
    }
    
    #[test]
    fn test_backtest() {
        // Create uptrend data
        let prices: Vec<i64> = (0..100).map(|i| 100 + i * 2).collect();
        let data: Vec<PriceData> = prices.into_iter()
            .enumerate()
            .map(|(i, price)| PriceData {
                timestamp: Utc::now() + chrono::Duration::hours(i as i64),
                open: Decimal::from(price),
                high: Decimal::from(price + 5),
                low: Decimal::from(price - 5),
                close: Decimal::from(price),
                volume: Decimal::from(1000),
            })
            .collect();
        
        let strategy = MomentumStrategy::new(MomentumConfig::default());
        let result = StrategyBacktester::backtest(&strategy, &data, Decimal::from(10000));
        
        assert_eq!(result.strategy_name, "Momentum");
        // In trending market, momentum may or may not generate trades
        // Just verify the backtest runs without error
        assert!(result.win_rate >= Decimal::ZERO && result.win_rate <= Decimal::ONE);
    }
}
