//! JetStream stream provisioning (Sprint N1).

use async_nats::jetstream::{self, stream};
use tracing::{info, warn};

/// Stream definitions for the ML pipeline.
const STREAMS: &[(&str, &str, i64, i64)] = &[
    // (name, subjects, max_age_secs, max_bytes)
    ("PRICES", "ios.prices.>", 30 * 86400, 1_073_741_824), // 30 days, 1GB
    ("FEATURES", "ios.features.>", 7 * 86400, 536_870_912), // 7 days, 512MB
    ("PREDICTIONS", "ios.predict.>", 30 * 86400, 1_073_741_824), // 30 days, 1GB
    ("CONSENSUS", "ios.consensus.>", 30 * 86400, 536_870_912), // 30 days, 512MB
    ("TRADES", "ios.trade.>", 90 * 86400, 536_870_912),    // 90 days, 512MB
    ("USER_SIGNALS", "ios.user.>", 30 * 86400, 536_870_912), // 30 days, 512MB — signals, proposals, fills
    ("PORTFOLIO", "ios.portfolio.>", 90 * 86400, 536_870_912), // 90 days, 512MB — updates, rebalances
];

/// Create all JetStream streams idempotently.
pub async fn ensure_streams(js: &jetstream::Context) -> Result<(), Box<dyn std::error::Error>> {
    for (name, subjects, max_age_secs, max_bytes) in STREAMS {
        let config = stream::Config {
            name: name.to_string(),
            subjects: vec![subjects.to_string()],
            retention: stream::RetentionPolicy::Limits,
            max_age: std::time::Duration::from_secs(*max_age_secs as u64),
            max_bytes: *max_bytes,
            storage: stream::StorageType::File,
            num_replicas: 1,
            ..Default::default()
        };

        match js.get_or_create_stream(config).await {
            Ok(stream) => {
                let info = stream.cached_info();
                info!(
                    "JetStream stream {name}: {} messages, {} bytes",
                    info.state.messages, info.state.bytes
                );
            }
            Err(e) => {
                warn!("Failed to create stream {name}: {e}");
                return Err(e.into());
            }
        }
    }

    info!("All {} JetStream streams ready", STREAMS.len());
    Ok(())
}
