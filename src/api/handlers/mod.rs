//! API Handlers

pub mod analytics;
pub mod broker;
pub mod rag;

use axum::{
    http::{header::CONTENT_TYPE, HeaderName, StatusCode},
    Json,
};
use serde::Serialize;

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
    let _ = crate::monitoring::metrics::init_metrics();
    crate::monitoring::metrics::record_api_request("GET", "/api/health", 200);
    Json(ApiResponse::success(serde_json::json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
    })))
}

pub async fn readiness() -> Json<ApiResponse<serde_json::Value>> {
    let _ = crate::monitoring::metrics::init_metrics();
    crate::monitoring::metrics::record_api_request("GET", "/api/ready", 200);
    Json(ApiResponse::success(serde_json::json!({
        "status": "ready",
    })))
}

// ==================== Metrics Endpoint (Prometheus) ====================

pub async fn metrics() -> (StatusCode, [(HeaderName, &'static str); 1], String) {
    let _ = crate::monitoring::metrics::init_metrics();
    let content_type = [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")];

    match crate::monitoring::metrics::encode_metrics() {
        Ok(payload) => {
            crate::monitoring::metrics::record_api_request("GET", "/metrics", 200);
            (StatusCode::OK, content_type, payload)
        }
        Err(err) => {
            crate::monitoring::metrics::record_api_request("GET", "/metrics", 500);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                content_type,
                format!("# metrics_export_error {}\n", err),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_endpoint_exports_prometheus_payload() {
        let _ = crate::monitoring::metrics::init_metrics();
        crate::monitoring::metrics::record_api_request("GET", "/api/health", 200);
        let (status, headers, body) = metrics().await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[0].1, "text/plain; version=0.0.4; charset=utf-8");
        assert!(
            body.contains("# HELP"),
            "prometheus text format should contain HELP descriptors"
        );
        assert!(
            body.contains("api_requests_total"),
            "export should include API request metrics family"
        );
    }
}
