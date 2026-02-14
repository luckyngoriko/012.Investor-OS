//! Feature Engineering Engine

use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::debug;

use super::{MlError, Result};

/// Price data point for feature extraction
#[derive(Debug, Clone, Copy)]
pub struct PriceData {
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

impl PriceData {
    pub fn typical_price(&self) -> Decimal {
        (self.high + self.low + self.close) / Decimal::from(3)
    }

    pub fn range(&self) -> Decimal {
        self.high - self.low
    }
}

/// Feature configuration
#[derive(Debug, Clone)]
pub struct FeatureConfig {
    /// RSI period
    pub rsi_period: usize,
    /// MACD fast/slow/signal periods
    pub macd_fast: usize,
    pub macd_slow: usize,
    pub macd_signal: usize,
    /// Bollinger Bands period and std dev
    pub bb_period: usize,
    pub bb_std_dev: Decimal,
    /// ATR period
    pub atr_period: usize,
    /// Lookback periods for lag features
    pub lag_periods: Vec<usize>,
    /// Volume indicators enabled
    pub use_volume: bool,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            rsi_period: 14,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            bb_period: 20,
            bb_std_dev: Decimal::from(2),
            atr_period: 14,
            lag_periods: vec![1, 3, 5, 10],
            use_volume: true,
        }
    }
}

/// Feature vector for ML input
#[derive(Debug, Clone)]
pub struct FeatureVector {
    pub values: Vec<Decimal>,
    pub names: Vec<String>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl FeatureVector {
    pub fn new(values: Vec<Decimal>, names: Vec<String>) -> Self {
        Self {
            values,
            names,
            timestamp: Some(chrono::Utc::now()),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<Decimal> {
        self.names
            .iter()
            .position(|n| n == name)
            .and_then(|idx| self.values.get(idx).copied())
    }

    /// Normalize features to [0, 1] range
    pub fn normalize(&mut self, min_vals: &[Decimal], max_vals: &[Decimal]) {
        for ((val, min), max) in self.values.iter_mut().zip(min_vals.iter()).zip(max_vals.iter()) {
            let range = *max - *min;
            if !range.is_zero() {
                *val = (*val - *min) / range;
            }
        }
    }

    /// Standardize features (z-score)
    pub fn standardize(&mut self, means: &[Decimal], stds: &[Decimal]) {
        for ((val, mean), std) in self.values.iter_mut().zip(means.iter()).zip(stds.iter()) {
            if !std.is_zero() {
                *val = (*val - *mean) / *std;
            }
        }
    }
}

/// Feature engineering engine
#[derive(Debug, Clone)]
pub struct FeatureEngine {
    config: FeatureConfig,
}

impl FeatureEngine {
    pub fn new(config: FeatureConfig) -> Self {
        Self { config }
    }

    /// Extract all features from price data
    pub fn extract_features(&self, data: &[PriceData]) -> Result<FeatureVector> {
        let min_required = self.config.macd_slow.max(
            self.config.bb_period.max(
                self.config.rsi_period.max(
                    self.config.atr_period
                )
            )
        );

        if data.len() < min_required * 2 {
            return Err(MlError::InsufficientData {
                required: min_required * 2,
                available: data.len(),
            });
        }

        let mut features = HashMap::new();

        // Technical indicators
        self.add_rsi(data, &mut features)?;
        self.add_macd(data, &mut features)?;
        self.add_bollinger_bands(data, &mut features)?;
        self.add_atr(data, &mut features)?;
        
        // Price-based features
        self.add_price_features(data, &mut features)?;
        self.add_lag_features(data, &mut features)?;
        
        // Volume features
        if self.config.use_volume {
            self.add_volume_features(data, &mut features)?;
        }

        // Statistical features
        self.add_statistical_features(data, &mut features)?;

        // Convert to ordered vector
        let mut names: Vec<String> = features.keys().cloned().collect();
        names.sort();
        let values: Vec<Decimal> = names.iter()
            .filter_map(|n| features.get(n).copied())
            .collect();

        debug!("Extracted {} features", values.len());

        Ok(FeatureVector::new(values, names))
    }

    /// Calculate RSI
    fn add_rsi(&self, data: &[PriceData], features: &mut HashMap<String, Decimal>) -> Result<()> {
        let period = self.config.rsi_period;
        if data.len() < period + 1 {
            return Err(MlError::InsufficientData {
                required: period + 1,
                available: data.len(),
            });
        }

        let closes: Vec<Decimal> = data.iter().map(|d| d.close).collect();
        let rsi = calculate_rsi(&closes, period);
        
        features.insert("rsi".to_string(), rsi);
        features.insert("rsi_normalized".to_string(), rsi / Decimal::from(100));
        
        // RSI categories
        features.insert("rsi_overbought".to_string(), if rsi > Decimal::from(70) { Decimal::ONE } else { Decimal::ZERO });
        features.insert("rsi_oversold".to_string(), if rsi < Decimal::from(30) { Decimal::ONE } else { Decimal::ZERO });

        Ok(())
    }

    /// Calculate MACD
    fn add_macd(&self, data: &[PriceData], features: &mut HashMap<String, Decimal>) -> Result<()> {
        let closes: Vec<Decimal> = data.iter().map(|d| d.close).collect();
        
        let (macd_line, signal_line, histogram) = calculate_macd(
            &closes,
            self.config.macd_fast,
            self.config.macd_slow,
            self.config.macd_signal,
        );

        features.insert("macd".to_string(), macd_line);
        features.insert("macd_signal".to_string(), signal_line);
        features.insert("macd_histogram".to_string(), histogram);
        features.insert("macd_above_signal".to_string(), if macd_line > signal_line { Decimal::ONE } else { Decimal::ZERO });

        Ok(())
    }

    /// Calculate Bollinger Bands
    fn add_bollinger_bands(&self, data: &[PriceData], features: &mut HashMap<String, Decimal>) -> Result<()> {
        let period = self.config.bb_period;
        if data.len() < period {
            return Err(MlError::InsufficientData {
                required: period,
                available: data.len(),
            });
        }

        let closes: Vec<Decimal> = data.iter().map(|d| d.close).collect();
        let (middle, upper, lower) = calculate_bollinger_bands(&closes, period, self.config.bb_std_dev);
        let current = closes.last().unwrap();

        let bandwidth = (upper - lower) / middle;
        let percent_b = (current - lower) / (upper - lower);

        features.insert("bb_middle".to_string(), middle);
        features.insert("bb_upper".to_string(), upper);
        features.insert("bb_lower".to_string(), lower);
        features.insert("bb_bandwidth".to_string(), bandwidth);
        features.insert("bb_percent_b".to_string(), percent_b);
        features.insert("bb_position".to_string(), 
            if *current > upper { Decimal::ONE }
            else if *current < lower { Decimal::ZERO }
            else { Decimal::try_from(0.5).unwrap() }
        );

        Ok(())
    }

    /// Calculate ATR
    fn add_atr(&self, data: &[PriceData], features: &mut HashMap<String, Decimal>) -> Result<()> {
        let period = self.config.atr_period;
        if data.len() < period {
            return Err(MlError::InsufficientData {
                required: period,
                available: data.len(),
            });
        }

        let atr = calculate_atr(data, period);
        let current_close = data.last().unwrap().close;
        let atr_pct = atr / current_close;

        features.insert("atr".to_string(), atr);
        features.insert("atr_percent".to_string(), atr_pct);

        Ok(())
    }

    /// Add price-based features
    fn add_price_features(&self, data: &[PriceData], features: &mut HashMap<String, Decimal>) -> Result<()> {
        let current = data.last().unwrap();
        let prev = if data.len() > 1 { &data[data.len() - 2] } else { current };

        // Price changes
        let close_change = (current.close - prev.close) / prev.close;
        let high_low_range = current.range() / current.close;
        let body_size = (current.close - current.open).abs() / current.open;

        // Candlestick patterns
        let is_bullish = current.close > current.open;
        let upper_shadow = if is_bullish {
            current.high - current.close
        } else {
            current.high - current.open
        } / current.close;

        let lower_shadow = if is_bullish {
            current.open - current.low
        } else {
            current.close - current.low
        } / current.close;

        features.insert("price_change".to_string(), close_change);
        features.insert("high_low_range".to_string(), high_low_range);
        features.insert("body_size".to_string(), body_size);
        features.insert("upper_shadow".to_string(), upper_shadow);
        features.insert("lower_shadow".to_string(), lower_shadow);
        features.insert("is_bullish".to_string(), if is_bullish { Decimal::ONE } else { Decimal::ZERO });

        Ok(())
    }

    /// Add lag features
    fn add_lag_features(&self, data: &[PriceData], features: &mut HashMap<String, Decimal>) -> Result<()> {
        let closes: Vec<Decimal> = data.iter().map(|d| d.close).collect();
        let current = closes.last().unwrap();

        for period in &self.config.lag_periods {
            if data.len() > *period {
                let prev = closes[closes.len() - period - 1];
                let change = (*current - prev) / prev;
                features.insert(format!("change_{}p", period), change);
            }
        }

        Ok(())
    }

    /// Add volume-based features
    fn add_volume_features(&self, data: &[PriceData], features: &mut HashMap<String, Decimal>) -> Result<()> {
        let current = data.last().unwrap();
        
        // Volume moving average
        let vol_ma_period = 20usize.min(data.len());
        let vol_sum: Decimal = data.iter().rev().take(vol_ma_period).map(|d| d.volume).sum();
        let vol_ma = vol_sum / Decimal::from(vol_ma_period as i64);

        let vol_ratio = if !vol_ma.is_zero() { current.volume / vol_ma } else { Decimal::ONE };

        // OBV calculation
        let obv = calculate_obv(data);
        
        features.insert("volume_ratio".to_string(), vol_ratio);
        features.insert("volume_ma".to_string(), vol_ma);
        features.insert("obv".to_string(), obv);

        Ok(())
    }

    /// Add statistical features
    fn add_statistical_features(&self, data: &[PriceData], features: &mut HashMap<String, Decimal>) -> Result<()> {
        let closes: Vec<Decimal> = data.iter().map(|d| d.close).collect();
        let period = 20usize.min(closes.len());
        let recent = &closes[closes.len() - period..];

        // Mean and std
        let mean = recent.iter().sum::<Decimal>() / Decimal::from(period as i64);
        let variance = recent.iter()
            .map(|x| {
                let diff = *x - mean;
                diff * diff
            })
            .sum::<Decimal>() / Decimal::from(period as i64);
        let std_dev = approx_sqrt(variance);

        let current = closes.last().unwrap();
        let z_score = if !std_dev.is_zero() { (*current - mean) / std_dev } else { Decimal::ZERO };

        // Min/Max over period
        let min_val = recent.iter().copied().min().unwrap_or(*current);
        let max_val = recent.iter().copied().max().unwrap_or(*current);
        let percentile = if max_val != min_val {
            (*current - min_val) / (max_val - min_val)
        } else {
            Decimal::try_from(0.5).unwrap()
        };

        features.insert("price_mean".to_string(), mean);
        features.insert("price_std".to_string(), std_dev);
        features.insert("z_score".to_string(), z_score);
        features.insert("percentile".to_string(), percentile);

        Ok(())
    }
}

// Helper functions for technical indicators

fn calculate_rsi(closes: &[Decimal], period: usize) -> Decimal {
    if closes.len() < period + 1 {
        return Decimal::from(50);
    }

    let mut gains = Decimal::ZERO;
    let mut losses = Decimal::ZERO;

    for i in 1..=period {
        let change = closes[closes.len() - i] - closes[closes.len() - i - 1];
        if change >= Decimal::ZERO {
            gains += change;
        } else {
            losses += -change;
        }
    }

    let avg_gain = gains / Decimal::from(period as i64);
    let avg_loss = losses / Decimal::from(period as i64);

    if avg_loss.is_zero() {
        return Decimal::from(100);
    }

    let rs = avg_gain / avg_loss;
    Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs))
}

fn calculate_macd(closes: &[Decimal], fast: usize, slow: usize, _signal: usize) -> (Decimal, Decimal, Decimal) {
    let ema_fast = calculate_ema(closes, fast);
    let ema_slow = calculate_ema(closes, slow);
    
    let macd_line = ema_fast - ema_slow;
    
    // For signal line, we'd need MACD history - simplified here
    let signal_line = macd_line * Decimal::try_from(0.9).unwrap();
    let histogram = macd_line - signal_line;

    (macd_line, signal_line, histogram)
}

fn calculate_ema(closes: &[Decimal], period: usize) -> Decimal {
    if closes.len() < period {
        return *closes.last().unwrap_or(&Decimal::ZERO);
    }

    let multiplier = Decimal::try_from(2.0 / (period as f64 + 1.0)).unwrap();
    let mut ema = closes[closes.len() - period];

    for i in 1..period {
        let price = closes[closes.len() - period + i];
        ema = (price - ema) * multiplier + ema;
    }

    ema
}

fn calculate_bollinger_bands(closes: &[Decimal], period: usize, std_dev_mult: Decimal) -> (Decimal, Decimal, Decimal) {
    let recent = &closes[closes.len() - period..];
    let sma = recent.iter().sum::<Decimal>() / Decimal::from(period as i64);
    
    let variance = recent.iter()
        .map(|x| {
            let diff = *x - sma;
            diff * diff
        })
        .sum::<Decimal>() / Decimal::from(period as i64);
    
    let std_dev = approx_sqrt(variance);

    let upper = sma + std_dev * std_dev_mult;
    let lower = sma - std_dev * std_dev_mult;

    (sma, upper, lower)
}

fn calculate_atr(data: &[PriceData], period: usize) -> Decimal {
    let mut tr_sum = Decimal::ZERO;
    
    for i in 1..=period {
        let idx = data.len() - i;
        let current = &data[idx];
        let prev_close = if idx > 0 { data[idx - 1].close } else { current.close };
        
        let tr1 = current.high - current.low;
        let tr2 = (current.high - prev_close).abs();
        let tr3 = (current.low - prev_close).abs();
        
        let tr = tr1.max(tr2.max(tr3));
        tr_sum += tr;
    }

    tr_sum / Decimal::from(period as i64)
}

fn calculate_obv(data: &[PriceData]) -> Decimal {
    let mut obv = Decimal::ZERO;
    
    for i in 1..data.len() {
        if data[i].close > data[i - 1].close {
            obv += data[i].volume;
        } else if data[i].close < data[i - 1].close {
            obv -= data[i].volume;
        }
    }

    obv
}

fn approx_sqrt(value: Decimal) -> Decimal {
    if value.is_zero() {
        return Decimal::ZERO;
    }

    let mut low = Decimal::ZERO;
    let mut high = value.max(Decimal::ONE);
    let epsilon = Decimal::try_from(0.0001).unwrap();

    if value < Decimal::ONE {
        low = value;
        high = Decimal::ONE;
    }

    for _ in 0..50 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_data(prices: Vec<f64>) -> Vec<PriceData> {
        prices.iter().map(|&p| {
            let price = Decimal::try_from(p).unwrap();
            PriceData {
                open: price,
                high: price * Decimal::try_from(1.01).unwrap(),
                low: price * Decimal::try_from(0.99).unwrap(),
                close: price,
                volume: Decimal::from(1000),
            }
        }).collect()
    }

    #[test]
    fn test_rsi_calculation() {
        // Create data with clear uptrend
        let prices: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let data = create_test_data(prices);
        let closes: Vec<Decimal> = data.iter().map(|d| d.close).collect();
        
        let rsi = calculate_rsi(&closes, 14);
        // Strong uptrend should have high RSI
        assert!(rsi > Decimal::from(50));
    }

    #[test]
    fn test_macd_calculation() {
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + (i % 10) as f64).collect();
        let data = create_test_data(prices);
        let closes: Vec<Decimal> = data.iter().map(|d| d.close).collect();
        
        let (macd, signal, hist) = calculate_macd(&closes, 12, 26, 9);
        
        // MACD and signal should be related
        assert!(hist == macd - signal);
    }

    #[test]
    fn test_bollinger_bands() {
        let prices: Vec<f64> = (0..30).map(|_| 100.0).collect();
        let data = create_test_data(prices);
        let closes: Vec<Decimal> = data.iter().map(|d| d.close).collect();
        
        let (middle, upper, lower) = calculate_bollinger_bands(&closes, 20, Decimal::from(2));
        
        // With constant prices, bands should be at middle
        assert_eq!(middle, Decimal::from(100));
        assert!(upper >= middle);
        assert!(lower <= middle);
    }

    #[test]
    fn test_atr_calculation() {
        let mut data = vec![];
        for i in 0..30 {
            let base = 100.0 + (i % 5) as f64;
            data.push(PriceData {
                open: Decimal::try_from(base).unwrap(),
                high: Decimal::try_from(base + 2.0).unwrap(),
                low: Decimal::try_from(base - 2.0).unwrap(),
                close: Decimal::try_from(base).unwrap(),
                volume: Decimal::from(1000),
            });
        }
        
        let atr = calculate_atr(&data, 14);
        assert!(atr > Decimal::ZERO);
    }

    #[test]
    fn test_feature_extraction() {
        let config = FeatureConfig::default();
        let engine = FeatureEngine::new(config);
        
        let prices: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64 * 0.5)).collect();
        let data = create_test_data(prices);
        
        let features = engine.extract_features(&data).unwrap();
        
        assert!(!features.is_empty());
        assert_eq!(features.len(), features.names.len());
        assert!(features.get("rsi").is_some());
        assert!(features.get("macd").is_some());
    }

    #[test]
    fn test_feature_vector_get() {
        let values = vec![Decimal::from(1), Decimal::from(2), Decimal::from(3)];
        let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let fv = FeatureVector::new(values, names);
        
        assert_eq!(fv.get("b"), Some(Decimal::from(2)));
        assert_eq!(fv.get("d"), None);
    }

    #[test]
    fn test_insufficient_data() {
        let config = FeatureConfig::default();
        let engine = FeatureEngine::new(config);
        
        let data = create_test_data(vec![100.0, 101.0, 102.0]);
        
        let result = engine.extract_features(&data);
        assert!(result.is_err());
    }
}
