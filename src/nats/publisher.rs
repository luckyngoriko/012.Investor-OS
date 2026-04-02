//! Typed NATS publishers (Sprint N1).

use tracing::{debug, error};

use super::client::NatsClient;
use super::messages::*;

/// Typed publisher for the ML pipeline subjects.
#[derive(Clone)]
pub struct NatsPublisher {
    client: NatsClient,
}

impl NatsPublisher {
    pub fn new(client: NatsClient) -> Self {
        Self { client }
    }

    pub async fn publish_price(&self, symbol: &str, candle: PriceCandle) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "streaming", candle);
        let subject = format!("ios.prices.{symbol}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_features(&self, symbol: &str, features: FeatureSet) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "feature_service", features);
        let subject = format!("ios.features.{symbol}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_prediction(
        &self,
        symbol: &str,
        source: &str,
        prediction: ModelPredictionMsg,
    ) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, source, prediction);
        let subject = format!("ios.predict.{source}.{symbol}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_consensus(
        &self,
        symbol: &str,
        consensus: ConsensusMsg,
    ) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "consensus", consensus);
        let subject = format!("ios.consensus.{symbol}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_trade_proposal(
        &self,
        symbol: &str,
        proposal: TradeProposalMsg,
    ) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "trade_proposer", proposal);
        let subject = format!("ios.trade.proposal.{symbol}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_user_signal(
        &self,
        symbol: &str,
        user_id: &str,
        signal: UserSignalMsg,
    ) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "signal_router", signal);
        let subject = format!("ios.user.signal.{user_id}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_user_proposal(
        &self,
        symbol: &str,
        user_id: &str,
        proposal: UserProposalMsg,
    ) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "trade_executor", proposal);
        let subject = format!("ios.user.proposal.{user_id}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_trade_execute(
        &self,
        symbol: &str,
        user_id: &str,
        execute: TradeExecuteMsg,
    ) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "trade_executor", execute);
        let subject = format!("ios.trade.execute.{user_id}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_trade_fill(
        &self,
        symbol: &str,
        user_id: &str,
        fill: TradeFillMsg,
    ) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "trade_executor", fill);
        let subject = format!("ios.trade.fill.{user_id}");
        self.publish_envelope(&subject, &env).await
    }

    pub async fn publish_portfolio_update(
        &self,
        symbol: &str,
        user_id: &str,
        update: PortfolioUpdateMsg,
    ) -> Result<(), String> {
        let env = NatsEnvelope::new(symbol, "portfolio_tracker", update);
        let subject = format!("ios.portfolio.update.{user_id}");
        self.publish_envelope(&subject, &env).await
    }

    async fn publish_envelope<T: serde::Serialize>(
        &self,
        subject: &str,
        envelope: &NatsEnvelope<T>,
    ) -> Result<(), String> {
        let bytes = envelope.to_bytes().map_err(|e| format!("serialize: {e}"))?;
        self.client.publish(subject, bytes).await.map_err(|e| {
            error!("NATS publish to {subject} failed: {e}");
            format!("publish: {e}")
        })?;
        debug!(
            "Published to {subject} ({} bytes)",
            envelope.to_bytes().map(|b| b.len()).unwrap_or(0)
        );
        Ok(())
    }
}
