//! Context retrieval for the AI Chat module.
//!
//! Queries `ml_predictions`, `ai_decision_logs`, and `ml_feature_store`
//! to build a structured context string that the responder uses to
//! generate answers. Uses runtime sqlx queries (no compile-time macros).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::warn;

/// Collected context from the database for a given symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    /// Symbol that was queried (e.g. "BTCUSDT").
    pub symbol: String,

    /// Recent predictions from `ml_predictions` (last 24h).
    pub predictions: Vec<serde_json::Value>,

    /// Recent AI decision logs from `ai_decision_logs` (last 24h).
    pub decisions: Vec<serde_json::Value>,

    /// Latest features from `ml_feature_store`.
    pub features: Vec<serde_json::Value>,

    /// Sources list — table names that contributed data.
    pub sources: Vec<String>,
}

/// Result wrapper for context retrieval.
pub type ChatContextResult = Result<ChatContext, String>;

/// Retrieve context from the database for a given symbol and question.
///
/// Queries three tables in parallel:
/// 1. `ml_predictions` — recent predictions (last 24h) for the symbol
/// 2. `ai_decision_logs` — recent AI decisions (last 24h)
/// 3. `ml_feature_store` — latest computed features for the symbol
pub async fn retrieve_context(pool: &PgPool, symbol: &str, _question: &str) -> ChatContextResult {
    let predictions = fetch_recent_predictions(pool, symbol).await;
    let decisions = fetch_recent_decisions(pool, symbol).await;
    let features = fetch_latest_features(pool, symbol).await;

    let mut sources = Vec::new();
    if !predictions.is_empty() {
        sources.push("ml_predictions".to_string());
    }
    if !decisions.is_empty() {
        sources.push("ai_decision_logs".to_string());
    }
    if !features.is_empty() {
        sources.push("ml_feature_store".to_string());
    }

    Ok(ChatContext {
        symbol: symbol.to_string(),
        predictions,
        decisions,
        features,
        sources,
    })
}

/// Fetch predictions from `ml_predictions` for the last 24 hours.
async fn fetch_recent_predictions(pool: &PgPool, symbol: &str) -> Vec<serde_json::Value> {
    let rows = sqlx::query(
        r#"
        SELECT id, model_name, model_version, symbol, prediction_type,
               horizon, predicted_value, actual_value, confidence,
               is_fallback, error_metric, predicted_at
        FROM ml_predictions
        WHERE symbol = $1
          AND predicted_at >= NOW() - INTERVAL '24 hours'
        ORDER BY predicted_at DESC
        LIMIT 20
        "#,
    )
    .bind(symbol)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rows) => {
            use sqlx::Row;
            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.try_get::<uuid::Uuid, _>("id").unwrap_or_default().to_string(),
                        "model_name": r.try_get::<String, _>("model_name").unwrap_or_default(),
                        "model_version": r.try_get::<String, _>("model_version").unwrap_or_default(),
                        "prediction_type": r.try_get::<String, _>("prediction_type").unwrap_or_default(),
                        "horizon": r.try_get::<Option<String>, _>("horizon").unwrap_or(None),
                        "predicted_value": r.try_get::<serde_json::Value, _>("predicted_value").unwrap_or_default(),
                        "actual_value": r.try_get::<Option<serde_json::Value>, _>("actual_value").unwrap_or(None),
                        "confidence": r.try_get::<f64, _>("confidence").unwrap_or(0.0),
                        "is_fallback": r.try_get::<bool, _>("is_fallback").unwrap_or(false),
                        "error_metric": r.try_get::<Option<f64>, _>("error_metric").unwrap_or(None),
                        "predicted_at": r.try_get::<DateTime<Utc>, _>("predicted_at").ok().map(|t| t.to_rfc3339()),
                    })
                })
                .collect()
        }
        Err(e) => {
            warn!("Failed to fetch predictions for chat context: {e}");
            Vec::new()
        }
    }
}

/// Fetch recent AI decision logs from `ai_decision_logs` (last 24h).
///
/// Filters by decisions whose `output_data` JSONB contains the symbol.
async fn fetch_recent_decisions(pool: &PgPool, symbol: &str) -> Vec<serde_json::Value> {
    let symbol_pattern = format!("%{symbol}%");
    let rows = sqlx::query(
        r#"
        SELECT id, decision_type, confidence, explanation,
               output_data, timestamp
        FROM ai_decision_logs
        WHERE timestamp >= NOW() - INTERVAL '24 hours'
          AND (output_data::text ILIKE $1 OR explanation ILIKE $1)
        ORDER BY timestamp DESC
        LIMIT 10
        "#,
    )
    .bind(&symbol_pattern)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rows) => {
            use sqlx::Row;
            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.try_get::<uuid::Uuid, _>("id").unwrap_or_default().to_string(),
                        "decision_type": r.try_get::<String, _>("decision_type").unwrap_or_default(),
                        "confidence": r.try_get::<f64, _>("confidence").unwrap_or(0.0),
                        "explanation": r.try_get::<String, _>("explanation").unwrap_or_default(),
                        "output_data": r.try_get::<serde_json::Value, _>("output_data").unwrap_or_default(),
                        "timestamp": r.try_get::<DateTime<Utc>, _>("timestamp").ok().map(|t| t.to_rfc3339()),
                    })
                })
                .collect()
        }
        Err(e) => {
            warn!("Failed to fetch decision logs for chat context: {e}");
            Vec::new()
        }
    }
}

/// Fetch the latest features from `ml_feature_store` for a symbol.
async fn fetch_latest_features(pool: &PgPool, symbol: &str) -> Vec<serde_json::Value> {
    let rows = sqlx::query(
        r#"
        SELECT id, symbol, feature_set, features, computed_at, source
        FROM ml_feature_store
        WHERE symbol = $1
        ORDER BY computed_at DESC
        LIMIT 5
        "#,
    )
    .bind(symbol)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(rows) => {
            use sqlx::Row;
            rows.iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.try_get::<uuid::Uuid, _>("id").unwrap_or_default().to_string(),
                        "feature_set": r.try_get::<String, _>("feature_set").unwrap_or_default(),
                        "features": r.try_get::<serde_json::Value, _>("features").unwrap_or_default(),
                        "computed_at": r.try_get::<DateTime<Utc>, _>("computed_at").ok().map(|t| t.to_rfc3339()),
                        "source": r.try_get::<String, _>("source").unwrap_or_default(),
                    })
                })
                .collect()
        }
        Err(e) => {
            warn!("Failed to fetch features for chat context: {e}");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_context_serialization() {
        let ctx = ChatContext {
            symbol: "BTCUSDT".to_string(),
            predictions: vec![serde_json::json!({"model_name": "catboost", "confidence": 0.85})],
            decisions: vec![],
            features: vec![
                serde_json::json!({"feature_set": "technical", "features": {"rsi_14": 65.2}}),
            ],
            sources: vec!["ml_predictions".to_string(), "ml_feature_store".to_string()],
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("BTCUSDT"));
        assert!(json.contains("catboost"));
        assert!(json.contains("rsi_14"));
    }

    #[test]
    fn chat_context_empty() {
        let ctx = ChatContext {
            symbol: "ETHUSDT".to_string(),
            predictions: vec![],
            decisions: vec![],
            features: vec![],
            sources: vec![],
        };
        assert!(ctx.sources.is_empty());
        assert_eq!(ctx.symbol, "ETHUSDT");
    }

    #[test]
    fn chat_context_deserialization() {
        let json =
            r#"{"symbol":"BTCUSDT","predictions":[],"decisions":[],"features":[],"sources":[]}"#;
        let ctx: ChatContext = serde_json::from_str(json).unwrap();
        assert_eq!(ctx.symbol, "BTCUSDT");
        assert!(ctx.predictions.is_empty());
    }
}
