//! Security Policy Module
//!
//! Configurable security policies for password, lockout, and session management

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Security policy manager
#[derive(Debug)]
pub struct SecurityPolicyManager {
    policies: HashMap<PolicyType, SecurityPolicy>,
    user_policies: HashMap<Uuid, Vec<PolicyOverride>>,
}

/// Security policy
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub policy_type: PolicyType,
    pub enabled: bool,
    pub settings: PolicySettings,
    pub updated_at: DateTime<Utc>,
}

/// Policy types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyType {
    Password,
    Lockout,
    Session,
    ApiKey,
    TwoFactor,
    Encryption,
}

impl PolicyType {
    pub fn name(&self) -> &'static str {
        match self {
            PolicyType::Password => "Password Policy",
            PolicyType::Lockout => "Account Lockout Policy",
            PolicyType::Session => "Session Management Policy",
            PolicyType::ApiKey => "API Key Policy",
            PolicyType::TwoFactor => "Two-Factor Authentication Policy",
            PolicyType::Encryption => "Encryption Policy",
        }
    }
}

/// Policy settings
#[derive(Debug, Clone)]
pub enum PolicySettings {
    Password(PasswordPolicy),
    Lockout(LockoutPolicy),
    Session(SessionPolicy),
    ApiKey(ApiKeyPolicy),
    TwoFactor(TwoFactorPolicy),
    Encryption(EncryptionPolicy),
}

/// Password policy
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_symbols: bool,
    pub history_count: usize,
    pub max_age_days: i64,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 12,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: true,
            history_count: 5,
            max_age_days: 90,
        }
    }
}

/// Lockout policy
#[derive(Debug, Clone)]
pub struct LockoutPolicy {
    pub max_failed_attempts: u32,
    pub lockout_duration_minutes: i64,
    pub reset_after_minutes: i64,
    pub progressive_lockout: bool,
    pub notify_admin: bool,
}

impl Default for LockoutPolicy {
    fn default() -> Self {
        Self {
            max_failed_attempts: 5,
            lockout_duration_minutes: 30,
            reset_after_minutes: 15,
            progressive_lockout: true,
            notify_admin: true,
        }
    }
}

/// Session policy
#[derive(Debug, Clone)]
pub struct SessionPolicy {
    pub max_session_duration_minutes: i64,
    pub idle_timeout_minutes: i64,
    pub absolute_timeout_minutes: i64,
    pub max_concurrent_sessions: u32,
    pub enforce_ip_binding: bool,
    pub enforce_user_agent: bool,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            max_session_duration_minutes: 480,
            idle_timeout_minutes: 30,
            absolute_timeout_minutes: 480,
            max_concurrent_sessions: 3,
            enforce_ip_binding: false,
            enforce_user_agent: true,
        }
    }
}

/// API key policy
#[derive(Debug, Clone)]
pub struct ApiKeyPolicy {
    pub max_keys_per_user: u32,
    pub default_expiry_days: i64,
    pub max_expiry_days: i64,
    pub require_approval: bool,
    pub allowed_clearance_levels: Vec<crate::security::ClearanceLevel>,
}

impl Default for ApiKeyPolicy {
    fn default() -> Self {
        Self {
            max_keys_per_user: 5,
            default_expiry_days: 90,
            max_expiry_days: 365,
            require_approval: true,
            allowed_clearance_levels: vec![
                crate::security::ClearanceLevel::Public,
                crate::security::ClearanceLevel::Internal,
                crate::security::ClearanceLevel::Confidential,
            ],
        }
    }
}

/// 2FA policy
#[derive(Debug, Clone)]
pub struct TwoFactorPolicy {
    pub required_for_clearance: Vec<crate::security::ClearanceLevel>,
    pub grace_period_days: i64,
    pub allowed_methods: Vec<crate::security::two_factor::TwoFactorType>,
    pub trusted_device_days: i64,
    pub require_backup_codes: bool,
}

impl Default for TwoFactorPolicy {
    fn default() -> Self {
        use crate::security::two_factor::TwoFactorType;
        
        Self {
            required_for_clearance: vec![
                crate::security::ClearanceLevel::Confidential,
                crate::security::ClearanceLevel::Restricted,
                crate::security::ClearanceLevel::TopSecret,
            ],
            grace_period_days: 7,
            allowed_methods: vec![
                TwoFactorType::Totp,
                TwoFactorType::Email,
            ],
            trusted_device_days: 30,
            require_backup_codes: true,
        }
    }
}

/// Encryption policy
#[derive(Debug, Clone)]
pub struct EncryptionPolicy {
    pub algorithm: EncryptionAlgorithm,
    pub key_rotation_days: i64,
    pub data_at_rest: bool,
    pub data_in_transit: bool,
    pub require_hsm: bool,
}

impl Default for EncryptionPolicy {
    fn default() -> Self {
        Self {
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_rotation_days: 90,
            data_at_rest: true,
            data_in_transit: true,
            require_hsm: false,
        }
    }
}

/// Encryption algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}

/// Policy override for specific user
#[derive(Debug, Clone)]
pub struct PolicyOverride {
    pub policy_type: PolicyType,
    pub setting: String,
    pub value: PolicyValue,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Policy value
#[derive(Debug, Clone)]
pub enum PolicyValue {
    Boolean(bool),
    Integer(i64),
    String(String),
    Duration(i64),
}

impl SecurityPolicyManager {
    /// Create new policy manager with defaults
    pub fn new() -> Self {
        let mut policies = HashMap::new();
        
        policies.insert(
            PolicyType::Password,
            SecurityPolicy {
                policy_type: PolicyType::Password,
                enabled: true,
                settings: PolicySettings::Password(PasswordPolicy::default()),
                updated_at: Utc::now(),
            },
        );
        
        policies.insert(
            PolicyType::Lockout,
            SecurityPolicy {
                policy_type: PolicyType::Lockout,
                enabled: true,
                settings: PolicySettings::Lockout(LockoutPolicy::default()),
                updated_at: Utc::now(),
            },
        );
        
        policies.insert(
            PolicyType::Session,
            SecurityPolicy {
                policy_type: PolicyType::Session,
                enabled: true,
                settings: PolicySettings::Session(SessionPolicy::default()),
                updated_at: Utc::now(),
            },
        );
        
        policies.insert(
            PolicyType::ApiKey,
            SecurityPolicy {
                policy_type: PolicyType::ApiKey,
                enabled: true,
                settings: PolicySettings::ApiKey(ApiKeyPolicy::default()),
                updated_at: Utc::now(),
            },
        );
        
        policies.insert(
            PolicyType::TwoFactor,
            SecurityPolicy {
                policy_type: PolicyType::TwoFactor,
                enabled: true,
                settings: PolicySettings::TwoFactor(TwoFactorPolicy::default()),
                updated_at: Utc::now(),
            },
        );
        
        policies.insert(
            PolicyType::Encryption,
            SecurityPolicy {
                policy_type: PolicyType::Encryption,
                enabled: true,
                settings: PolicySettings::Encryption(EncryptionPolicy::default()),
                updated_at: Utc::now(),
            },
        );
        
        Self {
            policies,
            user_policies: HashMap::new(),
        }
    }
    
    /// Get policy
    pub fn get_policy(&self, policy_type: PolicyType) -> Option<&SecurityPolicy> {
        self.policies.get(&policy_type)
    }
    
    /// Update password policy
    pub fn set_password_policy(&mut self, policy: PasswordPolicy) {
        self.policies.insert(
            PolicyType::Password,
            SecurityPolicy {
                policy_type: PolicyType::Password,
                enabled: true,
                settings: PolicySettings::Password(policy),
                updated_at: Utc::now(),
            },
        );
    }
    
    /// Update lockout policy
    pub fn set_lockout_policy(&mut self, policy: LockoutPolicy) {
        self.policies.insert(
            PolicyType::Lockout,
            SecurityPolicy {
                policy_type: PolicyType::Lockout,
                enabled: true,
                settings: PolicySettings::Lockout(policy),
                updated_at: Utc::now(),
            },
        );
    }
    
    /// Validate password against policy
    pub fn validate_password(&self, password: &str) -> PasswordValidationResult {
        let Some(policy) = self.get_policy(PolicyType::Password) else {
            return PasswordValidationResult::default();
        };
        
        let PolicySettings::Password(ref password_policy) = policy.settings else {
            return PasswordValidationResult::default();
        };
        
        let mut errors = Vec::new();
        
        if password.len() < password_policy.min_length {
            errors.push(format!("Minimum length is {}", password_policy.min_length));
        }
        
        if password.len() > password_policy.max_length {
            errors.push(format!("Maximum length is {}", password_policy.max_length));
        }
        
        if password_policy.require_uppercase && !password.chars().any(|c| c.is_ascii_uppercase()) {
            errors.push("Must contain uppercase letter".to_string());
        }
        
        if password_policy.require_lowercase && !password.chars().any(|c| c.is_ascii_lowercase()) {
            errors.push("Must contain lowercase letter".to_string());
        }
        
        if password_policy.require_numbers && !password.chars().any(|c| c.is_ascii_digit()) {
            errors.push("Must contain number".to_string());
        }
        
        if password_policy.require_symbols && !password.chars().any(|c| !c.is_alphanumeric()) {
            errors.push("Must contain special character".to_string());
        }
        
        PasswordValidationResult {
            valid: errors.is_empty(),
            errors,
            score: self.calculate_password_score(password),
        }
    }
    
    /// Get lockout settings
    pub fn get_lockout_settings(&self) -> Option<&LockoutPolicy> {
        self.get_policy(PolicyType::Lockout)
            .and_then(|p| match &p.settings {
                PolicySettings::Lockout(l) => Some(l),
                _ => None,
            })
    }
    
    /// Get session settings
    pub fn get_session_settings(&self) -> Option<&SessionPolicy> {
        self.get_policy(PolicyType::Session)
            .and_then(|p| match &p.settings {
                PolicySettings::Session(s) => Some(s),
                _ => None,
            })
    }
    
    /// Check if 2FA is required for clearance level
    pub fn is_2fa_required(&self, clearance: crate::security::ClearanceLevel) -> bool {
        self.get_policy(PolicyType::TwoFactor)
            .and_then(|p| match &p.settings {
                PolicySettings::TwoFactor(t) => {
                    Some(t.required_for_clearance.contains(&clearance))
                }
                _ => None,
            })
            .unwrap_or(false)
    }
    
    /// Get API key settings
    pub fn get_api_key_settings(&self) -> Option<&ApiKeyPolicy> {
        self.get_policy(PolicyType::ApiKey)
            .and_then(|p| match &p.settings {
                PolicySettings::ApiKey(a) => Some(a),
                _ => None,
            })
    }
    
    /// Enable/disable policy
    pub fn set_policy_enabled(&mut self, policy_type: PolicyType, enabled: bool) {
        if let Some(policy) = self.policies.get_mut(&policy_type) {
            policy.enabled = enabled;
            policy.updated_at = Utc::now();
        }
    }
    
    /// Add user policy override
    pub fn add_user_override(&mut self, user_id: Uuid, override_: PolicyOverride) {
        self.user_policies
            .entry(user_id)
            .or_default()
            .push(override_);
    }
    
    /// Get active overrides for user
    pub fn get_user_overrides(&self, user_id: Uuid) -> Vec<&PolicyOverride> {
        self.user_policies.get(&user_id)
            .map(|overrides| {
                overrides.iter()
                    .filter(|o| o.expires_at.map(|e| e > Utc::now()).unwrap_or(true))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Calculate password strength score (0-100)
    fn calculate_password_score(&self, password: &str) -> u32 {
        let mut score: u32 = 0;
        
        // Length
        score += (password.len() as u32).min(30) * 2;
        
        // Character variety
        if password.chars().any(|c| c.is_ascii_lowercase()) { score += 10; }
        if password.chars().any(|c| c.is_ascii_uppercase()) { score += 10; }
        if password.chars().any(|c| c.is_ascii_digit()) { score += 10; }
        if password.chars().any(|c| !c.is_alphanumeric()) { score += 15; }
        
        score.min(100)
    }
}

impl Default for SecurityPolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Password validation result
#[derive(Debug, Clone)]
pub struct PasswordValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub score: u32,
}

impl Default for PasswordValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            score: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = SecurityPolicyManager::new();
        assert!(manager.get_policy(PolicyType::Password).is_some());
        assert!(manager.get_policy(PolicyType::Lockout).is_some());
    }

    #[test]
    fn test_validate_password() {
        let manager = SecurityPolicyManager::new();
        
        // Too short
        let result = manager.validate_password("Short1!");
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        
        // Valid password
        let result = manager.validate_password("MyP@ssw0rd123!");
        assert!(result.valid);
        assert!(result.score > 50);
    }

    #[test]
    fn test_password_requirements() {
        let manager = SecurityPolicyManager::new();
        
        // Missing uppercase
        let result = manager.validate_password("myp@ssw0rd123!");
        assert!(!result.valid);
        
        // Missing lowercase
        let result = manager.validate_password("MYP@SSW0RD123!");
        assert!(!result.valid);
        
        // Missing number
        let result = manager.validate_password("MyP@ssword!!!");
        assert!(!result.valid);
        
        // Missing symbol
        let result = manager.validate_password("MyPassword1234");
        assert!(!result.valid);
    }

    #[test]
    fn test_get_lockout_settings() {
        let manager = SecurityPolicyManager::new();
        let settings = manager.get_lockout_settings().unwrap();
        
        assert_eq!(settings.max_failed_attempts, 5);
        assert_eq!(settings.lockout_duration_minutes, 30);
    }

    #[test]
    fn test_is_2fa_required() {
        let manager = SecurityPolicyManager::new();
        
        // Confidential requires 2FA
        assert!(manager.is_2fa_required(crate::security::ClearanceLevel::Confidential));
        assert!(manager.is_2fa_required(crate::security::ClearanceLevel::Restricted));
        
        // Public doesn't require 2FA by default
        assert!(!manager.is_2fa_required(crate::security::ClearanceLevel::Public));
    }

    #[test]
    fn test_set_policy_enabled() {
        let mut manager = SecurityPolicyManager::new();
        
        let policy = manager.get_policy(PolicyType::Password).unwrap();
        assert!(policy.enabled);
        
        manager.set_policy_enabled(PolicyType::Password, false);
        
        let policy = manager.get_policy(PolicyType::Password).unwrap();
        assert!(!policy.enabled);
    }

    #[test]
    fn test_user_overrides() {
        let mut manager = SecurityPolicyManager::new();
        let user_id = Uuid::new_v4();
        
        let override_ = PolicyOverride {
            policy_type: PolicyType::Password,
            setting: "min_length".to_string(),
            value: PolicyValue::Integer(8),
            expires_at: None,
        };
        
        manager.add_user_override(user_id, override_);
        
        let overrides = manager.get_user_overrides(user_id);
        assert_eq!(overrides.len(), 1);
    }

    #[test]
    fn test_policy_type_names() {
        assert_eq!(PolicyType::Password.name(), "Password Policy");
        assert_eq!(PolicyType::Lockout.name(), "Account Lockout Policy");
        assert_eq!(PolicyType::Session.name(), "Session Management Policy");
    }

    #[test]
    fn test_password_score() {
        let manager = SecurityPolicyManager::new();
        
        // Weak password
        let result = manager.validate_password("a");
        assert!(result.score < 20);
        
        // Strong password
        let result = manager.validate_password("My$uper$tr0ngP@ssw0rd!!!");
        assert!(result.score > 80);
    }
}
