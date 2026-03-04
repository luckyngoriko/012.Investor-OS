//! Sprint 34: Security & Encryption Integration Tests
//!
//! Tests for HSM-backed encryption, secrets vault, audit trails, and 2FA

use chrono::Utc;
use investor_os::security::two_factor::TwoFactorType;
use investor_os::security::*;
use std::collections::HashMap;

#[test]
fn test_security_manager_integration() {
    let mut manager = SecurityManager::new();

    // Generate API key
    let user_id = uuid::Uuid::new_v4();
    let (key_id, key) = manager.generate_api_key(
        user_id,
        "Test Integration Key".to_string(),
        ClearanceLevel::Confidential,
        90,
    );

    // Validate key
    let validated = manager.validate_api_key(&key);
    assert!(validated.is_some());
    assert_eq!(validated.unwrap().id, key_id);

    // Log security event
    manager.log_event(SecurityEvent::ApiKeyCreated {
        user_id,
        key_id,
        clearance: ClearanceLevel::Confidential,
    });

    // Check audit stats
    let stats = manager.get_audit_stats();
    assert_eq!(stats.total_events, 1);
}

#[test]
fn test_clearance_level_hierarchy() {
    // Test clearance hierarchy
    assert!(ClearanceLevel::Public.meets(&ClearanceLevel::Public));
    assert!(ClearanceLevel::Internal.meets(&ClearanceLevel::Public));
    assert!(ClearanceLevel::Confidential.meets(&ClearanceLevel::Internal));
    assert!(ClearanceLevel::Restricted.meets(&ClearanceLevel::Confidential));
    assert!(ClearanceLevel::TopSecret.meets(&ClearanceLevel::Restricted));

    // Test insufficient clearance
    assert!(!ClearanceLevel::Public.meets(&ClearanceLevel::Internal));
    assert!(!ClearanceLevel::Internal.meets(&ClearanceLevel::Confidential));
}

#[test]
fn test_encryption_key_rotation() {
    let mut encryption = ApiKeyEncryption::new();

    // Initial key
    let key_id1 = encryption.current_key_id();

    // Encrypt some data
    let encrypted1 = encryption.encrypt("sensitive_data").unwrap();

    // Rotate keys
    encryption.rotate_keys().unwrap();
    let key_id2 = encryption.current_key_id();

    // Key ID should change
    assert_ne!(key_id1, key_id2);

    // Old data should still decrypt
    let decrypted = encryption.decrypt(&encrypted1).unwrap();
    assert_eq!(decrypted, "sensitive_data");

    // New encryption should use new key
    let encrypted2 = encryption.encrypt("new_data").unwrap();
    assert_eq!(encrypted2.key_id, key_id2);
    assert_ne!(encrypted2.key_id, key_id1);
}

#[test]
fn test_audit_trail_comprehensive() {
    let mut logger = AuditLogger::new();
    let user_id = uuid::Uuid::new_v4();

    // Log various events
    logger.log(SecurityEvent::LoginSuccess { user_id });
    logger.log(SecurityEvent::LoginFailed {
        user_id,
        reason: "Invalid password".to_string(),
    });
    logger.log(SecurityEvent::TwoFactorVerified { user_id });
    logger.log(SecurityEvent::AccessGranted {
        user_id,
        resource: "/api/portfolio".to_string(),
    });

    // Get events for user
    let events = logger.get_events_for_user(user_id, 1);
    assert_eq!(events.len(), 4);

    // Get failed logins
    let failed = logger.get_failed_logins(user_id, 24);
    assert_eq!(failed.len(), 1);

    // Check stats
    let stats = logger.get_stats();
    assert_eq!(stats.total_events, 4);
    assert_eq!(stats.failed_events, 1); // LoginFailed
}

#[test]
fn test_two_factor_full_flow() {
    let mut provider = TwoFactorProvider::new();
    let user_id = uuid::Uuid::new_v4();

    // Setup 2FA
    let result = provider.setup_totp(user_id);
    assert_eq!(result.backup_codes.len(), 10);
    assert!(!result.secret.is_empty());

    // Initially not enabled
    assert!(!provider.has_2fa_enabled(user_id));

    // Can't verify before enabling
    let code = TwoFactorProvider::generate_totp(&result.secret);
    assert!(!provider.verify_totp(result.method_id, &code));

    // Enable 2FA
    assert!(provider.enable_method(result.method_id));
    assert!(provider.has_2fa_enabled(user_id));

    // Now verification should work
    assert!(provider.verify_totp(result.method_id, &code));

    // Use backup code
    let backup_code = result.backup_codes[0].clone();
    assert!(provider.verify_backup_code(user_id, &backup_code));

    // Backup code should be consumed
    assert!(!provider.verify_backup_code(user_id, &backup_code));

    // Check remaining codes
    assert_eq!(provider.get_backup_codes_remaining(user_id), 9);

    // Add trusted device
    provider.add_trusted_device(user_id, "My Phone".to_string(), 30);
    let devices = provider.get_trusted_devices(user_id);
    assert_eq!(devices.len(), 1);

    // Disable 2FA
    assert!(provider.disable_method(result.method_id));
    assert!(!provider.has_2fa_enabled(user_id));
}

#[test]
fn test_security_policy_validation() {
    let manager = SecurityPolicyManager::new();

    // Valid password
    let result = manager.validate_password("MyP@ssw0rd123!");
    assert!(result.valid);
    assert!(result.score >= 70);

    // Too short
    let result = manager.validate_password("Short1!");
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("Minimum")));

    // Missing requirements
    let result = manager.validate_password("lowercase123!");
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("uppercase")));

    let result = manager.validate_password("UPPERCASE123!");
    assert!(!result.valid);
    assert!(result.errors.iter().any(|e| e.contains("lowercase")));
}

#[test]
fn test_api_key_clearance_validation() {
    let mut manager = ApiKeyManager::new();
    let user_id = uuid::Uuid::new_v4();

    // Create key with Internal clearance
    let (_, key) = manager.generate_key(
        user_id,
        "Internal Key".to_string(),
        ClearanceLevel::Internal,
        30,
    );

    // Should pass Public check
    assert!(manager.has_clearance(&key, &ClearanceLevel::Public));

    // Should pass Internal check
    assert!(manager.has_clearance(&key, &ClearanceLevel::Internal));

    // Should fail Confidential check
    assert!(!manager.has_clearance(&key, &ClearanceLevel::Confidential));
}

#[test]
fn test_audit_event_types() {
    use investor_os::security::audit::AuditSeverity;

    // Test event severities
    let login_success = SecurityEvent::LoginSuccess {
        user_id: uuid::Uuid::new_v4(),
    };
    assert_eq!(login_success.severity(), AuditSeverity::Info);

    let login_failed = SecurityEvent::LoginFailed {
        user_id: uuid::Uuid::new_v4(),
        reason: "Invalid".to_string(),
    };
    assert_eq!(login_failed.severity(), AuditSeverity::Warning);

    let access_denied = SecurityEvent::AccessDenied {
        user_id: uuid::Uuid::new_v4(),
        resource: "/admin".to_string(),
        reason: "Insufficient clearance".to_string(),
    };
    assert_eq!(access_denied.severity(), AuditSeverity::Error);

    let suspicious = SecurityEvent::SuspiciousActivity {
        user_id: uuid::Uuid::new_v4(),
        description: "Multiple failed logins".to_string(),
    };
    assert_eq!(suspicious.severity(), AuditSeverity::Critical);
}

#[test]
fn test_encryption_algorithms() {
    use investor_os::security::EncryptionAlgorithm;

    assert_eq!(EncryptionAlgorithm::Aes256Gcm.name(), "AES-256-GCM");
    assert_eq!(
        EncryptionAlgorithm::ChaCha20Poly1305.name(),
        "ChaCha20-Poly1305"
    );
    assert_eq!(EncryptionAlgorithm::HsmProtected.name(), "HSM-Protected");
}

#[test]
fn test_two_factor_types() {
    use investor_os::security::two_factor::TwoFactorType;

    assert_eq!(TwoFactorType::Totp.name(), "TOTP");
    assert_eq!(TwoFactorType::Hotp.name(), "HOTP");
    assert_eq!(TwoFactorType::WebAuthn.name(), "WebAuthn");
    assert_eq!(TwoFactorType::Sms.name(), "SMS");
    assert_eq!(TwoFactorType::Email.name(), "Email");
}

#[test]
fn test_policy_type_names() {
    use investor_os::security::policy::PolicyType;

    assert_eq!(PolicyType::Password.name(), "Password Policy");
    assert_eq!(PolicyType::Lockout.name(), "Account Lockout Policy");
    assert_eq!(PolicyType::Session.name(), "Session Management Policy");
    assert_eq!(PolicyType::ApiKey.name(), "API Key Policy");
    assert_eq!(
        PolicyType::TwoFactor.name(),
        "Two-Factor Authentication Policy"
    );
    assert_eq!(PolicyType::Encryption.name(), "Encryption Policy");
}

#[test]
fn test_security_rotation_policy() {
    let policy = KeyRotationPolicy::default();

    assert!(policy.enabled);
    assert_eq!(policy.rotation_interval_days, 90);
    assert!(policy.auto_rotate);
    assert_eq!(policy.grace_period_days, 7);
}

#[test]
fn test_api_key_expiration() {
    let mut manager = ApiKeyManager::new();
    let user_id = uuid::Uuid::new_v4();

    // Create expired key (negative days)
    let (expired_key_id, expired_key) = manager.generate_key(
        user_id,
        "Expired Key".to_string(),
        ClearanceLevel::Public,
        -1, // Expired yesterday
    );

    // Should not validate
    assert!(manager.validate_key(&expired_key).is_none());

    // Create valid key
    let (valid_key_id, valid_key) =
        manager.generate_key(user_id, "Valid Key".to_string(), ClearanceLevel::Public, 30);

    // Should validate
    let validated = manager.validate_key(&valid_key);
    assert!(validated.is_some());
    assert_eq!(validated.unwrap().id, valid_key_id);

    // Check expired keys
    let expired = manager.get_expired_keys();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].id, expired_key_id);
}

#[test]
fn test_audit_cleanup() {
    use chrono::Duration;

    let mut logger = AuditLogger::new();
    let user_id = uuid::Uuid::new_v4();

    // Add events
    logger.log(SecurityEvent::LoginSuccess { user_id });
    logger.log(SecurityEvent::Logout { user_id });

    assert_eq!(logger.event_count(), 2);

    // Cleanup old events (keep last 1 hour)
    logger.cleanup(Duration::hours(1));

    // Events should still be there (they're recent)
    assert_eq!(logger.event_count(), 2);

    // Cleanup with 0 duration (should remove all)
    logger.cleanup(Duration::seconds(0));

    // All events should be gone
    assert_eq!(logger.event_count(), 0);
}

#[test]
fn test_security_check() {
    let manager = SecurityManager::new();

    // Security check should pass
    assert!(manager.security_check());
}

#[test]
fn test_2fa_required_by_clearance() {
    let manager = SecurityManager::new();

    // Public clearance shouldn't require 2FA
    assert!(!manager.is_2fa_required(ClearanceLevel::Public));

    // Internal clearance shouldn't require 2FA
    assert!(!manager.is_2fa_required(ClearanceLevel::Internal));

    // Confidential and above should require 2FA
    assert!(manager.is_2fa_required(ClearanceLevel::Confidential));
    assert!(manager.is_2fa_required(ClearanceLevel::Restricted));
    assert!(manager.is_2fa_required(ClearanceLevel::TopSecret));
}

#[test]
fn test_api_key_usage_tracking() {
    let mut manager = ApiKeyManager::new();
    let user_id = uuid::Uuid::new_v4();

    let (_, key) = manager.generate_key(
        user_id,
        "Usage Test Key".to_string(),
        ClearanceLevel::Public,
        30,
    );

    // Check initial state
    let validated = manager.validate_key(&key).unwrap();
    assert!(validated.last_used_at.is_none());

    // Use key
    assert!(manager.use_key(&key));

    // Check last used is now set
    let validated = manager.validate_key(&key).unwrap();
    assert!(validated.last_used_at.is_some());

    // Invalid key should fail
    assert!(!manager.use_key("invalid_key"));
}

#[test]
fn test_password_strength_scoring() {
    let manager = SecurityPolicyManager::new();

    // Weak password
    let weak = manager.validate_password("a");
    assert!(!weak.valid);
    assert!(weak.score < 20);

    // Medium password
    let medium = manager.validate_password("Password123");
    assert!(!medium.valid); // Missing special char
    assert!(medium.score >= 40);
    assert!(medium.score < 70);

    // Strong password
    let strong = manager.validate_password("My$tr0ngP@ssw0rd!");
    assert!(strong.valid);
    assert!(strong.score >= 70);
}
