//! Universal Event Logger NATS worker.
//!
//! Subscribes to `ios.>` (all subjects). For each event:
//! - Logs to the `nats_events` table (subject, symbol, source, payload_hash, timestamp)
//! - Provides a full audit trail of every event in the system.

use std::sync::Arc;

use futures::StreamExt;
use sqlx::PgPool;
use tracing::{debug, info, warn};

use super::NatsClient;

/// Run the event logger worker.
pub async fn run(nats: Arc<NatsClient>, pool: PgPool) {
    info!("Event logger started -- subscribing to ios.>");

    let mut sub = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.>"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!("Event logger subscribe failed: {e}");
            return;
        }
    };

    let mut logged: u64 = 0;

    while let Some(msg) = sub.next().await {
        let subject = msg.subject.as_str();
        let payload_hash = compute_hash(&msg.payload);

        // Try to extract symbol and source from the envelope
        let (symbol, source) = extract_envelope_fields(&msg.payload);

        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO nats_events (subject, symbol, source, payload_hash, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
        )
        .bind(subject)
        .bind(symbol.as_deref())
        .bind(source.as_deref())
        .bind(&payload_hash)
        .execute(&pool)
        .await
        {
            // Log sparingly to avoid flooding
            if logged % 1000 == 0 {
                warn!("Event logger insert failed (sample): {e}");
            }
        }

        logged += 1;

        if logged % 10000 == 0 {
            info!("Event logger: {logged} events recorded");
        }

        debug!("EVENT: {subject} hash={payload_hash}");
    }

    warn!("Event logger subscription ended (logged {logged} events)");
}

/// Compute a simple hash of the payload for deduplication.
fn compute_hash(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Extract `symbol` and `source` from the NATS envelope JSON.
/// Returns (Option<symbol>, Option<source>).
fn extract_envelope_fields(payload: &[u8]) -> (Option<String>, Option<String>) {
    let parsed: Result<serde_json::Value, _> = serde_json::from_slice(payload);
    match parsed {
        Ok(val) => {
            let symbol = val.get("symbol").and_then(|v| v.as_str()).map(String::from);
            let source = val.get("source").and_then(|v| v.as_str()).map(String::from);
            (symbol, source)
        }
        Err(_) => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let data = b"hello world";
        let h1 = compute_hash(data);
        let h2 = compute_hash(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16); // 16 hex chars
    }

    #[test]
    fn test_compute_hash_different() {
        let h1 = compute_hash(b"hello");
        let h2 = compute_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_extract_envelope_fields_valid() {
        let payload = serde_json::json!({
            "symbol": "BTCUSDT",
            "source": "streaming",
            "timestamp": "2026-04-02T00:00:00Z",
            "version": "v1",
            "data": {}
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
        let (symbol, source) = extract_envelope_fields(&bytes);
        assert_eq!(symbol.as_deref(), Some("BTCUSDT"));
        assert_eq!(source.as_deref(), Some("streaming"));
    }

    #[test]
    fn test_extract_envelope_fields_missing() {
        let payload = serde_json::json!({"foo": "bar"});
        let bytes = serde_json::to_vec(&payload).unwrap();
        let (symbol, source) = extract_envelope_fields(&bytes);
        assert!(symbol.is_none());
        assert!(source.is_none());
    }

    #[test]
    fn test_extract_envelope_fields_invalid_json() {
        let bytes = b"not json";
        let (symbol, source) = extract_envelope_fields(bytes);
        assert!(symbol.is_none());
        assert!(source.is_none());
    }

    #[test]
    fn test_compute_hash_empty() {
        let hash = compute_hash(b"");
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 16);
    }
}
