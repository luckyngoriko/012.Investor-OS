//! Feature computation service — subscribes to prices, publishes features (Sprint N2).
//!
//! Listens on `ios.prices.>` and for each new candle:
//! 1. Reads last 100 candles from DB for the symbol
//! 2. Computes technical features (RSI, MACD, Bollinger, ATR, OBV)
//! 3. Publishes to `ios.features.{symbol}`
//! 4. Stores in ml_feature_store table

use std::sync::Arc;

use futures::StreamExt;
use sqlx::PgPool;
use tracing::{debug, error, info, warn};

use super::messages::{FeatureSet, NatsEnvelope, PriceCandle};
use super::publisher::NatsPublisher;
use super::NatsClient;
use crate::prediction::features::technical::{compute_technical_features, Candle};

/// Run the feature computation service as a background task.
pub async fn run(nats: Arc<NatsClient>, publisher: Arc<NatsPublisher>, pool: PgPool) {
    info!("Feature service started — subscribing to ios.prices.>");

    let mut subscriber = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.prices.>"))
        .await
    {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to ios.prices.>: {e}");
            return;
        }
    };

    while let Some(msg) = subscriber.next().await {
        // Parse the price candle envelope
        let envelope: NatsEnvelope<PriceCandle> = match serde_json::from_slice(&msg.payload) {
            Ok(env) => env,
            Err(e) => {
                debug!("Failed to parse price message: {e}");
                continue;
            }
        };

        let symbol = &envelope.symbol;

        // Fetch last 100 candles from DB
        let rows = match sqlx::query_as::<_, (f64, f64, f64, f64, f64)>(
            "SELECT open, high, low, close, volume FROM prices WHERE symbol = $1 ORDER BY time DESC LIMIT 100",
        )
        .bind(symbol)
        .fetch_all(&pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("DB fetch for {symbol} candles failed: {e}");
                continue;
            }
        };

        if rows.len() < 30 {
            debug!(
                "{symbol}: only {} candles, need 30+ for features",
                rows.len()
            );
            continue;
        }

        // Convert to Candle structs (reverse to chronological order)
        let candles: Vec<Candle> = rows
            .iter()
            .rev()
            .map(|(o, h, l, c, v)| Candle {
                open: *o,
                high: *h,
                low: *l,
                close: *c,
                volume: *v,
            })
            .collect();

        // Compute technical features
        let features = match compute_technical_features(&candles) {
            Some(f) => f,
            None => {
                debug!("{symbol}: feature computation returned None");
                continue;
            }
        };

        let feature_set = FeatureSet {
            rsi_14: features.rsi_14,
            macd_signal: features.macd_signal,
            macd_histogram: features.macd_histogram,
            atr_14: features.atr_14,
            bb_position: features.bb_position,
            obv_trend: features.obv_trend,
            volume_change_pct: features.volume_change_pct,
            price_change_pct_5: features.price_change_pct_5,
            price_change_pct_20: features.price_change_pct_20,
        };

        // Publish to NATS
        if let Err(e) = publisher
            .publish_features(symbol, feature_set.clone())
            .await
        {
            warn!("{symbol}: NATS feature publish failed: {e}");
        }

        // Store in DB
        let features_json = serde_json::to_value(&feature_set).unwrap_or_default();
        let _ = sqlx::query(
            "INSERT INTO ml_feature_store (id, symbol, feature_set, features, source, computed_at) \
             VALUES (gen_random_uuid(), $1, 'technical', $2, 'nats_feature_service', NOW())",
        )
        .bind(symbol)
        .bind(&features_json)
        .execute(&pool)
        .await;

        debug!(
            "{symbol}: features computed and published (RSI={:.1})",
            features.rsi_14
        );
    }

    warn!("Feature service subscription ended");
}
