//! Axum HTTP handlers for the ML prediction API (Sprint 116).
//!
//! These handlers are wired into the main router in main.rs.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use super::client::MlSidecarClient;
use super::fallback;
use super::repository;
use super::types::PredictionRequest;

/// Shared state extracted by handlers. Mirrors the fields needed from AppState.
#[derive(Clone)]
pub struct PredictionState {
    pub db_pool: sqlx::PgPool,
    pub ml_sidecar: Option<Arc<MlSidecarClient>>,
}

/// POST /api/predictions/predict body.
#[derive(Debug, Deserialize)]
pub struct PredictBody {
    pub model: String,
    pub symbol: String,
    #[serde(default)]
    pub features: serde_json::Value,
    pub horizon: Option<String>,
}

/// GET /api/predictions/history query params.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub symbol: Option<String>,
    pub model: Option<String>,
    pub limit: Option<i64>,
}

/// POST /api/predictions/predict
///
/// Proxies to the ML sidecar, falls back to HRM policy if unavailable.
/// Stores the prediction in ml_predictions and returns it.
pub async fn predict_handler(
    db_pool: &sqlx::PgPool,
    ml_sidecar: &Option<Arc<MlSidecarClient>>,
    body: PredictBody,
) -> (StatusCode, Json<serde_json::Value>) {
    let request = PredictionRequest {
        model: body.model.clone(),
        symbol: body.symbol.clone(),
        features: body.features.clone(),
        horizon: body.horizon.clone(),
    };

    // Try sidecar, fall back if unavailable
    let (prediction_json, model_name, model_version, confidence, latency, is_fallback) =
        match ml_sidecar {
            Some(client) => match client.predict(&request).await {
                Ok(resp) => (
                    resp.prediction.clone(),
                    resp.model.clone(),
                    resp.model_version.clone(),
                    resp.confidence,
                    resp.latency_ms as i32,
                    false,
                ),
                Err(_) => {
                    let fb = fallback::fallback_predict(&request);
                    (
                        fb.prediction.clone(),
                        fb.model.clone(),
                        fb.model_version.clone(),
                        fb.confidence,
                        fb.latency_ms as i32,
                        true,
                    )
                }
            },
            None => {
                let fb = fallback::fallback_predict(&request);
                (
                    fb.prediction.clone(),
                    fb.model.clone(),
                    fb.model_version.clone(),
                    fb.confidence,
                    fb.latency_ms as i32,
                    true,
                )
            }
        };

    // Determine prediction_type from model name
    let prediction_type = match body.model.as_str() {
        "garch" => "volatility",
        "finbert" => "sentiment",
        "skfolio" => "allocation",
        "kronos" => "kline",
        _ => "return",
    };

    // Try to find the active model ID (may not exist yet)
    let model_id = repository::get_active_model_id(db_pool, &model_name)
        .await
        .unwrap_or(None)
        .unwrap_or_else(Uuid::nil);

    // EU AI Act Article 12: Log decision to ai_decision_logs
    let input_hash = {
        let mut hasher = Sha256::new();
        hasher.update(body.features.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let explanation = format!(
        "ML prediction by model '{}' v{} for {} (horizon: {}, fallback: {})",
        model_name,
        model_version,
        body.symbol,
        body.horizon.as_deref().unwrap_or("none"),
        is_fallback
    );

    let ai_decision_log_id = repository::log_ai_decision(
        db_pool,
        model_id,
        "ml_prediction",
        &input_hash,
        &prediction_json,
        confidence,
        &explanation,
    )
    .await
    .ok();

    // Log compliance audit trail entry
    if let Some(log_id) = ai_decision_log_id {
        let _ = repository::log_compliance_audit(
            db_pool,
            "ml_prediction",
            "ml_predictions",
            log_id,
            "create",
            &json!({
                "model": model_name,
                "symbol": body.symbol,
                "prediction_type": prediction_type,
                "confidence": confidence,
                "fallback": is_fallback,
            }),
        )
        .await;
    }

    // Store prediction with ai_decision_log_id reference
    let pred_id = repository::insert_prediction(
        db_pool,
        model_id,
        &model_name,
        &model_version,
        &body.symbol,
        prediction_type,
        body.horizon.as_deref(),
        &prediction_json,
        confidence,
        latency,
        is_fallback,
        &body.features,
        ai_decision_log_id,
    )
    .await;

    match pred_id {
        Ok(id) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": {
                    "id": id,
                    "model": model_name,
                    "model_version": model_version,
                    "symbol": body.symbol,
                    "prediction": prediction_json,
                    "confidence": confidence,
                    "latency_ms": latency,
                    "fallback": is_fallback,
                    "prediction_type": prediction_type,
                    "horizon": body.horizon,
                    "ai_decision_log_id": ai_decision_log_id,
                }
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "error": {"code": "PREDICTION_STORE_FAILED", "message": e.to_string()}
            })),
        ),
    }
}

/// GET /api/predictions/:id
pub async fn get_prediction_handler(
    db_pool: &sqlx::PgPool,
    id: Uuid,
) -> (StatusCode, Json<serde_json::Value>) {
    match repository::get_prediction(db_pool, id).await {
        Ok(Some(pred)) => (StatusCode::OK, Json(json!({"success": true, "data": pred}))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(
                json!({"success": false, "error": {"code": "NOT_FOUND", "message": "Prediction not found"}}),
            ),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({"success": false, "error": {"code": "DB_ERROR", "message": e.to_string()}}),
            ),
        ),
    }
}

/// GET /api/predictions/history
pub async fn get_history_handler(
    db_pool: &sqlx::PgPool,
    query: HistoryQuery,
) -> (StatusCode, Json<serde_json::Value>) {
    let limit = query.limit.unwrap_or(50).min(200);
    match repository::get_prediction_history(
        db_pool,
        query.symbol.as_deref(),
        query.model.as_deref(),
        limit,
    )
    .await
    {
        Ok(predictions) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": predictions,
                "count": predictions.len()
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({"success": false, "error": {"code": "DB_ERROR", "message": e.to_string()}}),
            ),
        ),
    }
}

/// GET /api/models/registry
pub async fn list_models_handler(db_pool: &sqlx::PgPool) -> (StatusCode, Json<serde_json::Value>) {
    match repository::list_models(db_pool).await {
        Ok(models) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": models,
                "count": models.len()
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({"success": false, "error": {"code": "DB_ERROR", "message": e.to_string()}}),
            ),
        ),
    }
}
