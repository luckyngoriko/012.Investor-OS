//! API Handlers
//!
//! HTTP API endpoints for Investor OS

pub mod handlers;

use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use crate::rag::RagService;

/// Application state shared across handlers
pub struct AppState {
    pub rag_service: RagService,
    pub database_url: String,
}

/// Create the API router
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health endpoints
        .route("/api/health", get(handlers::health))
        .route("/api/ready", get(handlers::readiness))
        
        // RAG endpoints (S5-D9)
        .route("/api/rag/search", post(handlers::rag::rag_search))
        .route("/api/rag/summarize", post(handlers::rag::rag_summarize))
        .route("/api/rag/journal-search", post(handlers::rag::rag_journal_search))
        .route("/api/rag/sec-filings", post(handlers::rag::process_sec_filing))
        .route("/api/rag/earnings", post(handlers::rag::process_earnings_call))
        
        // Broker endpoints (S6)
        .route("/api/broker/orders", post(handlers::broker::place_order))
        .route("/api/broker/orders/:id", delete(handlers::broker::cancel_order))
        .route("/api/broker/positions", get(handlers::broker::get_positions))
        .route("/api/broker/positions/:ticker", get(handlers::broker::get_position))
        .route("/api/broker/account", get(handlers::broker::get_account))
        .route("/api/broker/kill-switch", post(handlers::broker::trigger_kill_switch))
        
        // Analytics endpoints (S7)
        .route("/api/analytics/backtest", post(handlers::analytics::run_backtest))
        .route("/api/analytics/risk", get(handlers::analytics::get_risk_metrics))
        .route("/api/analytics/attribution", get(handlers::analytics::get_attribution))
        .route("/api/analytics/predict", post(handlers::analytics::get_ml_prediction))
        .route("/api/analytics/anomalies", get(handlers::analytics::check_anomalies))
        
        .with_state(state)
}
