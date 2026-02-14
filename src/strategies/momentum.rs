//! Momentum strategy implementation

use rust_decimal::Decimal;
use tracing::debug;

use super::types::{PriceData, Signal, SignalDirection, SignalMetadata};
use super::error::{StrategyError, Result};

/// Momentum strategy configuration
#[derive(Debug, Clone)]
pub struct MomentumConfig {
    pub short_period: usize,    // Short-term lookback (e.g., 12)
    pub long_period: usize,     // Long-term lookback (e.g., 26)
    pub signal_threshold: Decimal, // Minimum momentum to generate signal
    pub use_volume_confirmation: bool,
}

impl Default for MomentumConfig {
    fn default() -> Self {
        Self {
            short_period: 12,
            long_period: 26,
            signal_threshold: Decimal::try_from(0.02).unwrap(), // 2%
            use_volume_confirmation: true,
        }
    }
}

/// Momentum strategy
#[derive(Debug)]
pub struct MomentumStrategy {
    config: MomentumConfig,
    name: String,
}

impl MomentumStrategy {
    pub fn new(config: MomentumConfig) -> Self {
        Self {
            config,
            name: "Momentum".to_string(),
        }
    }
    
    /// Generate signal from price data
    pub fn generate_signal(&self, symbol: &str, data: &[PriceData]) -> Result<Signal> {
        if data.len() < self.config.long_period + 1 {
            return Err(StrategyError::InsufficientData {
                required: self.config.long_period + 1,
                available: data.len(),
            });
        }
        
        let recent_prices = &data[data.len() - self.config.long_period..];
        
        // Calculate short-term momentum
        let short_start = recent_prices[recent_prices.len() - self.config.short_period].close;
        let short_end = recent_prices.last().unwrap().close;
        let short_momentum = (short_end - short_start) / short_start;
        
        // Calculate long-term momentum
        let long_start = recent_prices[0].close;
        let long_end = recent_prices.last().unwrap().close;
        let long_momentum = (long_end - long_start) / long_start;
        
        debug!(
            "Momentum for {}: short={:.4}, long={:.4}",
            symbol, short_momentum, long_momentum
        );
        
        // Determine direction
        let direction = if short_momentum > long_momentum 
            && short_momentum > self.config.signal_threshold {
            SignalDirection::Long
        } else if short_momentum < long_momentum 
            && short_momentum < -self.config.signal_threshold {
            SignalDirection::Short
        } else {
            SignalDirection::Neutral
        };
        
        // Calculate signal strength (0 to 1)
        let strength = (short_momentum.abs() / Decimal::try_from(0.1).unwrap())
            .min(Decimal::ONE);
        
        // Volume confirmation
        let confidence = if self.config.use_volume_confirmation {
            self.calculate_volume_confidence(recent_prices)
        } else {
            Decimal::from(50)
        };
        
        let metadata = SignalMetadata {
            strategy_id: self.name.clone(),
            entry_price: Some(long_end),
            rationale: Some(format!(
                "Short momentum: {:.2}%, Long momentum: {:.2}%",
                short_momentum * Decimal::from(100),
                long_momentum * Decimal::from(100)
            )),
            ..Default::default()
        };
        
        Ok(Signal::new(symbol, direction, strength)
            .with_confidence(confidence)
            .with_metadata(metadata))
    }
    
    /// Calculate volume-based confidence
    fn calculate_volume_confidence(&self, data: &[PriceData]) -> Decimal {
        if data.len() < 2 {
            return Decimal::from(50);
        }
        
        let avg_volume: Decimal = data.iter()
            .map(|d| d.volume)
            .sum::<Decimal>() / Decimal::from(data.len() as i64);
        
        let recent_volume = data.last().unwrap().volume;
        
        if avg_volume.is_zero() {
            return Decimal::from(50);
        }
        
        let volume_ratio = recent_volume / avg_volume;
        
        // Higher volume = higher confidence (capped at 100%)
        (volume_ratio * Decimal::from(50)).min(Decimal::from(100))
    }
    
    /// Calculate Rate of Change (ROC)
    pub fn calculate_roc(data: &[PriceData], period: usize) -> Option<Decimal> {
        if data.len() < period + 1 {
            return None;
        }
        
        let current = data.last()?.close;
        let past = data[data.len() - period - 1].close;
        
        if past.is_zero() {
            return None;
        }
        
        Some((current - past) / past)
    }
}

/// RSI (Relative Strength Index) calculator
pub struct RSICalculator {
    pub period: usize,
}

impl RSICalculator {
    pub fn new(period: usize) -> Self {
        Self { period }
    }
    
    /// Calculate RSI
    pub fn calculate(&self, data: &[PriceData]) -> Option<Decimal> {
        if data.len() < self.period + 1 {
            return None;
        }
        
        let mut gains = Vec::new();
        let mut losses = Vec::new();
        
        for i in 1..=self.period {
            let change = data[data.len() - i].close - data[data.len() - i - 1].close;
            if change >= Decimal::ZERO {
                gains.push(change);
                losses.push(Decimal::ZERO);
            } else {
                gains.push(Decimal::ZERO);
                losses.push(-change);
            }
        }
        
        let avg_gain: Decimal = gains.iter().sum::<Decimal>() / Decimal::from(self.period as i64);
        let avg_loss: Decimal = losses.iter().sum::<Decimal>() / Decimal::from(self.period as i64);
        
        if avg_loss.is_zero() {
            return Some(Decimal::from(100));
        }
        
        let rs = avg_gain / avg_loss;
        let rsi = Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs));
        
        Some(rsi)
    }
    
    /// Generate signal from RSI
    pub fn generate_signal(&self, symbol: &str, data: &[PriceData]) -> Option<Signal> {
        let rsi = self.calculate(data)?;
        
        let (direction, strength) = if rsi > Decimal::from(70) {
            (SignalDirection::Short, (rsi - Decimal::from(70)) / Decimal::from(30))
        } else if rsi < Decimal::from(30) {
            (SignalDirection::Long, (Decimal::from(30) - rsi) / Decimal::from(30))
        } else {
            return None;
        };
        
        Some(Signal::new(symbol, direction, strength.min(Decimal::ONE)))
    }
}

// Implement Strategy trait
impl super::Strategy for MomentumStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn strategy_type(&self) -> super::StrategyType {
        super::StrategyType::Momentum
    }
    
    fn generate_signal(&self, symbol: &str, data: &[PriceData]) -> super::Result<Signal> {
        self.generate_signal(symbol, data)
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
                high: Decimal::from(price + 10),
                low: Decimal::from(price - 10),
                close: Decimal::from(price),
                volume: Decimal::from(1000 + i as i64 * 100),
            })
            .collect()
    }
    
    #[test]
    fn test_momentum_signal_generation() {
        // Use config with smaller periods for testing
        let config = MomentumConfig {
            short_period: 10,
            long_period: 20,
            signal_threshold: Decimal::try_from(0.02).unwrap(),
            use_volume_confirmation: true,
        };
        let strategy = MomentumStrategy::new(config);
        
        // Create accelerating uptrend data (prices rise faster at the end)
        // Need short_momentum > long_momentum for Long signal
        // long_momentum = (end - start_of_20) / start_of_20
        // short_momentum = (end - start_of_last_10) / start_of_last_10
        //
        // Strategy: Flat for first 10, slow rise for next 10, then jump
        // [100 x 10], [100, 105, 110, 115, 120, 125, 130, 135, 140, 145], 200
        // Start of 20 = 100, End = 200, long_momentum = (200-100)/100 = 1.0
        // Start of last 10 = 145, End = 200, short_momentum = (200-145)/145 = 0.379
        // That won't work either...
        //
        // Better: Make the last 10 period gain percentage higher than overall
        // Start at 100, dip to 50, then rise to 150
        // long_momentum = (150-100)/100 = 0.5
        // short_momentum = (150-50)/50 = 2.0
        let mut prices: Vec<i64> = (0..10).map(|i| 100 - i * 5).collect(); // Decline: 100, 95, 90...55
        prices.extend((0..10).map(|i| 50 + i * 10)); // Sharp rise: 50, 60, 70...140
        prices.push(150); // End at 150 (21 prices total)
        let data = create_test_data(prices);
        
        let signal = strategy.generate_signal("BTC", &data).unwrap();
        
        // Should detect acceleration = Long signal
        assert_eq!(signal.direction, SignalDirection::Long, 
            "Expected Long signal for accelerating uptrend");
        assert!(signal.strength > Decimal::ZERO);
    }
    
    #[test]
    fn test_momentum_insufficient_data() {
        let config = MomentumConfig {
            long_period: 50,
            ..Default::default()
        };
        let strategy = MomentumStrategy::new(config);
        
        let data = create_test_data(vec![100; 10]);
        
        let result = strategy.generate_signal("BTC", &data);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_calculate_roc() {
        let prices = vec![100, 102, 104, 103, 105, 107];
        let data = create_test_data(prices);
        
        let roc = MomentumStrategy::calculate_roc(&data, 5).unwrap();
        
        // (107 - 100) / 100 = 0.07 = 7%
        assert_eq!(roc, Decimal::try_from(0.07).unwrap());
    }
    
    #[test]
    fn test_rsi_calculation() {
        // Create alternating up/down data
        let prices: Vec<i64> = vec![
            100, 102, 101, 103, 102, 104, 103, 105, 104, 106,
            105, 107, 106, 108, 107
        ];
        let data = create_test_data(prices);
        
        let rsi_calc = RSICalculator::new(14);
        let rsi = rsi_calc.calculate(&data);
        
        assert!(rsi.is_some());
        let rsi = rsi.unwrap();
        assert!(rsi >= Decimal::ZERO && rsi <= Decimal::from(100));
    }
    
    #[test]
    fn test_rsi_overbought_signal() {
        // Strong uptrend - RSI should be high
        let prices: Vec<i64> = (0..20).map(|i| 100 + i * 10).collect();
        let data = create_test_data(prices);
        
        let rsi_calc = RSICalculator::new(14);
        let signal = rsi_calc.generate_signal("BTC", &data);
        
        // Should suggest short (overbought)
        if let Some(sig) = signal {
            assert_eq!(sig.direction, SignalDirection::Short);
        }
    }
}
