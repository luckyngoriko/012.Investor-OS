//! AES-256-GCM Vault for encrypting broker API keys.
//!
//! Reads a 32-byte master key from the `BROKER_VAULT_KEY` env var (64 hex chars).
//! Each encryption produces a random 12-byte nonce so identical plaintexts yield
//! different ciphertexts.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use thiserror::Error;

/// Vault errors
#[derive(Error, Debug)]
pub enum VaultError {
    #[error("BROKER_VAULT_KEY env var missing")]
    MissingKey,

    #[error("BROKER_VAULT_KEY must be exactly 64 hex characters (32 bytes)")]
    InvalidKeyLength,

    #[error("BROKER_VAULT_KEY contains invalid hex: {0}")]
    InvalidHex(String),

    #[error("Encryption failed: {0}")]
    Encrypt(String),

    #[error("Decryption failed: {0}")]
    Decrypt(String),

    #[error("Invalid base64: {0}")]
    Base64(String),
}

/// AES-256-GCM vault for broker API key encryption.
pub struct Vault {
    cipher: Aes256Gcm,
}

impl std::fmt::Debug for Vault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vault")
            .field("cipher", &"<redacted>")
            .finish()
    }
}

impl Vault {
    /// Create a vault from the `BROKER_VAULT_KEY` environment variable.
    ///
    /// The key must be exactly 64 hex characters (representing 32 bytes).
    pub fn from_env() -> Result<Self, VaultError> {
        let hex_key = std::env::var("BROKER_VAULT_KEY").map_err(|_| VaultError::MissingKey)?;
        Self::from_hex(&hex_key)
    }

    /// Create a vault from a hex-encoded key string.
    pub fn from_hex(hex_key: &str) -> Result<Self, VaultError> {
        if hex_key.len() != 64 {
            return Err(VaultError::InvalidKeyLength);
        }

        let key_bytes = hex_to_bytes(hex_key)?;
        let key = aes_gcm::aead::generic_array::GenericArray::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { cipher })
    }

    /// Encrypt plaintext, returning `(ciphertext_base64, nonce_base64)`.
    ///
    /// A random 12-byte nonce is generated for each call, so encrypting the same
    /// plaintext twice produces different ciphertexts.
    pub fn encrypt(&self, plaintext: &str) -> Result<(String, String), VaultError> {
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| VaultError::Encrypt(e.to_string()))?;

        let ciphertext_b64 = B64.encode(&ciphertext);
        let nonce_b64 = B64.encode(nonce_bytes);

        Ok((ciphertext_b64, nonce_b64))
    }

    /// Decrypt a ciphertext given its base64-encoded ciphertext and nonce.
    pub fn decrypt(&self, ciphertext_b64: &str, nonce_b64: &str) -> Result<String, VaultError> {
        let ciphertext = B64
            .decode(ciphertext_b64)
            .map_err(|e| VaultError::Base64(e.to_string()))?;
        let nonce_bytes = B64
            .decode(nonce_b64)
            .map_err(|e| VaultError::Base64(e.to_string()))?;

        if nonce_bytes.len() != 12 {
            return Err(VaultError::Decrypt(format!(
                "nonce must be 12 bytes, got {}",
                nonce_bytes.len()
            )));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| VaultError::Decrypt(e.to_string()))?;

        String::from_utf8(plaintext).map_err(|e| VaultError::Decrypt(format!("invalid UTF-8: {e}")))
    }
}

/// Convert a hex string to bytes.
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, VaultError> {
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| VaultError::InvalidHex(hex[i..i + 2].to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic 32-byte key for testing (64 hex chars).
    const TEST_KEY_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let vault = Vault::from_hex(TEST_KEY_HEX).expect("valid test key");
        let plaintext = "my-super-secret-api-key-12345";

        let (ciphertext_b64, nonce_b64) = vault.encrypt(plaintext).expect("encrypt");
        let decrypted = vault.decrypt(&ciphertext_b64, &nonce_b64).expect("decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn different_nonce_per_call() {
        let vault = Vault::from_hex(TEST_KEY_HEX).expect("valid test key");
        let plaintext = "same-input";

        let (ct1, n1) = vault.encrypt(plaintext).expect("encrypt 1");
        let (ct2, n2) = vault.encrypt(plaintext).expect("encrypt 2");

        // Nonces must differ (random), so ciphertexts must also differ.
        assert_ne!(n1, n2);
        assert_ne!(ct1, ct2);

        // Both must still decrypt to the same plaintext.
        assert_eq!(vault.decrypt(&ct1, &n1).unwrap(), plaintext);
        assert_eq!(vault.decrypt(&ct2, &n2).unwrap(), plaintext);
    }

    #[test]
    fn invalid_key_length() {
        let result = Vault::from_hex("0123456789abcdef"); // too short
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VaultError::InvalidKeyLength));
    }

    #[test]
    fn invalid_hex_chars() {
        // 64 chars but 'zz' is not valid hex
        let bad = "zz23456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = Vault::from_hex(bad);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VaultError::InvalidHex(_)));
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let vault1 = Vault::from_hex(TEST_KEY_HEX).expect("key 1");
        let vault2 =
            Vault::from_hex("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
                .expect("key 2");

        let (ct, nonce) = vault1.encrypt("secret").expect("encrypt");
        let result = vault2.decrypt(&ct, &nonce);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_tampered_ciphertext_fails() {
        let vault = Vault::from_hex(TEST_KEY_HEX).expect("valid key");
        let (ct, nonce) = vault.encrypt("secret").expect("encrypt");

        // Tamper with ciphertext by changing the first character.
        let tampered = format!("X{}", &ct[1..]);
        // This may or may not fail at base64 decode vs AEAD auth — either is correct.
        let result = vault.decrypt(&tampered, &nonce);
        assert!(result.is_err());
    }
}
