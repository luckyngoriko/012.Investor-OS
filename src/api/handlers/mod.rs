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
