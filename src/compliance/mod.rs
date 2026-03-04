//! EU AI Act & GDPR Compliance Module
//!
//! Sprint 52: EU Compliance Integration
//!
//! This module provides:
//! - EU AI Act compliance tracking via AI-OS.NET integration
//! - GDPR "Right to be forgotten" and "Data portability"
//! - Audit logging for AI decisions (Article 12 requirement)
//! - DLP (Data Loss Prevention) via AI-OS-PG
//!
//! # Example
//!
//! ```rust
//! use crate::compliance::{ComplianceClient, GdprManager};
//!
//! async fn example() {
//!     // Initialize compliance client
//!     let compliance = ComplianceClient::new(
//!         "http://ai-os-net:8080",
//!         "investor-os-hrm"
//!     ).await.unwrap();
//!
//!     // Log AI decision (required by EU AI Act)
//!     compliance.log_ai_decision(&trading_decision).await.unwrap();
//!
//!     // Check compliance score
//!     let score = compliance.get_compliance_score().await.unwrap();
//!     assert!(score >= 70, "Compliance score too low!");
//! }
//! ```

pub mod ai_os_net;
pub mod audit;
pub mod dlp_integration;
pub mod gdpr;
pub mod policy_integration;
pub mod types;

pub use ai_os_net::ComplianceClient;
pub use audit::AuditLogger;
pub use gdpr::GdprManager;
pub use types::*;

use axum::{
    routing::{delete, get, post},
    Router,
};

use std::sync::Arc;

/// Create compliance routes for the API
pub fn routes() -> Router<Arc<crate::api::AppState>> {
    Router::new()
        // GDPR endpoints
        .route("/gdpr/forget-me", delete(gdpr::handlers::forget_me))
        .route("/gdpr/export-data", get(gdpr::handlers::export_data))
        .route(
            "/gdpr/data-portability",
            get(gdpr::handlers::data_portability),
        )
        // Compliance endpoints
        .route("/score", get(ai_os_net::handlers::get_compliance_score))
        .route("/report", get(ai_os_net::handlers::get_compliance_report))
        .route("/audit-log", post(audit::handlers::log_event))
        .route("/audit-log", get(audit::handlers::query_events))
}

/// Feature flag for EU compliance
pub const EU_COMPLIANCE_FEATURE: &str = "eu_compliance";

/// Check if EU compliance is enabled
pub fn is_eu_compliance_enabled() -> bool {
    std::env::var("EU_COMPLIANCE_ENABLED")
        .map(|v| v.parse().unwrap_or(false))
        .unwrap_or(false)
}

/// Get AI-OS.NET URL from environment
pub fn ai_os_net_url() -> String {
    std::env::var("AI_OS_NET_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Get AI-OS-PG URL from environment
pub fn ai_os_pg_url() -> String {
    std::env::var("AI_OS_PG_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_config() {
        // Test default URLs
        std::env::remove_var("AI_OS_NET_URL");
        std::env::remove_var("AI_OS_PG_URL");

        assert_eq!(ai_os_net_url(), "http://localhost:8080");
        assert_eq!(ai_os_pg_url(), "http://localhost:3000");
    }

    #[test]
    fn test_eu_compliance_feature_flag() {
        std::env::set_var("EU_COMPLIANCE_ENABLED", "true");
        assert!(is_eu_compliance_enabled());

        std::env::set_var("EU_COMPLIANCE_ENABLED", "false");
        assert!(!is_eu_compliance_enabled());
    }
}
