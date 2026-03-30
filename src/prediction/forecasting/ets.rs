//! ETS (Error-Trend-Seasonality) forecasting via augurs (Sprint 123).
//!
//! Exponential smoothing state space model for short-term price forecasting.

use augurs::ets::AutoETS;
use augurs::{Fit, Predict};
use serde::Serialize;

/// ETS forecast result.
#[derive(Debug, Clone, Serialize)]
pub struct EtsForecast {
    pub point_forecasts: Vec<f64>,
    pub model_type: String,
    pub horizon: usize,
}

/// Run AutoETS on a time series and forecast `horizon` steps ahead.
///
/// AutoETS automatically selects the best ETS model variant.
/// `season_length` of 1 means non-seasonal.
pub fn forecast_ets(data: &[f64], horizon: usize) -> Result<EtsForecast, String> {
    if data.len() < 10 {
        return Err(format!("Need at least 10 data points, got {}", data.len()));
    }

    let model = AutoETS::new(1, "ZZN").map_err(|e| format!("AutoETS init failed: {e}"))?;

    let fitted = model
        .fit(data)
        .map_err(|e| format!("AutoETS fit failed: {e}"))?;

    let forecasts = fitted
        .predict(horizon, 0.95)
        .map_err(|e| format!("AutoETS predict failed: {e}"))?;

    Ok(EtsForecast {
        point_forecasts: forecasts.point.to_vec(),
        model_type: "AutoETS".to_string(),
        horizon,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ets_forecast_uptrend() {
        let data: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 0.5).collect();
        let result = forecast_ets(&data, 5).unwrap();
        assert_eq!(result.point_forecasts.len(), 5);
        assert_eq!(result.horizon, 5);
        // Forecasts should continue roughly in same range
        assert!(result.point_forecasts[0] > 100.0);
    }

    #[test]
    fn ets_forecast_constant() {
        let data: Vec<f64> = vec![50.0; 30];
        let result = forecast_ets(&data, 3).unwrap();
        for f in &result.point_forecasts {
            assert!((f - 50.0).abs() < 5.0, "Expected ~50, got {f}");
        }
    }

    #[test]
    fn ets_rejects_short_series() {
        let data: Vec<f64> = vec![1.0; 5];
        assert!(forecast_ets(&data, 3).is_err());
    }

    #[test]
    fn ets_performance() {
        let data: Vec<f64> = (0..200)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = forecast_ets(&data, 24);
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_secs() < 10, "Too slow: {:?}", elapsed);
    }
}
