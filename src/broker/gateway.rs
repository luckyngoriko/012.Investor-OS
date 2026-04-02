//! Broker Gateway — unified interface for storing and retrieving
//! encrypted broker API credentials.
//!
//! Uses [`Vault`] for AES-256-GCM encryption and `sqlx` runtime queries
//! for persistence in the `broker_connections` table.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::vault::{Vault, VaultError};

/// Gateway errors
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("Vault error: {0}")]
    Vault(#[from] VaultError),

    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("No active connection found for user {user_id} / broker {broker_type}")]
    NotFound { user_id: Uuid, broker_type: String },
}

/// Unified broker gateway for managing encrypted API key storage.
pub struct BrokerGateway {
    pool: PgPool,
    vault: Vault,
}

impl BrokerGateway {
    /// Create a new gateway.
    ///
    /// The vault key is read from `BROKER_VAULT_KEY` env var.
    pub fn new(pool: PgPool) -> Result<Self, GatewayError> {
        let vault = Vault::from_env()?;
        Ok(Self { pool, vault })
    }

    /// Create a gateway with an explicit vault (useful for testing).
    pub fn with_vault(pool: PgPool, vault: Vault) -> Self {
        Self { pool, vault }
    }

    /// Store a broker connection with encrypted API keys.
    ///
    /// Returns the UUID of the newly created `broker_connections` row.
    pub async fn store_connection(
        &self,
        user_id: Uuid,
        broker_type: &str,
        api_key: &str,
        api_secret: &str,
        label: Option<&str>,
    ) -> Result<Uuid, GatewayError> {
        let (enc_key, key_nonce) = self.vault.encrypt(api_key)?;
        let (enc_secret, secret_nonce) = self.vault.encrypt(api_secret)?;

        let row = sqlx::query(
            r#"
            INSERT INTO broker_connections
                (user_id, broker_type, encrypted_api_key, encrypted_api_secret,
                 key_nonce, secret_nonce, label)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (user_id, broker_type, label)
            DO UPDATE SET
                encrypted_api_key   = EXCLUDED.encrypted_api_key,
                encrypted_api_secret = EXCLUDED.encrypted_api_secret,
                key_nonce           = EXCLUDED.key_nonce,
                secret_nonce        = EXCLUDED.secret_nonce,
                is_active           = true
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(broker_type)
        .bind(&enc_key)
        .bind(&enc_secret)
        .bind(&key_nonce)
        .bind(&secret_nonce)
        .bind(label)
        .fetch_one(&self.pool)
        .await?;

        let id: Uuid = row.try_get("id")?;
        Ok(id)
    }

    /// Retrieve and decrypt API keys for a user + broker combination.
    ///
    /// Returns `(api_key, api_secret)` in plaintext.
    pub async fn get_decrypted_keys(
        &self,
        user_id: Uuid,
        broker_type: &str,
    ) -> Result<(String, String), GatewayError> {
        let row = sqlx::query(
            r#"
            SELECT encrypted_api_key, encrypted_api_secret, key_nonce, secret_nonce
            FROM broker_connections
            WHERE user_id = $1 AND broker_type = $2 AND is_active = true
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(broker_type)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| GatewayError::NotFound {
            user_id,
            broker_type: broker_type.to_string(),
        })?;

        let enc_key: String = row.try_get("encrypted_api_key")?;
        let enc_secret: String = row.try_get("encrypted_api_secret")?;
        let key_nonce: String = row.try_get("key_nonce")?;
        let secret_nonce: String = row.try_get("secret_nonce")?;

        let api_key = self.vault.decrypt(&enc_key, &key_nonce)?;
        let api_secret = self.vault.decrypt(&enc_secret, &secret_nonce)?;

        Ok((api_key, api_secret))
    }
}
