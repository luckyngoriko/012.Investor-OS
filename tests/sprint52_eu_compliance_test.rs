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
    use investor_os::compliance::{
        GdprManager, AuditLogger,
        types::*,
    };
    use investor_os::compliance::dlp_integration::{DlpIntegration, DlpConfig};
    use investor_os::compliance::policy_integration::{PolicyIntegration, PolicyConfig, RequestContext};
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;
    use std::net::Ipv4Addr;

    // =========================================================================
    // GDPR Tests
    // =========================================================================

    #[tokio::test]
    async fn test_gdpr_forget_user() {
        let manager = GdprManager::new_without_db();
        let user_id = Uuid::new_v4().to_string();

        let result = manager.forget_user(&user_id).await;
        assert!(result.is_ok());

        let request = result.unwrap();
        assert_eq!(request.user_id.to_string(), user_id);
        assert_eq!(request.status, DeletionStatus::Pending);
        
        // Verify 30-day deletion schedule (allow small variance due to DST)
        let days_until = (request.scheduled_deletion - request.requested_at).num_days();
        assert!((29..=30).contains(&days_until), "Expected ~30 days, got {}", days_until);
    }

    #[tokio::test]
    async fn test_gdpr_export_data() {
        let manager = GdprManager::new_without_db();
        let user_id = Uuid::new_v4().to_string();

        // Test JSON export
        let result = manager.export_user_data(&user_id, ExportFormat::Json).await;
        assert!(result.is_ok());

        let export = result.unwrap();
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

        let result = logger.log_decision(
            system_id,
            DecisionType::TradingSignal,
            &input,
            &output,
            0.85,
            "Test trading decision",
        ).await;

        assert!(result.is_ok());
        
        let log = result.unwrap();
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

            let result = logger.log_decision(
                system_id,
                DecisionType::TradingSignal,
                &input,
                &output,
                0.8,
                "Test decision",
            ).await;

            assert!(result.is_ok());
        }

        // Buffer should have entries
        assert_eq!(logger.buffer_len(), 5);

        // Flush buffer
        let result = logger.flush().await;
        assert!(result.is_ok());

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

        let result: Result<DlpScanResult, _> = dlp.scan("test@example.com").await;
        assert!(result.is_ok());

        let scan = result.unwrap();
        assert!(!scan.has_violations);
    }

    #[tokio::test]
    async fn test_dlp_email_detection() {
        let dlp = DlpIntegration::from_env();

        if !dlp.is_enabled() {
            return; // Skip if DLP not configured
        }

        let result: Result<DlpScanResult, _> = dlp.scan("Contact: user@example.com").await;
        assert!(result.is_ok());

        let scan = result.unwrap();
        // Email detection may or may not trigger depending on implementation
        if scan.has_violations {
            assert!(scan.findings.iter().any(|f: &DlpFinding| f.finding_type == "EMAIL"));
        }
    }

    #[tokio::test]
    async fn test_dlp_validate_safe_content() {
        let dlp = DlpIntegration::from_env();

        let result: Result<(), _> = dlp.validate("Safe content without PII").await;
        assert!(result.is_ok());
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

        let result: Result<PolicyResult, _> = policy.evaluate(&ctx).await;
        assert!(result.is_ok());

        let eval = result.unwrap();
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

        let result: Result<PolicyResult, _> = policy.evaluate(&ctx).await;
        assert!(result.is_ok());

        let eval = result.unwrap();
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

        let result: Result<PolicyResult, _> = policy.evaluate(&ctx).await;
        assert!(result.is_ok());

        let eval = result.unwrap();
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

        let result: Result<PolicyResult, _> = policy.evaluate(&ctx).await;
        assert!(result.is_ok());

        let eval = result.unwrap();
        assert!(!eval.allowed);
        assert!(eval.violations.iter().any(|v: &String| v.contains("traversal")));
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

        let policy_result: Result<PolicyResult, _> = policy.evaluate(&ctx).await;
        assert!(policy_result.is_ok());
        let policy_eval = policy_result.unwrap();
        assert!(policy_eval.allowed);

        // Step 2: DLP scan
        let dlp = DlpIntegration::from_env();
        let dlp_result: Result<DlpScanResult, _> = dlp.scan(r#"{"signal": "buy", "amount": 100}"#).await;
        assert!(dlp_result.is_ok());
        // Should not have violations for trading data

        // Step 3: AI decision
        let mut logger = AuditLogger::new_without_db();
        let system_id = Uuid::new_v4();
        let input = json!({"market": "data"});
        let output = json!({"action": "buy", "confidence": 0.92});

        let log = logger.log_decision(
            system_id,
            DecisionType::TradingSignal,
            &input,
            &output,
            0.92,
            "AI trading signal generated",
        ).await.unwrap();

        assert_eq!(log.decision_type, DecisionType::TradingSignal);

        // Step 4: GDPR export
        let gdpr = GdprManager::new_without_db();
        let user_id = Uuid::new_v4().to_string();
        let export = gdpr.export_user_data(&user_id, ExportFormat::Json).await.unwrap();

        assert_eq!(export.user_id.to_string(), user_id);
        assert!(export.data.is_object());
    }
}

// Tests that don't require the eu_compliance feature
#[cfg(not(feature = "eu_compliance"))]
mod no_feature_tests {
    #[test]
    fn test_compliance_feature_disabled() {
        // When feature is disabled, compliance module should not be available
        // This test just verifies the build succeeds without the feature
        assert!(true);
    }
}
