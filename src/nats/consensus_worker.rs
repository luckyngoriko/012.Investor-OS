//! Consensus NATS worker (Sprint N4).
//!
//! Subscribes to ALL ios.predict.*.{symbol} subjects.
//! Collects per-symbol predictions and runs weighted consensus.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use futures::StreamExt;
use tracing::{debug, info, warn};

use super::messages::{ConsensusMsg, ModelPredictionMsg, NatsEnvelope};
use super::publisher::NatsPublisher;
use super::NatsClient;
use crate::prediction::consensus::{compute_consensus, ModelPrediction};

/// Per-symbol latest predictions from each model.
struct SymbolPredictions {
    predictions: HashMap<String, (ModelPrediction, DateTime<Utc>)>,
}

impl SymbolPredictions {
    fn new() -> Self {
        Self {
            predictions: HashMap::new(),
        }
    }

    fn update(&mut self, model: &str, pred: ModelPrediction) {
        self.predictions
            .insert(model.to_string(), (pred, Utc::now()));
    }

    fn get_fresh(&self, max_age_secs: i64) -> Vec<ModelPrediction> {
        let now = Utc::now();
        self.predictions
            .values()
            .filter(|(_, ts)| (now - *ts).num_seconds() <= max_age_secs)
            .map(|(p, _)| p.clone())
            .collect()
    }
}

/// Run the consensus worker.
pub async fn run(nats: Arc<NatsClient>, publisher: Arc<NatsPublisher>) {
    info!("Consensus worker started — subscribing to ios.predict.*.>");

    let mut sub = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.predict.*.*"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!("Consensus subscribe failed: {e}");
            return;
        }
    };

    let min_models: usize = std::env::var("CONSENSUS_MIN_MODELS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);

    let staleness_secs: i64 = std::env::var("CONSENSUS_STALENESS_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    let mut all_predictions: HashMap<String, SymbolPredictions> = HashMap::new();

    while let Some(msg) = sub.next().await {
        let env: NatsEnvelope<ModelPredictionMsg> = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let symbol = env.symbol.clone();
        let model_name = env.data.model_name.clone();

        let model_pred = ModelPrediction {
            model_name: model_name.clone(),
            predicted_return: env.data.predicted_return.unwrap_or(0.0),
            confidence: env.data.confidence,
            accuracy_weight: 1.0, // TODO: load from ml_model_registry metrics
        };

        let sym_preds = all_predictions
            .entry(symbol.clone())
            .or_insert_with(SymbolPredictions::new);
        sym_preds.update(&model_name, model_pred);

        // Check if we have enough fresh predictions
        let fresh = sym_preds.get_fresh(staleness_secs);
        if fresh.len() < min_models {
            continue;
        }

        // Run consensus
        if let Some(consensus) = compute_consensus(&fresh) {
            let direction = match consensus.direction {
                crate::prediction::consensus::ConsensusDirection::Long => "long",
                crate::prediction::consensus::ConsensusDirection::Short => "short",
                crate::prediction::consensus::ConsensusDirection::Neutral => "neutral",
            };

            let msg = ConsensusMsg {
                direction: direction.to_string(),
                predicted_return: consensus.predicted_return,
                confidence: consensus.confidence,
                agreement: consensus.agreement,
                n_models: consensus.n_models,
                model_contributions: consensus
                    .model_contributions
                    .iter()
                    .map(|c| format!("{}:{:.0}%", c.model_name, c.weight * 100.0))
                    .collect(),
            };

            if let Err(e) = publisher.publish_consensus(&symbol, msg).await {
                warn!("Consensus publish failed: {e}");
            }

            debug!(
                "Consensus {symbol}: {direction} return={:.4}% conf={:.2} agreement={:.2} ({} models)",
                consensus.predicted_return * 100.0,
                consensus.confidence,
                consensus.agreement,
                consensus.n_models,
            );
        }
    }
}
