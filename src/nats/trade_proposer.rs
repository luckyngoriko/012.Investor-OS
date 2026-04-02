//! Trade Proposer NATS worker (Sprint N4).
//!
//! Subscribes to ios.consensus.{symbol} — when confidence exceeds
//! threshold, generates a trade proposal.

use std::sync::Arc;

use futures::StreamExt;
use tracing::{debug, info, warn};

use super::messages::{ConsensusMsg, NatsEnvelope, TradeProposalMsg};
use super::publisher::NatsPublisher;
use super::NatsClient;

/// Run the trade proposer.
pub async fn run(nats: Arc<NatsClient>, publisher: Arc<NatsPublisher>) {
    let min_confidence: f64 = std::env::var("TRADE_PROPOSAL_MIN_CONFIDENCE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.65);

    info!(
        "Trade proposer started — min confidence: {:.0}%",
        min_confidence * 100.0
    );

    let mut sub = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.consensus.*"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!("Trade proposer subscribe failed: {e}");
            return;
        }
    };

    while let Some(msg) = sub.next().await {
        let env: NatsEnvelope<ConsensusMsg> = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let symbol = &env.symbol;
        let consensus = &env.data;

        if consensus.confidence < min_confidence {
            debug!(
                "{symbol}: confidence {:.0}% < {:.0}% threshold — no proposal",
                consensus.confidence * 100.0,
                min_confidence * 100.0
            );
            continue;
        }

        // Size based on confidence: higher confidence = larger position
        let size_pct = (consensus.confidence - min_confidence) / (1.0 - min_confidence) * 5.0; // max 5%
        let size_pct = size_pct.clamp(0.5, 5.0);

        let proposal = TradeProposalMsg {
            direction: consensus.direction.clone(),
            confidence: consensus.confidence,
            suggested_size_pct: (size_pct * 100.0).round() / 100.0,
            stop_loss_pct: 2.0,
            take_profit_pct: 4.0,
            reason: format!(
                "Consensus {} with {:.0}% confidence ({} models, {:.0}% agreement)",
                consensus.direction,
                consensus.confidence * 100.0,
                consensus.n_models,
                consensus.agreement * 100.0,
            ),
        };

        if let Err(e) = publisher.publish_trade_proposal(symbol, proposal).await {
            warn!("Trade proposal publish failed: {e}");
        }

        info!(
            "TRADE PROPOSAL {symbol}: {} {:.1}% size, conf={:.0}%, reason: {}",
            consensus.direction,
            size_pct,
            consensus.confidence * 100.0,
            consensus.n_models,
        );
    }
}
