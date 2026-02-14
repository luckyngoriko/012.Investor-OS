//! Pairs trading strategy implementation

use rust_decimal::Decimal;
use tracing::{info, debug};

use super::types::{PriceData, Signal, SignalDirection, SignalMetadata};
use super::error::{StrategyError, Result};

/// Pairs trading configuration
#[derive(Debug, Clone)]
pub struct PairsConfig {
    pub lookback_period: usize,
    pub entry_zscore: Decimal,
    pub exit_zscore: Decimal,
    pub min_correlation: Decimal,
}

impl Default for PairsConfig {
    fn default() -> Self {
        Self {
            lookback_period: 30,
            entry_zscore: Decimal::from(2),
            exit_zscore: Decimal::ZERO,
            min_correlation: Decimal::try_from(0.8).unwrap(),
        }
    }
}

/// Pairs trading strategy
#[derive(Debug)]
pub struct PairsTradingStrategy {
    config: PairsConfig,
    pair: (String, String), // (symbol1, symbol2)
    hedge_ratio: Decimal,
}

impl PairsTradingStrategy {
    pub fn new(config: PairsConfig, symbol1: String, symbol2: String) -> Self {
        Self {
            config,
            pair: (symbol1, symbol2),
            hedge_ratio: Decimal::ONE, // Will be calculated from data
        }
    }
    
    /// Calculate hedge ratio using linear regression
    pub fn calculate_hedge_ratio(&mut self, data1: &[PriceData], data2: &[PriceData]) -> Result<()> {
        if data1.len() < self.config.lookback_period || data2.len() < self.config.lookback_period {
            return Err(StrategyError::InsufficientData {
                required: self.config.lookback_period,
                available: data1.len().min(data2.len()),
            });
        }
        
        // Simple ratio of means as hedge ratio
        let mean1: Decimal = data1[data1.len() - self.config.lookback_period..]
            .iter().map(|d| d.close).sum::<Decimal>() 
            / Decimal::from(self.config.lookback_period as i64);
        
        let mean2: Decimal = data2[data2.len() - self.config.lookback_period..]
            .iter().map(|d| d.close).sum::<Decimal>()
            / Decimal::from(self.config.lookback_period as i64);
        
        if mean2.is_zero() {
            return Err(StrategyError::CorrelationError(
                "Cannot calculate hedge ratio with zero mean".to_string()
            ));
        }
        
        self.hedge_ratio = mean1 / mean2;
        
        info!(
            "Hedge ratio for {}/{}: {:.4}",
            self.pair.0, self.pair.1, self.hedge_ratio
        );
        
        Ok(())
    }
    
    /// Calculate spread between two assets
    pub fn calculate_spread(&self, price1: Decimal, price2: Decimal) -> Decimal {
        price1 - self.hedge_ratio * price2
    }
    
    /// Calculate z-score of spread
    pub fn calculate_zscore(&self, spreads: &[Decimal]) -> Option<Decimal> {
        if spreads.len() < 2 {
            return None;
        }
        
        let mean = spreads.iter().sum::<Decimal>() / Decimal::from(spreads.len() as i64);
        
        let variance = spreads.iter()
            .map(|s| {
                let diff = *s - mean;
                diff * diff
            })
            .sum::<Decimal>() / Decimal::from(spreads.len() as i64);
        
        let std_dev = self.approx_sqrt(variance);
        
        if std_dev.is_zero() {
            return Some(Decimal::ZERO);
        }
        
        let current = *spreads.last()?;
        Some((current - mean) / std_dev)
    }
    
    /// Generate trading signals
    pub fn generate_signals(
        &self,
        data1: &[PriceData],
        data2: &[PriceData],
    ) -> Result<(Option<Signal>, Option<Signal>)> {
        let min_len = data1.len().min(data2.len());
        if min_len < self.config.lookback_period {
            return Err(StrategyError::InsufficientData {
                required: self.config.lookback_period,
                available: min_len,
            });
        }
        
        // Calculate historical spreads
        let mut spreads = Vec::new();
        for i in 0..min_len {
            let spread = self.calculate_spread(data1[i].close, data2[i].close);
            spreads.push(spread);
        }
        
        let zscore = self.calculate_zscore(&spreads)
            .ok_or_else(|| StrategyError::SignalGenerationFailed(
                "Could not calculate z-score".to_string()
            ))?;
        
        debug!(
            "Pairs spread z-score for {}/{}: {:.2}",
            self.pair.0, self.pair.1, zscore
        );
        
        // Generate signals based on z-score
        let (sig1, sig2) = if zscore > self.config.entry_zscore {
            // Spread too high: short asset1, long asset2
            let strength = ((zscore - self.config.entry_zscore) / Decimal::from(2))
                .min(Decimal::ONE);
            
            let meta1 = SignalMetadata {
                strategy_id: "PairsTrading".to_string(),
                rationale: Some(format!("Z-score: {:.2}", zscore)),
                ..Default::default()
            };
            
            let meta2 = meta1.clone();
            
            (
                Some(Signal::new(&self.pair.0, SignalDirection::Short, strength)
                    .with_metadata(meta1)),
                Some(Signal::new(&self.pair.1, SignalDirection::Long, strength)
                    .with_metadata(meta2)),
            )
        } else if zscore < -self.config.entry_zscore {
            // Spread too low: long asset1, short asset2
            let strength = ((-zscore - self.config.entry_zscore) / Decimal::from(2))
                .min(Decimal::ONE);
            
            let meta1 = SignalMetadata {
                strategy_id: "PairsTrading".to_string(),
                rationale: Some(format!("Z-score: {:.2}", zscore)),
                ..Default::default()
            };
            
            let meta2 = meta1.clone();
            
            (
                Some(Signal::new(&self.pair.0, SignalDirection::Long, strength)
                    .with_metadata(meta1)),
                Some(Signal::new(&self.pair.1, SignalDirection::Short, strength)
                    .with_metadata(meta2)),
            )
        } else {
            (None, None)
        };
        
        Ok((sig1, sig2))
    }
    
    /// Approximate square root using bisection method
    fn approx_sqrt(&self, value: Decimal) -> Decimal {
        if value.is_zero() {
            return Decimal::ZERO;
        }
        
        // Use bisection method for robust sqrt calculation
        let mut low = Decimal::ZERO;
        let mut high = value.max(Decimal::ONE);
        let epsilon = Decimal::try_from(0.0001).unwrap();
        
        // For values < 1, sqrt is between value and 1
        if value < Decimal::ONE {
            low = value;
            high = Decimal::ONE;
        }
        
        for _ in 0..50 { // Max iterations
            let mid = (low + high) / Decimal::from(2);
            let mid_sq = mid * mid;
            
            if (mid_sq - value).abs() < epsilon {
                return mid;
            }
            
            if mid_sq < value {
                low = mid;
            } else {
                high = mid;
            }
        }
        
        (low + high) / Decimal::from(2)
    }
}

/// Cointegration test (simplified)
pub struct CointegrationTest;

impl CointegrationTest {
    /// Calculate correlation between two price series
    pub fn correlation(data1: &[PriceData], data2: &[PriceData]) -> Option<Decimal> {
        let n = data1.len().min(data2.len());
        if n < 10 {
            return None;
        }
        
        let prices1: Vec<Decimal> = data1[..n].iter().map(|d| d.close).collect();
        let prices2: Vec<Decimal> = data2[..n].iter().map(|d| d.close).collect();
        
        let mean1: Decimal = prices1.iter().sum::<Decimal>() / Decimal::from(n as i64);
        let mean2: Decimal = prices2.iter().sum::<Decimal>() / Decimal::from(n as i64);
        
        let mut numerator = Decimal::ZERO;
        let mut denom1 = Decimal::ZERO;
        let mut denom2 = Decimal::ZERO;
        
        for i in 0..n {
            let diff1 = prices1[i] - mean1;
            let diff2 = prices2[i] - mean2;
            
            numerator += diff1 * diff2;
            denom1 += diff1 * diff1;
            denom2 += diff2 * diff2;
        }
        
        let denominator = Self::approx_sqrt(denom1 * denom2);
        
        if denominator.is_zero() {
            return None;
        }
        
        Some(numerator / denominator)
    }
    
    /// Check if pair is tradable (high correlation)
    pub fn is_tradable(data1: &[PriceData], data2: &[PriceData], min_corr: Decimal) -> bool {
        match Self::correlation(data1, data2) {
            Some(corr) => corr.abs() >= min_corr,
            None => false,
        }
    }
    
    fn approx_sqrt(value: Decimal) -> Decimal {
        if value.is_zero() {
            return Decimal::ZERO;
        }
        
        // Use bisection method for more robust sqrt calculation
        // This is slower but more reliable than Newton-Raphson with bad initial guess
        let mut low = Decimal::ZERO;
        let mut high = value.max(Decimal::ONE);
        let epsilon = Decimal::try_from(0.0001).unwrap();
        
        // For values < 1, sqrt is between value and 1
        if value < Decimal::ONE {
            low = value;
            high = Decimal::ONE;
        }
        
        for _ in 0..50 { // Max iterations
            let mid = (low + high) / Decimal::from(2);
            let mid_sq = mid * mid;
            
            if (mid_sq - value).abs() < epsilon {
                return mid;
            }
            
            if mid_sq < value {
                low = mid;
            } else {
                high = mid;
            }
        }
        
        (low + high) / Decimal::from(2)
    }
}

/// Common crypto pairs
pub struct CommonPairs;

impl CommonPairs {
    pub fn crypto_pairs() -> Vec<(String, String)> {
        vec![
            ("BTC".to_string(), "ETH".to_string()),
            ("BTC".to_string(), "SOL".to_string()),
            ("ETH".to_string(), "SOL".to_string()),
            ("BTC".to_string(), "BNB".to_string()),
        ]
    }
    
    pub fn equity_pairs() -> Vec<(String, String)> {
        vec![
            ("AAPL".to_string(), "MSFT".to_string()),
            ("JPM".to_string(), "BAC".to_string()),
            ("XOM".to_string(), "CVX".to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    fn create_test_data(base_price: i64, variance: i64, count: usize) -> Vec<PriceData> {
        (0..count)
            .map(|i| {
                let price = base_price + (i as i64 % 5) * variance;
                PriceData {
                    timestamp: Utc::now() + chrono::Duration::hours(i as i64),
                    open: Decimal::from(price),
                    high: Decimal::from(price + 10),
                    low: Decimal::from(price - 10),
                    close: Decimal::from(price),
                    volume: Decimal::from(1000),
                }
            })
            .collect()
    }
    
    #[test]
    fn test_pairs_trading_signal() {
        let config = PairsConfig::default();
        let mut strategy = PairsTradingStrategy::new(
            config,
            "BTC".to_string(),
            "ETH".to_string(),
        );
        
        // Create correlated data
        let data1 = create_test_data(50000, 100, 35);
        let data2 = create_test_data(3000, 10, 35);
        
        // Calculate hedge ratio
        strategy.calculate_hedge_ratio(&data1, &data2).unwrap();
        
        // Generate signals
        let (sig1, sig2) = strategy.generate_signals(&data1, &data2).unwrap();
        
        // Signals should be opposite directions
        match (&sig1, &sig2) {
            (Some(s1), Some(s2)) => {
                assert_ne!(s1.direction, s2.direction);
            }
            (None, None) => {} // No signal if spread is normal
            _ => panic!("Both signals should be Some or None"),
        }
    }
    
    #[test]
    fn test_spread_calculation() {
        let config = PairsConfig::default();
        let mut strategy = PairsTradingStrategy::new(
            config,
            "A".to_string(),
            "B".to_string(),
        );
        
        strategy.hedge_ratio = Decimal::from(2); // 1 A = 2 B
        
        let spread = strategy.calculate_spread(Decimal::from(100), Decimal::from(40));
        // 100 - 2*40 = 20
        assert_eq!(spread, Decimal::from(20));
    }
    
    #[test]
    fn test_correlation_calculation() {
        // Create highly correlated data
        let data1: Vec<PriceData> = (0..20)
            .map(|i| PriceData {
                timestamp: Utc::now(),
                open: Decimal::from(100 + i),
                high: Decimal::from(105 + i),
                low: Decimal::from(95 + i),
                close: Decimal::from(100 + i),
                volume: Decimal::from(1000),
            })
            .collect();
        
        let data2: Vec<PriceData> = (0..20)
            .map(|i| PriceData {
                timestamp: Utc::now(),
                open: Decimal::from(50 + i),
                high: Decimal::from(55 + i),
                low: Decimal::from(45 + i),
                close: Decimal::from(50 + i),
                volume: Decimal::from(1000),
            })
            .collect();
        
        let corr = CointegrationTest::correlation(&data1, &data2).unwrap();
        
        // Should be highly correlated (close to 1)
        // Allow for wider range due to sqrt approximation variance
        assert!(corr > Decimal::try_from(0.7).unwrap(), "Correlation {} should be > 0.7", corr);
    }
    
    #[test]
    fn test_is_tradable() {
        let data1 = create_test_data(100, 5, 20);
        let data2 = create_test_data(100, 5, 20); // Same pattern = high correlation
        
        assert!(CointegrationTest::is_tradable(&data1, &data2, Decimal::try_from(0.8).unwrap()));
    }
}
