//! Sprint 136: End-to-End ML Pipeline Integration Test
//!
//! Tests the full flow: features → predictions → consensus → signal score.
//! Runs without a database or sidecar (uses fallback path).

use investor_os::prediction::consensus::{
    compute_consensus, consensus_to_quality_score, ModelPrediction,
};
use investor_os::prediction::fallback::fallback_predict;
use investor_os::prediction::features::technical::{compute_technical_features, Candle};
use investor_os::prediction::forecasting::ets::forecast_ets;
use investor_os::prediction::types::PredictionRequest;
use investor_os::signals::{QualityScore, TickerSignals};

/// Generate sample candles for testing.
fn sample_candles(n: usize) -> Vec<Candle> {
    (0..n)
        .map(|i| {
            let base = 45000.0 + i as f64 * 50.0;
            let noise = ((i * 7 % 13) as f64 - 6.0) * 30.0;
            Candle {
                open: base - 20.0 + noise,
                high: base + 150.0 + noise.abs(),
                low: base - 100.0 - noise.abs(),
                close: base + noise,
                volume: 100.0 + (i * 11 % 17) as f64 * 10.0,
            }
        })
        .collect()
}

#[test]
fn e2e_pipeline_features_to_signal() {
    // STEP 1: Compute technical features from candle data
    let candles = sample_candles(100);
    let features = compute_technical_features(&candles).unwrap();

    assert!(features.rsi_14 >= 0.0 && features.rsi_14 <= 100.0);
    assert!(features.atr_14 > 0.0);
    assert!(features.bb_upper > features.bb_lower);

    // STEP 2: Use ETS to forecast next 12 candle closes
    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let ets_forecast = forecast_ets(&closes, 12).unwrap();
    assert_eq!(ets_forecast.point_forecasts.len(), 12);

    // STEP 3: Generate fallback prediction (sidecar unavailable)
    let request = PredictionRequest {
        model: "catboost".to_string(),
        symbol: "BTCUSDT".to_string(),
        features: serde_json::json!({
            "rsi_14": features.rsi_14,
            "macd_signal": features.macd_signal,
            "atr_14": features.atr_14,
            "bb_position": features.bb_position,
        }),
        horizon: Some("1d".to_string()),
    };
    let fallback = fallback_predict(&request);
    assert!(fallback.fallback);
    assert_eq!(fallback.model, "hrm_fallback");

    // STEP 4: Create consensus from multiple model predictions
    let predictions = vec![
        ModelPrediction {
            model_name: "catboost".to_string(),
            predicted_return: 0.015,
            confidence: 0.75,
            accuracy_weight: 1.0,
        },
        ModelPrediction {
            model_name: "ets".to_string(),
            predicted_return: ets_forecast.point_forecasts[0] / closes.last().unwrap() - 1.0,
            confidence: 0.6,
            accuracy_weight: 0.8,
        },
        ModelPrediction {
            model_name: "garch".to_string(),
            predicted_return: 0.005,
            confidence: 0.7,
            accuracy_weight: 0.9,
        },
    ];

    let consensus = compute_consensus(&predictions).unwrap();
    assert_eq!(consensus.n_models, 3);
    assert!(consensus.confidence > 0.0);
    assert_eq!(consensus.model_contributions.len(), 3);

    // STEP 5: Convert consensus to signal quality score
    let ml_score = consensus_to_quality_score(&consensus);
    assert!(ml_score <= 100);

    // STEP 6: Build TickerSignals with ML prediction score
    let signals = TickerSignals {
        rsi_14: features.rsi_14,
        macd_signal: features.macd_signal,
        ml_prediction_score: QualityScore(ml_score),
        ..TickerSignals::default()
    };

    assert!(signals.ml_prediction_score.0 <= 100);
    assert!(signals.rsi_14 >= 0.0);
}

#[test]
fn e2e_pipeline_all_models_agree() {
    let predictions = vec![
        ModelPrediction {
            model_name: "catboost".to_string(),
            predicted_return: 0.02,
            confidence: 0.85,
            accuracy_weight: 1.0,
        },
        ModelPrediction {
            model_name: "kronos".to_string(),
            predicted_return: 0.025,
            confidence: 0.8,
            accuracy_weight: 0.9,
        },
        ModelPrediction {
            model_name: "chronos".to_string(),
            predicted_return: 0.018,
            confidence: 0.7,
            accuracy_weight: 0.85,
        },
    ];

    let consensus = compute_consensus(&predictions).unwrap();
    assert_eq!(consensus.agreement, 1.0); // all agree on Long
    assert!(consensus.confidence > 0.7); // high agreement boosts confidence

    let score = consensus_to_quality_score(&consensus);
    assert!(
        score > 50,
        "Strong long consensus should give >50, got {score}"
    );
}

#[test]
fn e2e_pipeline_graceful_degradation() {
    // When no sidecar and no models — system still works with defaults
    let signals = TickerSignals::default();
    assert_eq!(signals.ml_prediction_score.0, 50); // neutral default
    assert_eq!(signals.composite_quality.0, 50);
}
