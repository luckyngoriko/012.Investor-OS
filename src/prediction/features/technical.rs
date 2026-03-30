//! Technical indicator computations (Sprint 118).
//!
//! Manual implementations to avoid external dependency bloat.
//! All indicators accept OHLCV slices and return f64 values.

use serde::Serialize;

/// OHLCV candle for indicator computation.
#[derive(Debug, Clone, Copy)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Computed technical features for a symbol at a point in time.
#[derive(Debug, Clone, Serialize)]
pub struct TechnicalFeatures {
    pub rsi_14: f64,
    pub macd_line: f64,
    pub macd_signal: f64,
    pub macd_histogram: f64,
    pub bb_upper: f64,
    pub bb_middle: f64,
    pub bb_lower: f64,
    pub bb_position: f64,
    pub atr_14: f64,
    pub obv: f64,
    pub obv_trend: f64,
    pub volume_change_pct: f64,
    pub price_change_pct_5: f64,
    pub price_change_pct_20: f64,
}

/// Compute all technical features from a candle history.
///
/// Requires at least 30 candles for meaningful results.
/// Returns `None` if insufficient data.
pub fn compute_technical_features(candles: &[Candle]) -> Option<TechnicalFeatures> {
    if candles.len() < 30 {
        return None;
    }

    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let volumes: Vec<f64> = candles.iter().map(|c| c.volume).collect();

    let rsi_14 = rsi(&closes, 14)?;
    let (macd_line, macd_signal, macd_histogram) = macd(&closes, 12, 26, 9)?;
    let (bb_upper, bb_middle, bb_lower) = bollinger_bands(&closes, 20, 2.0)?;
    let atr_14 = atr(candles, 14)?;
    let obv_val = obv(&closes, &volumes);
    let obv_trend = obv_slope(&closes, &volumes, 10);

    let last = *closes.last()?;
    let bb_range = bb_upper - bb_lower;
    let bb_position = if bb_range > 0.0 {
        (last - bb_lower) / bb_range
    } else {
        0.5
    };

    let volume_change_pct = if volumes.len() >= 2 {
        let prev = volumes[volumes.len() - 2];
        if prev > 0.0 {
            (volumes.last().unwrap() - prev) / prev
        } else {
            0.0
        }
    } else {
        0.0
    };

    let price_change_pct_5 = pct_change(&closes, 5);
    let price_change_pct_20 = pct_change(&closes, 20);

    Some(TechnicalFeatures {
        rsi_14,
        macd_line,
        macd_signal,
        macd_histogram,
        bb_upper,
        bb_middle,
        bb_lower,
        bb_position,
        atr_14,
        obv: obv_val,
        obv_trend,
        volume_change_pct,
        price_change_pct_5,
        price_change_pct_20,
    })
}

// ── RSI ──────────────────────────────────────────────────────────────

/// Relative Strength Index (Wilder's smoothing).
fn rsi(closes: &[f64], period: usize) -> Option<f64> {
    if closes.len() < period + 1 {
        return None;
    }

    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;

    // Initial averages
    for i in 1..=period {
        let diff = closes[i] - closes[i - 1];
        if diff > 0.0 {
            avg_gain += diff;
        } else {
            avg_loss += diff.abs();
        }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    // Wilder's smoothing for remaining
    for i in (period + 1)..closes.len() {
        let diff = closes[i] - closes[i - 1];
        let (gain, loss) = if diff > 0.0 {
            (diff, 0.0)
        } else {
            (0.0, diff.abs())
        };
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
    }

    if avg_loss == 0.0 {
        return Some(100.0);
    }
    let rs = avg_gain / avg_loss;
    Some(100.0 - 100.0 / (1.0 + rs))
}

// ── MACD ─────────────────────────────────────────────────────────────

/// MACD (Moving Average Convergence Divergence).
/// Returns (macd_line, signal_line, histogram).
fn macd(closes: &[f64], fast: usize, slow: usize, signal_period: usize) -> Option<(f64, f64, f64)> {
    if closes.len() < slow + signal_period {
        return None;
    }

    let fast_ema = ema_series(closes, fast);
    let slow_ema = ema_series(closes, slow);

    let macd_line: Vec<f64> = fast_ema
        .iter()
        .zip(slow_ema.iter())
        .map(|(f, s)| f - s)
        .collect();

    let signal = ema_series(&macd_line, signal_period);

    let last_macd = *macd_line.last()?;
    let last_signal = *signal.last()?;

    Some((last_macd, last_signal, last_macd - last_signal))
}

/// Exponential Moving Average series.
fn ema_series(data: &[f64], period: usize) -> Vec<f64> {
    if data.is_empty() || period == 0 {
        return vec![];
    }

    let k = 2.0 / (period as f64 + 1.0);
    let mut result = Vec::with_capacity(data.len());

    // SMA for the first value
    let sma: f64 = data.iter().take(period).sum::<f64>() / period as f64;
    result.push(sma);

    for &val in &data[period..] {
        let prev = *result.last().unwrap();
        result.push(val * k + prev * (1.0 - k));
    }

    result
}

// ── Bollinger Bands ──────────────────────────────────────────────────

/// Bollinger Bands (SMA ± num_std * std_dev).
/// Returns (upper, middle, lower).
fn bollinger_bands(closes: &[f64], period: usize, num_std: f64) -> Option<(f64, f64, f64)> {
    if closes.len() < period {
        return None;
    }

    let window = &closes[closes.len() - period..];
    let mean: f64 = window.iter().sum::<f64>() / period as f64;
    let variance: f64 = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / period as f64;
    let std_dev = variance.sqrt();

    Some((mean + num_std * std_dev, mean, mean - num_std * std_dev))
}

// ── ATR ──────────────────────────────────────────────────────────────

/// Average True Range (Wilder's smoothing).
fn atr(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() < period + 1 {
        return None;
    }

    let mut tr_values: Vec<f64> = Vec::with_capacity(candles.len() - 1);
    for i in 1..candles.len() {
        let high_low = candles[i].high - candles[i].low;
        let high_prev_close = (candles[i].high - candles[i - 1].close).abs();
        let low_prev_close = (candles[i].low - candles[i - 1].close).abs();
        tr_values.push(high_low.max(high_prev_close).max(low_prev_close));
    }

    // Initial ATR = SMA of first `period` TRs
    let mut current_atr: f64 = tr_values.iter().take(period).sum::<f64>() / period as f64;

    // Wilder's smoothing
    for &tr in &tr_values[period..] {
        current_atr = (current_atr * (period as f64 - 1.0) + tr) / period as f64;
    }

    Some(current_atr)
}

// ── OBV ──────────────────────────────────────────────────────────────

/// On-Balance Volume — cumulative volume based on price direction.
fn obv(closes: &[f64], volumes: &[f64]) -> f64 {
    let mut result = 0.0;
    for i in 1..closes.len().min(volumes.len()) {
        if closes[i] > closes[i - 1] {
            result += volumes[i];
        } else if closes[i] < closes[i - 1] {
            result -= volumes[i];
        }
    }
    result
}

/// OBV slope over the last `window` candles (normalized).
fn obv_slope(closes: &[f64], volumes: &[f64], window: usize) -> f64 {
    let n = closes.len().min(volumes.len());
    if n < window + 1 {
        return 0.0;
    }

    let end_obv = obv(closes, volumes);
    let start_obv = obv(&closes[..n - window], &volumes[..n - window]);
    let avg_vol: f64 = volumes.iter().rev().take(window).sum::<f64>() / window as f64;

    if avg_vol > 0.0 {
        (end_obv - start_obv) / avg_vol
    } else {
        0.0
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Percentage change over `lookback` periods.
fn pct_change(data: &[f64], lookback: usize) -> f64 {
    if data.len() <= lookback {
        return 0.0;
    }
    let current = data[data.len() - 1];
    let previous = data[data.len() - 1 - lookback];
    if previous != 0.0 {
        (current - previous) / previous
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candles(n: usize) -> Vec<Candle> {
        // Simulated uptrend with noise
        (0..n)
            .map(|i| {
                let base = 100.0 + i as f64 * 0.5;
                let noise = ((i * 7 % 13) as f64 - 6.0) * 0.3;
                Candle {
                    open: base - 0.2 + noise,
                    high: base + 1.5 + noise.abs(),
                    low: base - 1.0 - noise.abs(),
                    close: base + noise,
                    volume: 1000.0 + (i * 11 % 17) as f64 * 100.0,
                }
            })
            .collect()
    }

    #[test]
    fn compute_features_requires_min_30_candles() {
        let short = sample_candles(20);
        assert!(compute_technical_features(&short).is_none());
    }

    #[test]
    fn compute_features_works_with_50_candles() {
        let candles = sample_candles(50);
        let features = compute_technical_features(&candles).unwrap();

        // RSI should be 0-100
        assert!(features.rsi_14 >= 0.0 && features.rsi_14 <= 100.0);
        // Bollinger position should be 0-1 (roughly)
        assert!(features.bb_position >= -0.5 && features.bb_position <= 1.5);
        // ATR should be positive
        assert!(features.atr_14 > 0.0);
        // BB upper > middle > lower
        assert!(features.bb_upper > features.bb_middle);
        assert!(features.bb_middle > features.bb_lower);
    }

    #[test]
    fn rsi_returns_valid_range() {
        let closes: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 0.5).collect();
        let result = rsi(&closes, 14).unwrap();
        assert!(result >= 0.0 && result <= 100.0);
        // Strong uptrend → RSI > 50
        assert!(result > 50.0);
    }

    #[test]
    fn macd_returns_three_values() {
        let closes: Vec<f64> = (0..50)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 5.0)
            .collect();
        let (line, signal, hist) = macd(&closes, 12, 26, 9).unwrap();
        assert!((hist - (line - signal)).abs() < 1e-10);
    }

    #[test]
    fn bollinger_bands_symmetric() {
        let closes: Vec<f64> = vec![100.0; 30];
        let (upper, middle, lower) = bollinger_bands(&closes, 20, 2.0).unwrap();
        // Constant price → zero std dev → bands collapse to middle
        assert!((upper - middle).abs() < 1e-10);
        assert!((middle - lower).abs() < 1e-10);
        assert!((middle - 100.0).abs() < 1e-10);
    }

    #[test]
    fn atr_positive_for_volatile_data() {
        let candles = sample_candles(50);
        let result = atr(&candles, 14).unwrap();
        assert!(result > 0.0);
    }

    #[test]
    fn obv_accumulates_correctly() {
        let closes = vec![10.0, 11.0, 10.5, 11.5, 12.0];
        let volumes = vec![100.0, 200.0, 150.0, 300.0, 250.0];
        let result = obv(&closes, &volumes);
        // Up: +200, Down: -150, Up: +300, Up: +250 = 600
        assert!((result - 600.0).abs() < 1e-10);
    }

    #[test]
    fn compute_features_fast_enough() {
        let candles = sample_candles(500);
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = compute_technical_features(&candles);
        }
        let elapsed = start.elapsed();
        // 1000 iterations of 500 candles should be < 1 second
        assert!(elapsed.as_millis() < 1000, "Too slow: {:?}", elapsed);
    }
}
