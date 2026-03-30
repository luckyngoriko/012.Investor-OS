//! MSTL (Multiple Seasonal-Trend decomposition with LOESS) via augurs (Sprint 123).
//!
//! Decomposes a time series into trend + seasonal + remainder,
//! then forecasts using ETS on the decomposed components.

use augurs::ets::AutoETS;
use augurs::mstl::MSTLModel;
use augurs::{Fit, Predict};
use serde::Serialize;

/// MSTL decomposition + forecast result.
#[derive(Debug, Clone, Serialize)]
pub struct MstlForecast {
    pub point_forecasts: Vec<f64>,
    pub horizon: usize,
}

/// Decompose and forecast using MSTL with given seasonal periods.
///
/// `periods` defines the seasonal lengths (e.g., `[24]` for hourly with daily seasonality).
/// Uses AutoETS as the trend model internally.
pub fn forecast_mstl(
    data: &[f64],
    horizon: usize,
    periods: &[usize],
) -> Result<MstlForecast, String> {
    if data.len() < 20 {
        return Err(format!("Need at least 20 data points, got {}", data.len()));
    }

    if periods.is_empty() {
        return Err("At least one seasonal period required".to_string());
    }

    let non_zero_periods: Vec<usize> = periods.iter().copied().filter(|&p| p > 1).collect();
    if non_zero_periods.is_empty() {
        return Err("Seasonal periods must be > 1".to_string());
    }

    // AutoETS with season_length=1 (non-seasonal) as the trend model
    let trend_ets = AutoETS::new(1, "ZZN")
        .map_err(|e| format!("Trend model init failed: {e}"))?
        .into_trend_model();

    let model = MSTLModel::new(non_zero_periods, trend_ets);

    let fitted = model
        .fit(data)
        .map_err(|e| format!("MSTL fit failed: {e}"))?;

    let forecasts = fitted
        .predict(horizon, 0.95)
        .map_err(|e| format!("MSTL predict failed: {e}"))?;

    Ok(MstlForecast {
        point_forecasts: forecasts.point.to_vec(),
        horizon,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mstl_forecast_with_seasonality() {
        let data: Vec<f64> = (0..120)
            .map(|i| {
                let trend = 100.0 + i as f64 * 0.1;
                let seasonal = 5.0 * (2.0 * std::f64::consts::PI * i as f64 / 24.0).sin();
                trend + seasonal
            })
            .collect();

        let result = forecast_mstl(&data, 12, &[24]).unwrap();
        assert_eq!(result.point_forecasts.len(), 12);
    }

    #[test]
    fn mstl_rejects_short_series() {
        let data: Vec<f64> = vec![1.0; 10];
        assert!(forecast_mstl(&data, 5, &[7]).is_err());
    }

    #[test]
    fn mstl_rejects_empty_periods() {
        let data: Vec<f64> = vec![1.0; 50];
        assert!(forecast_mstl(&data, 5, &[]).is_err());
    }
}
