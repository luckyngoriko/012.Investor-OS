//! API Handlers

pub mod analytics;
pub mod broker;
pub mod rag;

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;

use crate::api::AppState;

/// Standard API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Default limit for pagination
pub fn default_limit() -> usize {
    10
}

// ==================== Health Endpoints ====================

pub async fn health() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

pub async fn readiness() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "ready",
    })))
}

// ==================== Metrics Endpoint (Prometheus) ====================

pub async fn metrics() -> (StatusCode, String) {
    // Simple Prometheus-compatible metrics output
    // In production, use a proper metrics crate like `metrics` or `prometheus`
    let metrics = format!(r#"
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{{method="GET",status="200"}} 1000

# HELP portfolio_value Current portfolio value
# TYPE portfolio_value gauge
portfolio_value {}

# HELP active_positions_count Number of active positions
# TYPE active_positions_count gauge
active_positions_count 5

# HELP killswitch_triggered Whether kill switch is activated
# TYPE killswitch_triggered gauge
killswitch_triggered 0

# HELP up Whether the service is up
# TYPE up gauge
up 1
"#, 100000.0);
    
    (StatusCode::OK, metrics)
}
