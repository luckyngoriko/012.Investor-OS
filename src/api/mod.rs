//! API Handlers
//!
//! HTTP API endpoints for Investor OS

pub mod handlers;

use axum::{
    routing::{get, post},
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
        .route("/api/rag/search", post(handlers::rag_search))
        .route("/api/rag/summarize", post(handlers::rag_summarize))
        .route("/api/rag/journal-search", post(handlers::rag_journal_search))
        
        // SEC filings endpoints
        .route("/api/rag/sec-filings", post(handlers::process_sec_filing))
        
        // Earnings endpoints  
        .route("/api/rag/earnings", post(handlers::process_earnings_call))
        
        .with_state(state)
}
