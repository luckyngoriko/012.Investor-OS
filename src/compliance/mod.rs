//! Compliance Module
//!
//! Sprint 52: EU Compliance Integration (behind `eu_compliance` feature)
//! Wave 4 Task 23: KYC/AML Verification via Sumsub (always enabled)
//!
//! This module provides:
//! - KYC/AML identity verification via Sumsub (always available)
//! - EU AI Act compliance tracking via AI-OS.NET integration (feature-gated)
//! - GDPR "Right to be forgotten" and "Data portability" (feature-gated)
//! - Audit logging for AI decisions (Article 12 requirement) (feature-gated)
//! - DLP (Data Loss Prevention) via AI-OS-PG (feature-gated)

// KYC/AML — always available
pub mod kyc;

// EU-specific compliance modules — behind feature gate
#[cfg(feature = "eu_compliance")]
pub mod ai_os_net;
#[cfg(feature = "eu_compliance")]
pub mod audit;
#[cfg(feature = "eu_compliance")]
pub mod dlp_integration;
#[cfg(feature = "eu_compliance")]
pub mod gdpr;
#[cfg(feature = "eu_compliance")]
pub mod policy_integration;
#[cfg(feature = "eu_compliance")]
pub mod types;

#[cfg(feature = "eu_compliance")]
pub use ai_os_net::ComplianceClient;
#[cfg(feature = "eu_compliance")]
pub use audit::AuditLogger;
#[cfg(feature = "eu_compliance")]
pub use gdpr::GdprManager;
#[cfg(feature = "eu_compliance")]
pub use types::*;

#[cfg(feature = "eu_compliance")]
use axum::{
    routing::{delete, get, post},
    Router,
};

#[cfg(feature = "eu_compliance")]
use std::sync::Arc;

/// Create compliance routes for the API (EU compliance only)
#[cfg(feature = "eu_compliance")]
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
#[cfg(feature = "eu_compliance")]
pub fn ai_os_net_url() -> String {
    std::env::var("AI_OS_NET_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Get AI-OS-PG URL from environment
#[cfg(feature = "eu_compliance")]
pub fn ai_os_pg_url() -> String {
    std::env::var("AI_OS_PG_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "eu_compliance")]
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
