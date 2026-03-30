//! Feature store DB operations (Sprint 118).
//!
//! Writes computed features to ml_feature_store and reads latest features for a symbol.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::prediction::error::{PredictionError, Result};

/// Store computed features for a symbol.
pub async fn store_features(
    pool: &PgPool,
    symbol: &str,
    feature_set: &str,
    features: &serde_json::Value,
    source: &str,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO ml_feature_store (id, symbol, feature_set, features, source)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id)
    .bind(symbol)
    .bind(feature_set)
    .bind(features)
    .bind(source)
    .execute(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    Ok(id)
}

/// Get the latest features for a symbol and feature set.
pub async fn get_latest_features(
    pool: &PgPool,
    symbol: &str,
    feature_set: &str,
) -> Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        r#"
        SELECT features, computed_at
        FROM ml_feature_store
        WHERE symbol = $1 AND feature_set = $2
        ORDER BY computed_at DESC
        LIMIT 1
        "#,
    )
    .bind(symbol)
    .bind(feature_set)
    .fetch_optional(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    use sqlx::Row;
    match row {
        Some(r) => {
            let features = r
                .try_get::<serde_json::Value, _>("features")
                .unwrap_or_default();
            let computed_at = r.try_get::<DateTime<Utc>, _>("computed_at").ok();
            Ok(Some(serde_json::json!({
                "features": features,
                "computed_at": computed_at,
            })))
        }
        None => Ok(None),
    }
}
