//! Mean reversion strategy implementation

use rust_decimal::Decimal;
use std::collections::VecDeque;
use tracing::debug;

use super::types::{PriceData, Signal, SignalDirection, SignalMetadata};

/// Mean reversion configuration
#[derive(Debug, Clone)]
pub struct MeanReversionConfig {
    pub lookback_period: usize,
    pub entry_threshold: Decimal, // Number of standard deviations
    pub exit_threshold: Decimal,
    pub use_bollinger_bands: bool,
    pub bb_std_dev: Decimal,
}

impl Default for MeanReversionConfig {
    fn default() -> Self {
        Self {
            lookback_period: 20,
            entry_threshold: Decimal::from(2),  // 2 std dev
            exit_threshold: Decimal::ONE,        // 1 std dev
            use_bollinger_bands: true,
            bb_std_dev: Decimal::from(2),
        }
    }
}

/// Mean reversion strategy
#[derive(Debug)]
pub struct MeanReversionStrategy {
    config: MeanReversionConfig,
    price_history: VecDeque<Decimal>,
}

impl MeanReversionStrategy {
    pub fn new(config: MeanReversionConfig) -> Self {
        let capacity = config.lookback_period + 10;
        Self {
            config,
            price_history: VecDeque::with_capacity(capacity),
        }
    }
    
    /// Update with new price
    pub fn update_price(&mut self, price: Decimal) {
        self.price_history.push_back(price);
        if self.price_history.len() > self.config.lookback_period {
            self.price_history.pop_front();
        }
    }
    
    /// Generate signal
    pub fn generate_signal(&self, symbol: &str) -> Option<Signal> {
        if self.price_history.len() < self.config.lookback_period {
            return None;
        }
        
        let (mean, std_dev) = self.calculate_stats()?;
        let current = *self.price_history.back()?;
        
        let z_score = if std_dev.is_zero() {
            Decimal::ZERO
        } else {
            (current - mean) / std_dev
        };
        
        debug!(
            "Mean Reversion for {}: price={}, mean={:.2}, z-score={:.2}",
            symbol, current, mean, z_score
        );
        
        // Generate signal based on z-score
        let (direction, strength) = if z_score > self.config.entry_threshold {
            // Price too high, expect reversion down
            let strength = (z_score - self.config.entry_threshold) 
                / Decimal::from(2); // Normalize
            (SignalDirection::Short, strength.min(Decimal::ONE))
        } else if z_score < -self.config.entry_threshold {
            // Price too low, expect reversion up
            let strength = (-z_score - self.config.entry_threshold) 
                / Decimal::from(2);
            (SignalDirection::Long, strength.min(Decimal::ONE))
        } else {
            return None;
        };
        
        let metadata = SignalMetadata {
            strategy_id: "MeanReversion".to_string(),
            entry_price: Some(current),
            rationale: Some(format!(
                "Z-score: {:.2}, Mean: {:.2}, StdDev: {:.2}",
                z_score, mean, std_dev
            )),
            ..Default::default()
        };
        
        Some(Signal::new(symbol, direction, strength)
            .with_confidence(Decimal::from(70))
            .with_metadata(metadata))
    }
    
    /// Calculate mean and standard deviation
    fn calculate_stats(&self) -> Option<(Decimal, Decimal)> {
        if self.price_history.is_empty() {
            return None;
        }
        
        let sum: Decimal = self.price_history.iter().sum();
        let count = Decimal::from(self.price_history.len() as i64);
        let mean = sum / count;
        
        let variance: Decimal = self.price_history.iter()
            .map(|p| {
                let diff = *p - mean;
                diff * diff
            })
            .sum::<Decimal>() / count;
        
        // Approximate sqrt for variance
        let std_dev = self.approx_sqrt(variance);
        
        Some((mean, std_dev))
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
    
    /// Calculate Bollinger Bands
    pub fn bollinger_bands(&self) -> Option<(Decimal, Decimal, Decimal)> {
        let (mean, std_dev) = self.calculate_stats()?;
        let multiplier = self.config.bb_std_dev;
        
        let upper = mean + std_dev * multiplier;
        let lower = mean - std_dev * multiplier;
        
        Some((lower, mean, upper))
    }
    
    /// Generate signal using Bollinger Bands
    pub fn generate_bb_signal(&self, symbol: &str) -> Option<Signal> {
        let (lower, _middle, upper) = self.bollinger_bands()?;
        let current = *self.price_history.back()?;
        
        if current > upper {
            let strength = ((current - upper) / (upper - lower))
                .min(Decimal::ONE);
            Some(Signal::new(symbol, SignalDirection::Short, strength))
        } else if current < lower {
            let strength = ((lower - current) / (upper - lower))
                .min(Decimal::ONE);
            Some(Signal::new(symbol, SignalDirection::Long, strength))
        } else {
            None
        }
    }
}

/// Simple Moving Average crossover for mean reversion
pub struct SmaCrossover {
    pub short_period: usize,
    pub long_period: usize,
}

impl SmaCrossover {
    pub fn new(short_period: usize, long_period: usize) -> Self {
        Self { short_period, long_period }
    }
    
    /// Calculate SMA
    pub fn calculate_sma(&self, data: &[PriceData], period: usize) -> Option<Decimal> {
        if data.len() < period {
            return None;
        }
        
        let sum: Decimal = data[data.len() - period..]
            .iter()
            .map(|d| d.close)
            .sum();
        
        Some(sum / Decimal::from(period as i64))
    }
    
    /// Generate signal based on price vs SMA
    pub fn generate_signal(&self, symbol: &str, data: &[PriceData]) -> Option<Signal> {
        if data.len() < self.long_period {
            return None;
        }
        
        let _short_sma = self.calculate_sma(data, self.short_period)?;
        let long_sma = self.calculate_sma(data, self.long_period)?;
        let current = data.last()?.close;
        
        // Price far above long SMA = overbought (short)
        // Price far below long SMA = oversold (long)
        let deviation = (current - long_sma) / long_sma;
        let threshold = Decimal::try_from(0.03).unwrap(); // 3%
        
        if deviation > threshold {
            let strength = (deviation - threshold) / threshold;
            Some(Signal::new(symbol, SignalDirection::Short, strength.min(Decimal::ONE)))
        } else if deviation < -threshold {
            let strength = (-deviation - threshold) / threshold;
            Some(Signal::new(symbol, SignalDirection::Long, strength.min(Decimal::ONE)))
        } else {
            None
        }
    }
}

// Implement Strategy trait
impl super::Strategy for MeanReversionStrategy {
    fn name(&self) -> &str {
        "MeanReversion"
    }
    
    fn strategy_type(&self) -> super::StrategyType {
        super::StrategyType::MeanReversion
    }
    
    fn generate_signal(&self, symbol: &str, data: &[PriceData]) -> super::Result<Signal> {
        // Update internal state
        if let Some(last) = data.last() {
            let mut this = Self::new(self.config.clone());
            for price in &self.price_history {
                this.update_price(*price);
            }
            this.update_price(last.close);
            
            if let Some(signal) = this.generate_signal(symbol) {
                return Ok(signal);
            }
        }
        
        Err(super::StrategyError::SignalGenerationFailed(
            "No signal generated".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    fn create_test_data(prices: Vec<i64>) -> Vec<PriceData> {
        prices.into_iter()
            .enumerate()
            .map(|(i, price)| PriceData {
                timestamp: Utc::now() + chrono::Duration::hours(i as i64),
                open: Decimal::from(price),
                high: Decimal::from(price + 5),
                low: Decimal::from(price - 5),
                close: Decimal::from(price),
                volume: Decimal::from(1000),
            })
            .collect()
    }
    
    #[test]
    fn test_mean_reversion_signal() {
        let config = MeanReversionConfig::default();
        let mut strategy = MeanReversionStrategy::new(config);
        
        // Prices around 100, then spike to 130 (2.5 std dev)
        let prices: Vec<i64> = vec![
            100, 101, 99, 102, 100, 101, 99, 100, 102, 101,
            100, 99, 101, 100, 102, 100, 99, 101, 100, 130 // Spike!
        ];
        
        for price in prices {
            strategy.update_price(Decimal::from(price));
        }
        
        let signal = strategy.generate_signal("BTC").unwrap();
        
        // Should be short signal (price too high)
        assert_eq!(signal.direction, SignalDirection::Short);
    }
    
    #[test]
    fn test_bollinger_bands() {
        let mut strategy = MeanReversionStrategy::new(MeanReversionConfig::default());
        
        // Stable prices around 100
        for _ in 0..20 {
            strategy.update_price(Decimal::from(100));
        }
        
        let bands = strategy.bollinger_bands().unwrap();
        
        // Middle should be 100
        assert_eq!(bands.1, Decimal::from(100));
        // Upper >= Middle >= Lower (with stable prices, all equal)
        assert!(bands.2 >= bands.1);
        assert!(bands.1 >= bands.0);
    }
    
    #[test]
    fn test_sma_crossover() {
        let crossover = SmaCrossover::new(5, 20);
        
        // Create data with prices trending up then spiking
        let prices: Vec<i64> = (0..25)
            .map(|i| 100 + i * 2)
            .collect();
        let data = create_test_data(prices);
        
        let sma5 = crossover.calculate_sma(&data, 5).unwrap();
        let sma20 = crossover.calculate_sma(&data, 20).unwrap();
        
        // Short SMA should be higher in uptrend
        assert!(sma5 > sma20);
    }
    
    #[test]
    fn test_not_enough_data() {
        let config = MeanReversionConfig {
            lookback_period: 50,
            ..Default::default()
        };
        let mut strategy = MeanReversionStrategy::new(config);
        
        for i in 0..10 {
            strategy.update_price(Decimal::from(100 + i));
        }
        
        let signal = strategy.generate_signal("BTC");
        assert!(signal.is_none());
    }
}
