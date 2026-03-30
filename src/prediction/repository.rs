//! Database operations for the ML prediction pipeline (Sprint 116).
//!
//! Uses runtime sqlx queries (no compile-time macros) to match project convention.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::error::{PredictionError, Result};

/// Insert a prediction record into ml_predictions.
pub async fn insert_prediction(
    pool: &PgPool,
    model_id: Uuid,
    model_name: &str,
    model_version: &str,
    symbol: &str,
    prediction_type: &str,
    horizon: Option<&str>,
    predicted_value: &serde_json::Value,
    confidence: f64,
    latency_ms: i32,
    is_fallback: bool,
    features_used: &serde_json::Value,
    ai_decision_log_id: Option<Uuid>,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO ml_predictions (
            id, model_id, model_name, model_version, symbol,
            prediction_type, horizon, predicted_value, confidence,
            latency_ms, is_fallback, features_used, ai_decision_log_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
    )
    .bind(id)
    .bind(model_id)
    .bind(model_name)
    .bind(model_version)
    .bind(symbol)
    .bind(prediction_type)
    .bind(horizon)
    .bind(predicted_value)
    .bind(confidence)
    .bind(latency_ms)
    .bind(is_fallback)
    .bind(features_used)
    .bind(ai_decision_log_id)
    .execute(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    Ok(id)
}

/// Get a prediction by ID.
pub async fn get_prediction(pool: &PgPool, id: Uuid) -> Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        r#"
        SELECT id, model_name, model_version, symbol, prediction_type,
               horizon, predicted_value, actual_value, confidence,
               latency_ms, is_fallback, error_metric,
               ai_decision_log_id, predicted_at, resolved_at
        FROM ml_predictions
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    match row {
        Some(r) => {
            use sqlx::Row;
            Ok(Some(serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").unwrap_or_default(),
                "model_name": r.try_get::<String, _>("model_name").unwrap_or_default(),
                "model_version": r.try_get::<String, _>("model_version").unwrap_or_default(),
                "symbol": r.try_get::<String, _>("symbol").unwrap_or_default(),
                "prediction_type": r.try_get::<String, _>("prediction_type").unwrap_or_default(),
                "horizon": r.try_get::<Option<String>, _>("horizon").unwrap_or(None),
                "predicted_value": r.try_get::<serde_json::Value, _>("predicted_value").unwrap_or_default(),
                "actual_value": r.try_get::<Option<serde_json::Value>, _>("actual_value").unwrap_or(None),
                "confidence": r.try_get::<f64, _>("confidence").unwrap_or(0.0),
                "latency_ms": r.try_get::<Option<i32>, _>("latency_ms").unwrap_or(None),
                "is_fallback": r.try_get::<bool, _>("is_fallback").unwrap_or(false),
                "error_metric": r.try_get::<Option<f64>, _>("error_metric").unwrap_or(None),
                "predicted_at": r.try_get::<DateTime<Utc>, _>("predicted_at").ok(),
                "resolved_at": r.try_get::<Option<DateTime<Utc>>, _>("resolved_at").unwrap_or(None),
            })))
        }
        None => Ok(None),
    }
}

/// Get prediction history for a symbol, optionally filtered by model.
pub async fn get_prediction_history(
    pool: &PgPool,
    symbol: Option<&str>,
    model: Option<&str>,
    limit: i64,
) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        r#"
        SELECT id, model_name, model_version, symbol, prediction_type,
               horizon, predicted_value, actual_value, confidence,
               latency_ms, is_fallback, error_metric, predicted_at
        FROM ml_predictions
        WHERE ($1::text IS NULL OR symbol = $1)
          AND ($2::text IS NULL OR model_name = $2)
        ORDER BY predicted_at DESC
        LIMIT $3
        "#,
    )
    .bind(symbol)
    .bind(model)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    use sqlx::Row;
    let results: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").unwrap_or_default(),
                "model_name": r.try_get::<String, _>("model_name").unwrap_or_default(),
                "model_version": r.try_get::<String, _>("model_version").unwrap_or_default(),
                "symbol": r.try_get::<String, _>("symbol").unwrap_or_default(),
                "prediction_type": r.try_get::<String, _>("prediction_type").unwrap_or_default(),
                "horizon": r.try_get::<Option<String>, _>("horizon").unwrap_or(None),
                "predicted_value": r.try_get::<serde_json::Value, _>("predicted_value").unwrap_or_default(),
                "actual_value": r.try_get::<Option<serde_json::Value>, _>("actual_value").unwrap_or(None),
                "confidence": r.try_get::<f64, _>("confidence").unwrap_or(0.0),
                "is_fallback": r.try_get::<bool, _>("is_fallback").unwrap_or(false),
                "error_metric": r.try_get::<Option<f64>, _>("error_metric").unwrap_or(None),
                "predicted_at": r.try_get::<DateTime<Utc>, _>("predicted_at").ok(),
            })
        })
        .collect();

    Ok(results)
}

/// List all models from the registry.
pub async fn list_models(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        r#"
        SELECT id, model_name, model_version, model_type, tier, status,
               metrics, trained_at, created_at, updated_at
        FROM ml_model_registry
        ORDER BY model_name, model_version DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    use sqlx::Row;
    let results: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").unwrap_or_default(),
                "model_name": r.try_get::<String, _>("model_name").unwrap_or_default(),
                "model_version": r.try_get::<String, _>("model_version").unwrap_or_default(),
                "model_type": r.try_get::<String, _>("model_type").unwrap_or_default(),
                "tier": r.try_get::<i32, _>("tier").unwrap_or(1),
                "status": r.try_get::<String, _>("status").unwrap_or_default(),
                "metrics": r.try_get::<serde_json::Value, _>("metrics").unwrap_or_default(),
                "trained_at": r.try_get::<Option<DateTime<Utc>>, _>("trained_at").unwrap_or(None),
                "created_at": r.try_get::<DateTime<Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    Ok(results)
}

/// Get the active model ID for a given model name (latest active version).
pub async fn get_active_model_id(pool: &PgPool, model_name: &str) -> Result<Option<Uuid>> {
    let row = sqlx::query(
        r#"
        SELECT id FROM ml_model_registry
        WHERE model_name = $1 AND status = 'active'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(model_name)
    .fetch_optional(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    use sqlx::Row;
    Ok(row.map(|r| r.try_get::<Uuid, _>("id").unwrap_or_default()))
}

/// Log an AI decision to ai_decision_logs for EU AI Act Article 12 compliance.
///
/// Returns the decision log UUID for cross-reference in ml_predictions.
pub async fn log_ai_decision(
    pool: &PgPool,
    system_id: Uuid,
    decision_type: &str,
    input_data_hash: &str,
    output_data: &serde_json::Value,
    confidence: f64,
    explanation: &str,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO ai_decision_logs (
            id, system_id, decision_type, input_data_hash,
            output_data, confidence, explanation
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(id)
    .bind(system_id)
    .bind(decision_type)
    .bind(input_data_hash)
    .bind(output_data)
    .bind(confidence)
    .bind(explanation)
    .execute(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    Ok(id)
}

/// Log to compliance_audit_trail for general audit purposes.
pub async fn log_compliance_audit(
    pool: &PgPool,
    event_type: &str,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    new_values: &serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO compliance_audit_trail (
            event_type, entity_type, entity_id, action, new_values
        ) VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(event_type)
    .bind(entity_type)
    .bind(entity_id)
    .bind(action)
    .bind(new_values)
    .execute(pool)
    .await
    .map_err(|e| PredictionError::Database(e.to_string()))?;

    Ok(())
}
