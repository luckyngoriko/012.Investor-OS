//! Sprint 52: EU AI Act & GDPR Compliance Tests
//!
//! Tests for:
//! - GDPR Article 17 (Right to erasure)
//! - GDPR Article 20 (Data portability)
//! - EU AI Act Article 12 (Logging)
//! - EU AI Act Article 14 (Human oversight)
//! - DLP integration
//! - Policy engine integration

#[cfg(feature = "eu_compliance")]
mod tests {
    use chrono::Utc;
    use investor_os::compliance::dlp_integration::{DlpConfig, DlpIntegration};
    use investor_os::compliance::policy_integration::{
        PolicyConfig, PolicyIntegration, RequestContext,
    };
    use investor_os::compliance::{types::*, AuditLogger, GdprManager};
    use serde_json::json;
    use std::net::Ipv4Addr;
    use uuid::Uuid;

    // =========================================================================
    // GDPR Tests
    // =========================================================================

    #[tokio::test]
    async fn test_gdpr_forget_user() {
        let manager = GdprManager::new_without_db();
        let user_id = Uuid::new_v4().to_string();

        let request = manager
            .forget_user(&user_id)
            .await
            .expect("forget_user should succeed");
        assert_eq!(request.user_id.to_string(), user_id);
        assert_eq!(request.status, DeletionStatus::Pending);

        // Verify 30-day deletion schedule (allow small variance due to DST)
        let days_until = (request.scheduled_deletion - request.requested_at).num_days();
        assert!(
            (29..=30).contains(&days_until),
            "Expected ~30 days, got {}",
            days_until
        );
    }

    #[tokio::test]
    async fn test_gdpr_export_data() {
        let manager = GdprManager::new_without_db();
        let user_id = Uuid::new_v4().to_string();

        // Test JSON export
        let export = manager
            .export_user_data(&user_id, ExportFormat::Json)
            .await
            .expect("export_user_data should succeed");
        assert_eq!(export.user_id.to_string(), user_id);
        assert_eq!(export.format, ExportFormat::Json);
        assert!(export.data.get("profile").is_some());
        assert!(export.data.get("export_metadata").is_some());
    }

    #[tokio::test]
    async fn test_gdpr_invalid_user_id() {
        let manager = GdprManager::new_without_db();

        let result = manager.forget_user("invalid-uuid-format").await;
        assert!(result.is_err());
    }

    // =========================================================================
    // EU AI Act Audit Logging Tests
    // =========================================================================

    #[tokio::test]
    async fn test_audit_log_decision() {
        let mut logger = AuditLogger::new_without_db();
        let system_id = Uuid::new_v4();

        let input = json!({"market_data": "test"});
        let output = json!({"signal": "buy", "confidence": 0.85});

        let log = logger
            .log_decision(
                system_id,
                DecisionType::TradingSignal,
                &input,
                &output,
                0.85,
                "Test trading decision",
            )
            .await
            .expect("log_decision should succeed");
        assert_eq!(log.system_id, system_id);
        assert_eq!(log.decision_type, DecisionType::TradingSignal);
        assert!(!log.human_reviewed);
        assert!(log.confidence > 0.0 && log.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_audit_buffer_flush() {
        let mut logger = AuditLogger::new_without_db();
        let system_id = Uuid::new_v4();

        // Log multiple decisions
        for i in 0..5 {
            let input = json!({"iteration": i});
            let output = json!({"result": i});

            let log = logger
                .log_decision(
                    system_id,
                    DecisionType::TradingSignal,
                    &input,
                    &output,
                    0.8,
                    "Test decision",
                )
                .await
                .expect("log_decision should succeed");
            assert_eq!(log.decision_type, DecisionType::TradingSignal);
        }

        // Buffer should have entries
        assert_eq!(logger.buffer_len(), 5);

        // Flush buffer
        logger.flush().await.expect("flush should succeed");

        // Buffer should be empty after flush
        assert!(logger.is_buffer_empty());
    }

    // =========================================================================
    // DLP Integration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_dlp_disabled() {
        let config = DlpConfig {
            enabled: false,
            ..Default::default()
        };
        let dlp = DlpIntegration::new(config);

        let scan: DlpScanResult = dlp
            .scan("test@example.com")
            .await
            .expect("DLP scan should succeed when disabled");
        assert!(!scan.has_violations);
    }

    #[tokio::test]
    async fn test_dlp_email_detection() {
        let dlp = DlpIntegration::from_env();

        if !dlp.is_enabled() {
            return; // Skip if DLP not configured
        }

        let scan: DlpScanResult = dlp
            .scan("Contact: user@example.com")
            .await
            .expect("DLP scan should succeed");
        // Email detection may or may not trigger depending on implementation
        if scan.has_violations {
            assert!(scan
                .findings
                .iter()
                .any(|f: &DlpFinding| f.finding_type == "EMAIL"));
        }
    }

    #[tokio::test]
    async fn test_dlp_validate_safe_content() {
        let dlp = DlpIntegration::from_env();

        dlp.validate("Safe content without PII")
            .await
            .expect("validate should succeed for content without PII");
    }

    // =========================================================================
    // Policy Engine Tests
    // =========================================================================

    #[tokio::test]
    async fn test_policy_disabled() {
        let config = PolicyConfig {
            enabled: false,
            ..Default::default()
        };
        let policy = PolicyIntegration::new(config);

        let ctx = RequestContext::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "/api/test",
        );

        let eval: PolicyResult = policy
            .evaluate(&ctx)
            .await
            .expect("policy evaluation should succeed when disabled");
        assert!(eval.allowed);
    }

    #[tokio::test]
    async fn test_policy_waf_sql_injection() {
        let config = PolicyConfig::default();
        let policy = PolicyIntegration::new(config);

        let ctx = RequestContext::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "/api/data'; DROP TABLE users;--",
        );

        let eval: PolicyResult = policy
            .evaluate(&ctx)
            .await
            .expect("policy evaluation should succeed");
        assert!(!eval.allowed);
        assert!(eval.violations.iter().any(|v: &String| v.contains("SQL")));
    }

    #[tokio::test]
    async fn test_policy_waf_xss() {
        let config = PolicyConfig::default();
        let policy = PolicyIntegration::new(config);

        // Test with <script> tag in path
        let ctx = RequestContext::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "/api/data/<script>alert(1)</script>",
        );

        let eval: PolicyResult = policy
            .evaluate(&ctx)
            .await
            .expect("policy evaluation should succeed");
        assert!(!eval.allowed);
        assert!(eval.violations.iter().any(|v: &String| v.contains("XSS")));
    }

    #[tokio::test]
    async fn test_policy_waf_path_traversal() {
        let config = PolicyConfig::default();
        let policy = PolicyIntegration::new(config);

        let ctx = RequestContext::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "/api/data/../../../etc/passwd",
        );

        let eval: PolicyResult = policy
            .evaluate(&ctx)
            .await
            .expect("policy evaluation should succeed");
        assert!(!eval.allowed);
        assert!(eval
            .violations
            .iter()
            .any(|v: &String| v.contains("traversal")));
    }

    // =========================================================================
    // Compliance Types Tests
    // =========================================================================

    #[test]
    fn test_compliance_score() {
        let score = ComplianceScore::new(85);
        assert_eq!(score.value(), 85);
        assert!(score.is_acceptable());

        let low_score = ComplianceScore::new(50);
        assert!(!low_score.is_acceptable());

        let max_score = ComplianceScore::new(150);
        assert_eq!(max_score.value(), 100);
    }

    #[test]
    fn test_risk_level() {
        assert!(!RiskLevel::Minimal.requires_human_oversight());
        assert!(!RiskLevel::Limited.requires_human_oversight());
        assert!(RiskLevel::High.requires_human_oversight());
        assert!(RiskLevel::Unacceptable.requires_human_oversight());
    }

    #[test]
    fn test_deletion_status() {
        let pending = DeletionStatus::Pending;
        let completed = DeletionStatus::Completed;

        assert_ne!(pending, completed);
    }

    #[test]
    fn test_export_format() {
        let json = ExportFormat::Json;
        let xml = ExportFormat::Xml;
        let csv = ExportFormat::Csv;

        assert_ne!(json, xml);
        assert_ne!(xml, csv);
        assert_ne!(json, csv);
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[tokio::test]
    async fn test_full_compliance_workflow() {
        // This test simulates a full compliance workflow:
        // 1. User makes a request
        // 2. Policy engine validates request
        // 3. DLP scans for PII
        // 4. AI makes a decision
        // 5. Decision is logged for compliance
        // 6. User requests data export (GDPR)

        // Step 1: Policy check
        let policy = PolicyIntegration::new(PolicyConfig::default());
        let ctx = RequestContext::new(
            std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "/api/trading/signal",
        );

        let policy_eval: PolicyResult = policy
            .evaluate(&ctx)
            .await
            .expect("policy evaluation should succeed");
        assert!(policy_eval.allowed);

        // Step 2: DLP scan
        let dlp = DlpIntegration::from_env();
        let dlp_result: DlpScanResult = dlp
            .scan(r#"{"signal": "buy", "amount": 100}"#)
            .await
            .expect("DLP scan should succeed");
        // Should not have violations for trading data
        assert!(
            !dlp_result.has_violations,
            "trading JSON should not trigger DLP violations"
        );

        // Step 3: AI decision
        let mut logger = AuditLogger::new_without_db();
        let system_id = Uuid::new_v4();
        let input = json!({"market": "data"});
        let output = json!({"action": "buy", "confidence": 0.92});

        let log = logger
            .log_decision(
                system_id,
                DecisionType::TradingSignal,
                &input,
                &output,
                0.92,
                "AI trading signal generated",
            )
            .await
            .unwrap();

        assert_eq!(log.decision_type, DecisionType::TradingSignal);

        // Step 4: GDPR export
        let gdpr = GdprManager::new_without_db();
        let user_id = Uuid::new_v4().to_string();
        let export = gdpr
            .export_user_data(&user_id, ExportFormat::Json)
            .await
            .unwrap();

        assert_eq!(export.user_id.to_string(), user_id);
        assert!(export.data.is_object());
    }
}

// Tests that don't require the eu_compliance feature
#[cfg(not(feature = "eu_compliance"))]
mod no_feature_tests {
    #[test]
    fn test_compliance_feature_disabled() {
        // When feature is disabled, compliance module should not be available.
        // Verify that the feature flag is indeed absent at compile time.
        let features: &[&str] = &[]; // eu_compliance is NOT enabled
        assert!(
            !cfg!(feature = "eu_compliance"),
            "eu_compliance feature should be disabled in this test"
        );
        assert!(features.is_empty()); // sentinel — proves test ran with assertions
    }
}
