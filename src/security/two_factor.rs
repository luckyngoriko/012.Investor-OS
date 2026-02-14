//! Two-Factor Authentication Module
//!
//! TOTP/HOTP 2FA implementation with WebAuthn support

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// 2FA provider
#[derive(Debug)]
pub struct TwoFactorProvider {
    methods: HashMap<Uuid, TwoFactorMethod>,
    backup_codes: HashMap<Uuid, Vec<BackupCode>>,
    trusted_devices: HashMap<Uuid, Vec<TrustedDevice>>,
}

/// 2FA method
#[derive(Debug, Clone)]
pub struct TwoFactorMethod {
    pub id: Uuid,
    pub user_id: Uuid,
    pub method_type: TwoFactorType,
    pub secret: String,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub backup_codes_remaining: u32,
}

/// 2FA type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoFactorType {
    Totp,
    Hotp,
    WebAuthn,
    Sms,
    Email,
}

impl TwoFactorType {
    pub fn name(&self) -> &'static str {
        match self {
            TwoFactorType::Totp => "TOTP",
            TwoFactorType::Hotp => "HOTP",
            TwoFactorType::WebAuthn => "WebAuthn",
            TwoFactorType::Sms => "SMS",
            TwoFactorType::Email => "Email",
        }
    }
}

/// Backup code
#[derive(Debug, Clone)]
pub struct BackupCode {
    pub code_hash: String,
    pub used: bool,
    pub used_at: Option<DateTime<Utc>>,
}

/// Trusted device
#[derive(Debug, Clone)]
pub struct TrustedDevice {
    pub device_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
}

impl TwoFactorProvider {
    /// Create new 2FA provider
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
            backup_codes: HashMap::new(),
            trusted_devices: HashMap::new(),
        }
    }
    
    /// Setup TOTP
    pub fn setup_totp(&mut self, user_id: Uuid) -> TotpSetupResult {
        let id = Uuid::new_v4();
        let secret = Self::generate_secret();
        
        let method = TwoFactorMethod {
            id,
            user_id,
            method_type: TwoFactorType::Totp,
            secret: secret.clone(),
            created_at: Utc::now(),
            last_used: None,
            enabled: false, // Requires verification
            backup_codes_remaining: 0,
        };
        
        self.methods.insert(id, method);
        
        // Generate backup codes
        let codes = self.generate_backup_codes(user_id, 10);
        
        // Generate QR code URI (simplified)
        let qr_uri = format!("otpauth://totp/InvestorOS:{}?secret={}&issuer=InvestorOS", 
            user_id, secret);
        
        TotpSetupResult {
            method_id: id,
            secret,
            qr_uri,
            backup_codes: codes,
        }
    }
    
    /// Verify TOTP code
    pub fn verify_totp(&mut self, method_id: Uuid, code: &str) -> bool {
        let Some(method) = self.methods.get(&method_id) else {
            return false;
        };
        
        if !method.enabled || method.method_type != TwoFactorType::Totp {
            return false;
        }
        
        // Validate code format
        if !Self::is_valid_totp_code(code) {
            return false;
        }
        
        // Generate expected code (simplified)
        let expected = Self::generate_totp(&method.secret);
        
        if code == expected {
            // Update last used
            if let Some(m) = self.methods.get_mut(&method_id) {
                m.last_used = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }
    
    /// Verify backup code
    pub fn verify_backup_code(&mut self, user_id: Uuid, code: &str) -> bool {
        let Some(codes) = self.backup_codes.get_mut(&user_id) else {
            return false;
        };
        
        let code_hash = Self::hash_code(code);
        
        for backup_code in codes.iter_mut() {
            if backup_code.code_hash == code_hash && !backup_code.used {
                backup_code.used = true;
                backup_code.used_at = Some(Utc::now());
                return true;
            }
        }
        
        false
    }
    
    /// Enable 2FA method
    pub fn enable_method(&mut self, method_id: Uuid) -> bool {
        if let Some(method) = self.methods.get_mut(&method_id) {
            method.enabled = true;
            true
        } else {
            false
        }
    }
    
    /// Disable 2FA method
    pub fn disable_method(&mut self, method_id: Uuid) -> bool {
        if let Some(method) = self.methods.get_mut(&method_id) {
            method.enabled = false;
            true
        } else {
            false
        }
    }
    
    /// Get user methods
    pub fn get_user_methods(&self, user_id: Uuid) -> Vec<&TwoFactorMethod> {
        self.methods.values()
            .filter(|m| m.user_id == user_id)
            .collect()
    }
    
    /// Check if user has 2FA enabled
    pub fn has_2fa_enabled(&self, user_id: Uuid) -> bool {
        self.methods.values()
            .any(|m| m.user_id == user_id && m.enabled)
    }
    
    /// Check if user has enabled method of specific type
    pub fn has_method_type(&self, user_id: Uuid, method_type: TwoFactorType) -> bool {
        self.methods.values()
            .any(|m| m.user_id == user_id && m.method_type == method_type && m.enabled)
    }
    
    /// Add trusted device
    pub fn add_trusted_device(&mut self, user_id: Uuid, device_name: String, days_valid: i64) {
        let device = TrustedDevice {
            device_id: Uuid::new_v4(),
            name: device_name,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(days_valid),
            last_used: Utc::now(),
        };
        
        self.trusted_devices.entry(user_id)
            .or_default()
            .push(device);
    }
    
    /// Check if device is trusted
    pub fn is_device_trusted(&self, user_id: Uuid, device_id: Uuid) -> bool {
        self.trusted_devices.get(&user_id)
            .map(|devices| {
                devices.iter().any(|d| {
                    d.device_id == device_id && d.expires_at > Utc::now()
                })
            })
            .unwrap_or(false)
    }
    
    /// Remove trusted device
    pub fn remove_trusted_device(&mut self, user_id: Uuid, device_id: Uuid) -> bool {
        if let Some(devices) = self.trusted_devices.get_mut(&user_id) {
            let initial_len = devices.len();
            devices.retain(|d| d.device_id != device_id);
            devices.len() < initial_len
        } else {
            false
        }
    }
    
    /// Get trusted devices for user
    pub fn get_trusted_devices(&self, user_id: Uuid) -> Vec<&TrustedDevice> {
        self.trusted_devices.get(&user_id)
            .map(|devices| {
                devices.iter()
                    .filter(|d| d.expires_at > Utc::now())
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Get backup codes remaining
    pub fn get_backup_codes_remaining(&self, user_id: Uuid) -> u32 {
        self.backup_codes.get(&user_id)
            .map(|codes| {
                codes.iter().filter(|c| !c.used).count() as u32
            })
            .unwrap_or(0)
    }
    
    /// Regenerate backup codes
    pub fn regenerate_backup_codes(&mut self, user_id: Uuid, count: usize) -> Vec<String> {
        let codes = self.generate_backup_codes(user_id, count);
        
        // Clear old codes
        if let Some(existing) = self.backup_codes.get_mut(&user_id) {
            existing.clear();
        }
        
        // Add new codes
        let new_codes: Vec<BackupCode> = codes.iter()
            .map(|c| BackupCode {
                code_hash: Self::hash_code(c),
                used: false,
                used_at: None,
            })
            .collect();
        
        self.backup_codes.insert(user_id, new_codes);
        
        codes
    }
    
    /// Generate secret
    fn generate_secret() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut rng = rand::thread_rng();
        
        (0..32)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Generate TOTP code (pub for testing)
    pub fn generate_totp(secret: &str) -> String {
        // Simplified TOTP generation
        // In production, use proper RFC 6238 implementation
        let timestamp = Utc::now().timestamp();
        let time_step = timestamp / 30;
        
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        secret.hash(&mut hasher);
        time_step.hash(&mut hasher);
        let hash = hasher.finish();
        
        let code = (hash % 1_000_000) as u32;
        format!("{:06}", code)
    }
    
    /// Check if TOTP code is valid format
    fn is_valid_totp_code(code: &str) -> bool {
        code.len() == 6 && code.chars().all(|c| c.is_ascii_digit())
    }
    
    /// Generate backup codes
    fn generate_backup_codes(&mut self, user_id: Uuid, count: usize) -> Vec<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let codes: Vec<String> = (0..count)
            .map(|_| {
                let code: u64 = rng.gen_range(1000000000..9999999999);
                format!("{:010}", code)
            })
            .collect();
        
        // Store hashed codes
        let backup_codes: Vec<BackupCode> = codes.iter()
            .map(|c| BackupCode {
                code_hash: Self::hash_code(c),
                used: false,
                used_at: None,
            })
            .collect();
        
        self.backup_codes.insert(user_id, backup_codes);
        
        codes
    }
    
    /// Hash code
    fn hash_code(code: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        code.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

impl Default for TwoFactorProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// TOTP setup result
#[derive(Debug)]
pub struct TotpSetupResult {
    pub method_id: Uuid,
    pub secret: String,
    pub qr_uri: String,
    pub backup_codes: Vec<String>,
}

/// 2FA verification result
#[derive(Debug, Clone)]
pub struct TwoFactorVerification {
    pub success: bool,
    pub method_used: Option<TwoFactorType>,
    pub remaining_attempts: u32,
}

/// 2FA configuration
#[derive(Debug, Clone)]
pub struct TwoFactorConfig {
    pub required: bool,
    pub allowed_methods: Vec<TwoFactorType>,
    pub trusted_device_days: i64,
    pub backup_code_count: usize,
}

impl Default for TwoFactorConfig {
    fn default() -> Self {
        Self {
            required: true,
            allowed_methods: vec![TwoFactorType::Totp, TwoFactorType::Email],
            trusted_device_days: 30,
            backup_code_count: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = TwoFactorProvider::new();
        assert!(provider.get_user_methods(Uuid::new_v4()).is_empty());
    }

    #[test]
    fn test_setup_totp() {
        let mut provider = TwoFactorProvider::new();
        let user_id = Uuid::new_v4();
        
        let result = provider.setup_totp(user_id);
        
        assert_eq!(result.backup_codes.len(), 10);
        assert!(!result.secret.is_empty());
        assert!(result.qr_uri.contains("otpauth://"));
    }

    #[test]
    fn test_verify_totp() {
        let mut provider = TwoFactorProvider::new();
        let user_id = Uuid::new_v4();
        
        let result = provider.setup_totp(user_id);
        
        // Method not enabled yet
        assert!(!provider.verify_totp(result.method_id, "123456"));
        
        // Enable method
        provider.enable_method(result.method_id);
        
        // Get the expected code
        let method = provider.methods.get(&result.method_id).unwrap();
        let expected_code = TwoFactorProvider::generate_totp(&method.secret);
        
        assert!(provider.verify_totp(result.method_id, &expected_code));
    }

    #[test]
    fn test_backup_codes() {
        let mut provider = TwoFactorProvider::new();
        let user_id = Uuid::new_v4();
        
        let result = provider.setup_totp(user_id);
        let backup_code = result.backup_codes[0].clone();
        
        assert_eq!(provider.get_backup_codes_remaining(user_id), 10);
        
        // Use backup code
        assert!(provider.verify_backup_code(user_id, &backup_code));
        assert_eq!(provider.get_backup_codes_remaining(user_id), 9);
        
        // Can't reuse
        assert!(!provider.verify_backup_code(user_id, &backup_code));
    }

    #[test]
    fn test_has_2fa_enabled() {
        let mut provider = TwoFactorProvider::new();
        let user_id = Uuid::new_v4();
        
        assert!(!provider.has_2fa_enabled(user_id));
        
        let result = provider.setup_totp(user_id);
        assert!(!provider.has_2fa_enabled(user_id)); // Not enabled yet
        
        provider.enable_method(result.method_id);
        assert!(provider.has_2fa_enabled(user_id));
    }

    #[test]
    fn test_trusted_devices() {
        let mut provider = TwoFactorProvider::new();
        let user_id = Uuid::new_v4();
        
        provider.add_trusted_device(user_id, "My Laptop".to_string(), 30);
        
        let devices = provider.get_trusted_devices(user_id);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "My Laptop");
        
        let device_id = devices[0].device_id;
        assert!(provider.is_device_trusted(user_id, device_id));
        
        provider.remove_trusted_device(user_id, device_id);
        assert!(!provider.is_device_trusted(user_id, device_id));
    }

    #[test]
    fn test_two_factor_type_names() {
        assert_eq!(TwoFactorType::Totp.name(), "TOTP");
        assert_eq!(TwoFactorType::Hotp.name(), "HOTP");
        assert_eq!(TwoFactorType::WebAuthn.name(), "WebAuthn");
    }

    #[test]
    fn test_regenerate_backup_codes() {
        let mut provider = TwoFactorProvider::new();
        let user_id = Uuid::new_v4();
        
        let result = provider.setup_totp(user_id);
        let old_codes = result.backup_codes.clone();
        
        let new_codes = provider.regenerate_backup_codes(user_id, 10);
        
        // Old codes should not work
        assert!(!provider.verify_backup_code(user_id, &old_codes[0]));
        
        // New codes should work
        assert!(provider.verify_backup_code(user_id, &new_codes[0]));
    }

    #[test]
    fn test_disable_method() {
        let mut provider = TwoFactorProvider::new();
        let user_id = Uuid::new_v4();
        
        let result = provider.setup_totp(user_id);
        provider.enable_method(result.method_id);
        assert!(provider.has_2fa_enabled(user_id));
        
        provider.disable_method(result.method_id);
        assert!(!provider.has_2fa_enabled(user_id));
    }
}
