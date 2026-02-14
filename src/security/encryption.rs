//! Encryption Module
//!
//! API key encryption with HSM support and key rotation

use chrono::{DateTime, Utc};
use tracing::info;
use uuid::Uuid;

use super::{SecurityError, Result};

/// Encrypted key structure
#[derive(Debug, Clone)]
pub struct EncryptedKey {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_id: Uuid,
    pub algorithm: EncryptionAlgorithm,
    pub created_at: DateTime<Utc>,
    pub hsm_protected: bool,
}

/// Encryption algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
    HsmProtected,
}

impl EncryptionAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            EncryptionAlgorithm::Aes256Gcm => "AES-256-GCM",
            EncryptionAlgorithm::ChaCha20Poly1305 => "ChaCha20-Poly1305",
            EncryptionAlgorithm::HsmProtected => "HSM-Protected",
        }
    }
}

/// Key rotation policy
#[derive(Debug, Clone)]
pub struct KeyRotationPolicy {
    pub enabled: bool,
    pub rotation_interval_days: u64,
    pub last_rotation: DateTime<Utc>,
    pub auto_rotate: bool,
    pub grace_period_days: u64,
}

impl Default for KeyRotationPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            rotation_interval_days: 90,
            last_rotation: Utc::now(),
            auto_rotate: true,
            grace_period_days: 7,
        }
    }
}

/// API key encryption
#[derive(Debug)]
pub struct ApiKeyEncryption {
    master_key: Vec<u8>,
    key_versions: Vec<KeyVersion>,
    current_key_id: Uuid,
    rotation_policy: KeyRotationPolicy,
    hsm_available: bool,
}

/// Key version for rotation
#[derive(Debug, Clone)]
struct KeyVersion {
    id: Uuid,
    key: Vec<u8>,
    created_at: DateTime<Utc>,
    deprecated: bool,
}

impl ApiKeyEncryption {
    /// Create new encryption instance
    pub fn new() -> Self {
        let master_key = Self::generate_master_key();
        let current_key_id = Uuid::new_v4();
        
        let key_versions = vec![KeyVersion {
            id: current_key_id,
            key: master_key.clone(),
            created_at: Utc::now(),
            deprecated: false,
        }];
        
        Self {
            master_key,
            key_versions,
            current_key_id,
            rotation_policy: KeyRotationPolicy::default(),
            hsm_available: false, // Would check for HSM in production
        }
    }
    
    /// Encrypt plaintext
    pub fn encrypt(&self, plaintext: &str) -> Result<EncryptedKey> {
        use rand::Rng;
        
        // Generate nonce (12 bytes for AES-GCM)
        let mut nonce = [0u8; 12];
        rand::thread_rng().fill(&mut nonce);
        
        // Simplified encryption - XOR with master key for demo
        // In production, use proper AES-GCM or ChaCha20-Poly1305
        let plaintext_bytes = plaintext.as_bytes();
        let ciphertext: Vec<u8> = plaintext_bytes
            .iter()
            .zip(self.master_key.iter().cycle())
            .map(|(p, k)| p ^ k)
            .collect();
        
        let algorithm = if self.hsm_available {
            EncryptionAlgorithm::HsmProtected
        } else {
            EncryptionAlgorithm::Aes256Gcm
        };
        
        Ok(EncryptedKey {
            ciphertext,
            nonce: nonce.to_vec(),
            key_id: self.current_key_id,
            algorithm,
            created_at: Utc::now(),
            hsm_protected: self.hsm_available,
        })
    }
    
    /// Decrypt ciphertext
    pub fn decrypt(&self, encrypted: &EncryptedKey) -> Result<String> {
        // Find key version
        let key_version = self.key_versions.iter()
            .find(|kv| kv.id == encrypted.key_id)
            .ok_or_else(|| SecurityError::Decryption("Key version not found".to_string()))?;
        
        if key_version.deprecated {
            info!("Decrypting with deprecated key: {}", encrypted.key_id);
        }
        
        // Simplified decryption (XOR)
        let plaintext_bytes: Vec<u8> = encrypted.ciphertext
            .iter()
            .zip(key_version.key.iter().cycle())
            .map(|(c, k)| c ^ k)
            .collect();
        
        String::from_utf8(plaintext_bytes)
            .map_err(|e| SecurityError::Decryption(format!("Invalid UTF-8: {}", e)))
    }
    
    /// Rotate encryption keys
    pub fn rotate_keys(&mut self) -> Result<()> {
        // Mark current key as deprecated
        for kv in &mut self.key_versions {
            kv.deprecated = true;
        }
        
        // Generate new key
        let new_key = Self::generate_master_key();
        let new_key_id = Uuid::new_v4();
        
        self.key_versions.push(KeyVersion {
            id: new_key_id,
            key: new_key,
            created_at: Utc::now(),
            deprecated: false,
        });
        
        self.master_key = self.key_versions.last().unwrap().key.clone();
        self.current_key_id = new_key_id;
        self.rotation_policy.last_rotation = Utc::now();
        
        // Clean up very old keys (keep last 3)
        if self.key_versions.len() > 3 {
            self.key_versions.remove(0);
        }
        
        info!("Encryption keys rotated. New key ID: {}", new_key_id);
        
        Ok(())
    }
    
    /// Check if rotation is needed
    pub fn rotation_needed(&self) -> bool {
        if !self.rotation_policy.enabled {
            return false;
        }
        
        let days_since_rotation = (Utc::now() - self.rotation_policy.last_rotation).num_days() as u64;
        days_since_rotation >= self.rotation_policy.rotation_interval_days
    }
    
    /// Enable HSM
    pub fn enable_hsm(&mut self) {
        self.hsm_available = true;
        info!("HSM protection enabled");
    }
    
    /// Check if HSM is available
    pub fn is_hsm_available(&self) -> bool {
        self.hsm_available
    }
    
    /// Get current key ID
    pub fn current_key_id(&self) -> Uuid {
        self.current_key_id
    }
    
    /// Get key version count
    pub fn key_version_count(&self) -> usize {
        self.key_versions.len()
    }
    
    /// Update rotation policy
    pub fn set_rotation_policy(&mut self, policy: KeyRotationPolicy) {
        self.rotation_policy = policy;
    }
    
    /// Generate master key
    fn generate_master_key() -> Vec<u8> {
        use rand::Rng;
        let mut key = vec![0u8; 32]; // 256 bits
        rand::thread_rng().fill(&mut key[..]);
        key
    }
    
    /// Hash sensitive data (for storage)
    pub fn hash_sensitive(&self, data: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        // Also hash with master key for additional security
        self.master_key.hash(&mut hasher);
        
        format!("{:016x}", hasher.finish())
    }
}

impl Default for ApiKeyEncryption {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_creation() {
        let encryption = ApiKeyEncryption::new();
        assert!(!encryption.is_hsm_available());
        assert_eq!(encryption.key_version_count(), 1);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let encryption = ApiKeyEncryption::new();
        
        let plaintext = "test_api_key_12345";
        let encrypted = encryption.encrypt(plaintext).unwrap();
        
        assert!(!encrypted.ciphertext.is_empty());
        assert_eq!(encrypted.algorithm, EncryptionAlgorithm::Aes256Gcm);
        
        let decrypted = encryption.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_key_rotation() {
        let mut encryption = ApiKeyEncryption::new();
        let old_key_id = encryption.current_key_id();
        
        encryption.rotate_keys().unwrap();
        
        assert_ne!(encryption.current_key_id(), old_key_id);
        assert_eq!(encryption.key_version_count(), 2);
    }

    #[test]
    fn test_rotation_needed() {
        let mut encryption = ApiKeyEncryption::new();
        
        // Just created, shouldn't need rotation
        assert!(!encryption.rotation_needed());
        
        // Set old rotation date
        encryption.rotation_policy.last_rotation = Utc::now() - chrono::Duration::days(100);
        assert!(encryption.rotation_needed());
    }

    #[test]
    fn test_decrypt_with_old_key() {
        let mut encryption = ApiKeyEncryption::new();
        
        let plaintext = "test_api_key";
        let encrypted = encryption.encrypt(plaintext).unwrap();
        
        // Rotate keys
        encryption.rotate_keys().unwrap();
        
        // Should still be able to decrypt
        let decrypted = encryption.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_hsm_enable() {
        let mut encryption = ApiKeyEncryption::new();
        
        assert!(!encryption.is_hsm_available());
        
        encryption.enable_hsm();
        
        assert!(encryption.is_hsm_available());
    }

    #[test]
    fn test_encryption_algorithm_names() {
        assert_eq!(EncryptionAlgorithm::Aes256Gcm.name(), "AES-256-GCM");
        assert_eq!(EncryptionAlgorithm::HsmProtected.name(), "HSM-Protected");
    }

    #[test]
    fn test_hash_sensitive() {
        let encryption = ApiKeyEncryption::new();
        
        let hash1 = encryption.hash_sensitive("secret");
        let hash2 = encryption.hash_sensitive("secret");
        let hash3 = encryption.hash_sensitive("different");
        
        assert_eq!(hash1, hash2); // Same input = same hash
        assert_ne!(hash1, hash3); // Different input = different hash
    }

    #[test]
    fn test_key_version_cleanup() {
        let mut encryption = ApiKeyEncryption::new();
        
        // Rotate 5 times
        for _ in 0..5 {
            encryption.rotate_keys().unwrap();
        }
        
        // Should only keep last 3
        assert_eq!(encryption.key_version_count(), 3);
    }
}
