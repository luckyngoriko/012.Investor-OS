//! HRM NATS worker (Sprint N4).
//!
//! Subscribes to sentiment + GARCH + features subjects.
//! Maps inputs to HRM's signal format and publishes conviction/regime.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::StreamExt;
use tracing::{debug, error, info, warn};

use super::messages::{ModelPredictionMsg, NatsEnvelope};
use super::publisher::NatsPublisher;
use super::NatsClient;
use crate::hrm::{HRMConfig, InferenceEngine, HRM};

/// Per-symbol state for HRM input aggregation.
struct SymbolState {
    sentiment: Option<f64>,
    volatility: Option<f64>,
    rsi: Option<f64>,
    last_updated: DateTime<Utc>,
}

impl SymbolState {
    fn new() -> Self {
        Self {
            sentiment: None,
            volatility: None,
            rsi: None,
            last_updated: Utc::now(),
        }
    }

    fn is_ready(&self) -> bool {
        self.sentiment.is_some() && self.volatility.is_some()
    }

    fn is_stale(&self, max_age_secs: i64) -> bool {
        (Utc::now() - self.last_updated).num_seconds() > max_age_secs
    }
}

/// Run the HRM worker.
pub async fn run(nats: Arc<NatsClient>, publisher: Arc<NatsPublisher>) {
    info!("HRM worker started — subscribing to sentiment + garch + features");

    // Initialize HRM inference engine
    let config = HRMConfig::default();
    let hrm = match HRM::new(&config) {
        Ok(h) => h,
        Err(e) => {
            error!("HRM init failed: {e}");
            return;
        }
    };

    let mut states: HashMap<String, SymbolState> = HashMap::new();
    let staleness_secs: i64 = std::env::var("CONSENSUS_STALENESS_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    // Subscribe to all relevant subjects
    let mut sub = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.predict.garch.*"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("HRM subscribe failed: {e}");
            return;
        }
    };

    let mut sent_sub = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.sentiment.*"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            error!("HRM sentiment subscribe failed: {e}");
            return;
        }
    };

    loop {
        tokio::select! {
            Some(msg) = sub.next() => {
                // GARCH volatility
                if let Ok(env) = serde_json::from_slice::<NatsEnvelope<serde_json::Value>>(&msg.payload) {
                    let symbol = env.symbol.clone();
                    let state = states.entry(symbol.clone()).or_insert_with(SymbolState::new);
                    state.volatility = env.data.get("volatility").and_then(|v| v.as_f64());
                    state.last_updated = Utc::now();

                    if state.is_ready() && !state.is_stale(staleness_secs) {
                        run_hrm_inference(&hrm, &symbol, state, &publisher).await;
                    }
                }
            }
            Some(msg) = sent_sub.next() => {
                // Sentiment
                if let Ok(env) = serde_json::from_slice::<NatsEnvelope<serde_json::Value>>(&msg.payload) {
                    let symbol = env.symbol.clone();
                    let state = states.entry(symbol.clone()).or_insert_with(SymbolState::new);
                    state.sentiment = env.data.get("avg_sentiment_score").and_then(|v| v.as_f64());
                    state.last_updated = Utc::now();

                    if state.is_ready() && !state.is_stale(staleness_secs) {
                        run_hrm_inference(&hrm, &symbol, state, &publisher).await;
                    }
                }
            }
        }
    }
}

async fn run_hrm_inference(
    hrm: &HRM,
    symbol: &str,
    state: &SymbolState,
    publisher: &NatsPublisher,
) {
    let sentiment = state.sentiment.unwrap_or(0.0);
    let volatility = state.volatility.unwrap_or(0.02);

    // Map to HRM 6-signal input
    let signals: Vec<f32> = vec![
        0.5,                                   // PEGY (neutral placeholder)
        ((sentiment + 1.0) / 2.0) as f32,      // insider sentiment proxy (0-1)
        ((sentiment + 1.0) / 2.0) as f32,      // social sentiment (0-1)
        (volatility * 100.0).min(80.0) as f32, // VIX proxy from GARCH vol
        0.0,                                   // regime (determined by HRM)
        0.5,                                   // time of day (midday)
    ];

    match hrm.infer(&signals) {
        Ok(result) => {
            let action = if result.conviction > 0.6 {
                "long"
            } else if result.conviction < 0.4 {
                "short"
            } else {
                "neutral"
            };

            let predicted_return = (result.conviction as f64 - 0.5) * 0.04; // map conviction to return estimate

            let msg = ModelPredictionMsg {
                model_name: "hrm".to_string(),
                model_version: "v1".to_string(),
                prediction_type: "return".to_string(),
                direction: Some(action.to_string()),
                predicted_return: Some(predicted_return),
                volatility: Some(volatility),
                var_95: None,
                confidence: result.confidence as f64,
                latency_ms: 0,
                forecasts: None,
            };

            if let Err(e) = publisher.publish_prediction(symbol, "hrm", msg).await {
                warn!("HRM publish failed: {e}");
            }

            debug!(
                "HRM {symbol}: conviction={:.3} conf={:.3} action={action}",
                result.conviction, result.confidence
            );
        }
        Err(e) => {
            warn!("HRM inference failed for {symbol}: {e}");
        }
    }
}
