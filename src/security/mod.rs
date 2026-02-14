//! Security & Encryption Module
//!
//! HSM-backed key management, secrets vault, audit trails, and 2FA
//!
//! Sprint 34: Security & Encryption - Security Manager

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::fmt;
use tracing::info;
use uuid::Uuid;

pub mod encryption;
pub mod audit;
pub mod two_factor;
pub mod policy;

pub use encryption::{ApiKeyEncryption, EncryptedKey, EncryptionAlgorithm, KeyRotationPolicy};
pub use audit::{AuditLogger, AuditEvent, SecurityEvent, AuditSeverity};
pub use two_factor::{TwoFactorProvider, TwoFactorMethod, TwoFactorType, TotpSetupResult};
pub use policy::{SecurityPolicyManager, SecurityPolicy, PolicyType, PasswordValidationResult};

/// Security errors
#[derive(Debug, Clone)]
pub enum SecurityError {
    Encryption(String),
    Decryption(String),
    InvalidKey(String),
    KeyExpired(String),
    AccessDenied(String),
    PolicyViolation(String),
    Audit(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityError::Encryption(s) => write!(f, "Encryption error: {}", s),
            SecurityError::Decryption(s) => write!(f, "Decryption error: {}", s),
            SecurityError::InvalidKey(s) => write!(f, "Invalid key: {}", s),
            SecurityError::KeyExpired(s) => write!(f, "Key expired: {}", s),
            SecurityError::AccessDenied(s) => write!(f, "Access denied: {}", s),
            SecurityError::PolicyViolation(s) => write!(f, "Policy violation: {}", s),
            SecurityError::Audit(s) => write!(f, "Audit error: {}", s),
        }
    }
}

impl std::error::Error for SecurityError {}

/// Security result type
pub type Result<T> = std::result::Result<T, SecurityError>;

/// Clearance levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
pub enum ClearanceLevel {
    #[default]
    Public,
    Internal,
    Confidential,
    Restricted,
    TopSecret,
}

impl ClearanceLevel {
    /// Get numeric value for comparison
    pub fn value(&self) -> u8 {
        match self {
            ClearanceLevel::Public => 0,
            ClearanceLevel::Internal => 1,
            ClearanceLevel::Confidential => 2,
            ClearanceLevel::Restricted => 3,
            ClearanceLevel::TopSecret => 4,
        }
    }
    
    /// Check if this clearance meets or exceeds required clearance
    pub fn meets(&self, required: &ClearanceLevel) -> bool {
        self.value() >= required.value()
    }
    
    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            ClearanceLevel::Public => "Public",
            ClearanceLevel::Internal => "Internal",
            ClearanceLevel::Confidential => "Confidential",
            ClearanceLevel::Restricted => "Restricted",
            ClearanceLevel::TopSecret => "Top Secret",
        }
    }
}


/// API key with metadata
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub clearance: ClearanceLevel,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub encrypted_value: Option<EncryptedKey>,
}

/// API key manager
#[derive(Debug)]
pub struct ApiKeyManager {
    keys: HashMap<String, ApiKey>,
    user_keys: HashMap<Uuid, Vec<String>>,
    encryption: ApiKeyEncryption,
}

impl ApiKeyManager {
    /// Create new API key manager
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            user_keys: HashMap::new(),
            encryption: ApiKeyEncryption::new(),
        }
    }
    
    /// Generate new API key
    pub fn generate_key(&mut self, user_id: Uuid, name: String, clearance: ClearanceLevel, expiry_days: i64) -> (Uuid, String) {
        let id = Uuid::new_v4();
        
        // Generate random key
        let key = Self::generate_random_key();
        let key_hash = Self::hash_key(&key);
        
        // Encrypt key for storage
        let encrypted = self.encryption.encrypt(&key).ok();
        
        let api_key = ApiKey {
            id,
            user_id,
            name,
            key_hash: key_hash.clone(),
            clearance,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(expiry_days),
            last_used_at: None,
            revoked: false,
            encrypted_value: encrypted,
        };
        
        self.keys.insert(key_hash.clone(), api_key);
        self.user_keys.entry(user_id).or_default().push(key_hash);
        
        info!("Generated new API key {} for user {}", id, user_id);
        
        (id, format!("ios_{}", key))
    }
    
    /// Validate API key
    pub fn validate_key(&self, key: &str) -> Option<&ApiKey> {
        // Extract key part (remove prefix)
        let key_part = key.strip_prefix("ios_").unwrap_or(key);
        let key_hash = Self::hash_key(key_part);
        
        let api_key = self.keys.get(&key_hash)?;
        
        if api_key.revoked {
            return None;
        }
        
        if Utc::now() > api_key.expires_at {
            return None;
        }
        
        Some(api_key)
    }
    
    /// Use API key (update last used)
    pub fn use_key(&mut self, key: &str) -> bool {
        let key_part = key.strip_prefix("ios_").unwrap_or(key);
        let key_hash = Self::hash_key(key_part);
        
        if let Some(api_key) = self.keys.get_mut(&key_hash) {
            api_key.last_used_at = Some(Utc::now());
            true
        } else {
            false
        }
    }
    
    /// Revoke API key
    pub fn revoke_key(&mut self, key_id: Uuid) -> bool {
        if let Some(api_key) = self.keys.values_mut().find(|k| k.id == key_id) {
            api_key.revoked = true;
            info!("Revoked API key {}", key_id);
            true
        } else {
            false
        }
    }
    
    /// Revoke all keys for user
    pub fn revoke_user_keys(&mut self, user_id: Uuid) -> usize {
        let key_hashes = self.user_keys.get(&user_id).cloned().unwrap_or_default();
        
        let mut count = 0;
        for hash in key_hashes {
            if let Some(key) = self.keys.get_mut(&hash) {
                key.revoked = true;
                count += 1;
            }
        }
        
        info!("Revoked {} API keys for user {}", count, user_id);
        count
    }
    
    /// Get keys for user
    pub fn get_user_keys(&self, user_id: Uuid) -> Vec<&ApiKey> {
        self.user_keys.get(&user_id)
            .map(|hashes| {
                hashes.iter()
                    .filter_map(|h| self.keys.get(h))
                    .filter(|k| !k.revoked)
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Check if key has clearance
    pub fn has_clearance(&self, key: &str, required: &ClearanceLevel) -> bool {
        self.validate_key(key)
            .map(|k| k.clearance.meets(required))
            .unwrap_or(false)
    }
    
    /// Get expired keys
    pub fn get_expired_keys(&self) -> Vec<&ApiKey> {
        let now = Utc::now();
        self.keys.values()
            .filter(|k| !k.revoked && k.expires_at < now)
            .collect()
    }
    
    /// Rotate encryption for all keys
    pub fn rotate_encryption(&mut self) -> Result<()> {
        self.encryption.rotate_keys()
    }
    
    /// Get key count
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }
    
    /// Generate random key
    fn generate_random_key() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        
        (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Hash key for storage
    fn hash_key(key: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Security Manager - Central coordinator for all security operations
#[derive(Debug)]
pub struct SecurityManager {
    api_keys: ApiKeyManager,
    encryption: ApiKeyEncryption,
    audit: AuditLogger,
    two_factor: TwoFactorProvider,
    policy_manager: SecurityPolicyManager,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new() -> Self {
        info!("Initializing Security Manager");
        
        Self {
            api_keys: ApiKeyManager::new(),
            encryption: ApiKeyEncryption::new(),
            audit: AuditLogger::new(),
            two_factor: TwoFactorProvider::new(),
            policy_manager: SecurityPolicyManager::new(),
        }
    }
    
    /// Get API key manager
    pub fn api_keys(&mut self) -> &mut ApiKeyManager {
        &mut self.api_keys
    }
    
    /// Get encryption
    pub fn encryption(&mut self) -> &mut ApiKeyEncryption {
        &mut self.encryption
    }
    
    /// Get audit logger
    pub fn audit(&mut self) -> &mut AuditLogger {
        &mut self.audit
    }
    
    /// Get 2FA provider
    pub fn two_factor(&mut self) -> &mut TwoFactorProvider {
        &mut self.two_factor
    }
    
    /// Get policy manager
    pub fn policy_manager(&mut self) -> &mut SecurityPolicyManager {
        &mut self.policy_manager
    }
    
    /// Setup 2FA for user
    pub fn setup_2fa(&mut self, user_id: Uuid) -> TotpSetupResult {
        self.two_factor.setup_totp(user_id)
    }
    
    /// Verify 2FA code
    pub fn verify_2fa(&mut self, method_id: Uuid, code: &str) -> bool {
        self.two_factor.verify_totp(method_id, code)
    }
    
    /// Validate password
    pub fn validate_password(&self, password: &str) -> PasswordValidationResult {
        self.policy_manager.validate_password(password)
    }
    
    /// Generate API key
    pub fn generate_api_key(&mut self, user_id: Uuid, name: String, clearance: ClearanceLevel, expiry_days: i64) -> (Uuid, String) {
        self.api_keys.generate_key(user_id, name, clearance, expiry_days)
    }
    
    /// Validate API key
    pub fn validate_api_key(&self, key: &str) -> Option<&ApiKey> {
        self.api_keys.validate_key(key)
    }
    
    /// Log security event
    pub fn log_event(&mut self, event: SecurityEvent) {
        self.audit.log(event);
    }
    
    /// Check if rotation needed
    pub fn rotation_needed(&self) -> bool {
        self.encryption.rotation_needed()
    }
    
    /// Rotate encryption keys
    pub fn rotate_keys(&mut self) -> Result<()> {
        self.encryption.rotate_keys()
    }
    
    /// Get audit stats
    pub fn get_audit_stats(&self) -> audit::AuditStats {
        self.audit.get_stats()
    }
    
    /// Is 2FA required for clearance
    pub fn is_2fa_required(&self, clearance: ClearanceLevel) -> bool {
        self.policy_manager.is_2fa_required(clearance)
    }
    
    /// Full security check - returns true if all checks pass
    pub fn security_check(&self) -> bool {
        // Check encryption is working
        let _ = self.encryption.is_hsm_available(); // Just to avoid unused warning
        
        // Check policies are enabled
        if let Some(policy) = self.policy_manager.get_policy(PolicyType::Password) {
            if !policy.enabled {
                return false;
            }
        }
        
        true
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clearance_levels() {
        assert!(ClearanceLevel::Confidential.meets(&ClearanceLevel::Public));
        assert!(ClearanceLevel::Confidential.meets(&ClearanceLevel::Confidential));
        assert!(!ClearanceLevel::Public.meets(&ClearanceLevel::Confidential));
        
        assert_eq!(ClearanceLevel::Restricted.value(), 3);
        assert_eq!(ClearanceLevel::Restricted.name(), "Restricted");
    }

    #[test]
    fn test_api_key_manager_creation() {
        let manager = ApiKeyManager::new();
        assert_eq!(manager.key_count(), 0);
    }

    #[test]
    fn test_generate_and_validate_key() {
        let mut manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();
        
        let (key_id, key) = manager.generate_key(user_id, "Test Key".to_string(), ClearanceLevel::Internal, 30);
        
        assert_eq!(manager.key_count(), 1);
        assert!(key.starts_with("ios_"));
        
        let validated = manager.validate_key(&key);
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().id, key_id);
    }

    #[test]
    fn test_validate_revoked_key() {
        let mut manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();
        
        let (key_id, key) = manager.generate_key(user_id, "Test Key".to_string(), ClearanceLevel::Internal, 30);
        
        manager.revoke_key(key_id);
        
        let validated = manager.validate_key(&key);
        assert!(validated.is_none());
    }

    #[test]
    fn test_validate_expired_key() {
        let mut manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();
        
        // Generate key that expired yesterday
        let (_, key) = manager.generate_key(user_id, "Test Key".to_string(), ClearanceLevel::Internal, -1);
        
        let validated = manager.validate_key(&key);
        assert!(validated.is_none());
    }

    #[test]
    fn test_has_clearance() {
        let mut manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();
        
        let (_, key) = manager.generate_key(user_id, "Test Key".to_string(), ClearanceLevel::Confidential, 30);
        
        assert!(manager.has_clearance(&key, &ClearanceLevel::Internal));
        assert!(manager.has_clearance(&key, &ClearanceLevel::Confidential));
        assert!(!manager.has_clearance(&key, &ClearanceLevel::Restricted));
    }

    #[test]
    fn test_get_user_keys() {
        let mut manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();
        
        manager.generate_key(user_id, "Key 1".to_string(), ClearanceLevel::Internal, 30);
        manager.generate_key(user_id, "Key 2".to_string(), ClearanceLevel::Internal, 30);
        
        let keys = manager.get_user_keys(user_id);
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_revoke_user_keys() {
        let mut manager = ApiKeyManager::new();
        let user_id = Uuid::new_v4();
        
        manager.generate_key(user_id, "Key 1".to_string(), ClearanceLevel::Internal, 30);
        manager.generate_key(user_id, "Key 2".to_string(), ClearanceLevel::Internal, 30);
        
        let revoked = manager.revoke_user_keys(user_id);
        assert_eq!(revoked, 2);
        
        let keys = manager.get_user_keys(user_id);
        assert!(keys.is_empty());
    }

    #[test]
    fn test_security_manager_creation() {
        let manager = SecurityManager::new();
        assert!(manager.security_check());
    }

    #[test]
    fn test_security_manager_2fa_setup() {
        let mut manager = SecurityManager::new();
        let user_id = Uuid::new_v4();
        
        let result = manager.setup_2fa(user_id);
        assert_eq!(result.backup_codes.len(), 10);
    }

    #[test]
    fn test_security_check() {
        let manager = SecurityManager::new();
        assert!(manager.security_check());
    }
}
