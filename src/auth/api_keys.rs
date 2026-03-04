//! Persistent API key management (Sprint 106).
//!
//! API keys are stored with SHA-256 hashes in PostgreSQL.
//! Only the key prefix (first 8 chars) is returned after creation.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::error::AuthError;

/// Create a new API key for a user. Returns the full key (shown once).
pub async fn create_api_key(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    permissions: &[String],
    expires_in_days: Option<i64>,
) -> Result<ApiKeyCreated, AuthError> {
    let raw_key = format!("ik_{}", Uuid::new_v4().as_simple());
    let key_hash = hash_key(&raw_key);
    let key_prefix = raw_key[..12].to_string();

    let expires_at = expires_in_days.map(|days| Utc::now() + chrono::Duration::days(days));
    let permissions_json = serde_json::to_value(permissions).unwrap_or_default();

    let id: (Uuid,) = sqlx::query_as(
        "INSERT INTO api_keys (user_id, name, key_hash, key_prefix, permissions, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(user_id)
    .bind(name)
    .bind(&key_hash)
    .bind(&key_prefix)
    .bind(&permissions_json)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(ApiKeyCreated {
        id: id.0,
        name: name.to_string(),
        api_key: raw_key,
        key_prefix,
        permissions: permissions.to_vec(),
        expires_at,
        created_at: Utc::now(),
    })
}

/// List all API keys for a user (never returns the full key).
pub async fn list_api_keys(pool: &PgPool, user_id: Uuid) -> Result<Vec<ApiKeyInfo>, AuthError> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, key_prefix, permissions, expires_at, last_used_at, revoked_at, created_at
         FROM api_keys
         WHERE user_id = $1 AND revoked_at IS NULL
         ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            let permissions: Vec<String> =
                serde_json::from_value(r.try_get("permissions")?).unwrap_or_default();
            Ok(ApiKeyInfo {
                id: r.try_get("id")?,
                name: r.try_get("name")?,
                key_prefix: r.try_get("key_prefix")?,
                permissions,
                expires_at: r.try_get("expires_at")?,
                last_used_at: r.try_get("last_used_at")?,
                created_at: r.try_get("created_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(AuthError::Database)
}

/// Revoke an API key.
pub async fn revoke_api_key(pool: &PgPool, key_id: Uuid, user_id: Uuid) -> Result<bool, AuthError> {
    let result = sqlx::query(
        "UPDATE api_keys SET revoked_at = NOW()
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(key_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Validate an API key and return the user_id if valid.
pub async fn validate_api_key(pool: &PgPool, raw_key: &str) -> Result<Option<Uuid>, AuthError> {
    let key_hash = hash_key(raw_key);

    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM api_keys
         WHERE key_hash = $1
           AND revoked_at IS NULL
           AND (expires_at IS NULL OR expires_at > NOW())",
    )
    .bind(&key_hash)
    .fetch_optional(pool)
    .await?;

    if let Some((user_id,)) = row {
        // Update last_used_at
        let _ = sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE key_hash = $1")
            .bind(&key_hash)
            .execute(pool)
            .await;
        Ok(Some(user_id))
    } else {
        Ok(None)
    }
}

fn hash_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
}

#[derive(Debug, serde::Serialize)]
pub struct ApiKeyCreated {
    pub id: Uuid,
    pub name: String,
    pub api_key: String,
    pub key_prefix: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_key("test-key");
        let h2 = hash_key("test-key");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn different_keys_different_hashes() {
        let h1 = hash_key("key-a");
        let h2 = hash_key("key-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn generated_key_starts_with_prefix() {
        // Simulate key format
        let key = format!("ik_{}", Uuid::new_v4().as_simple());
        assert!(key.starts_with("ik_"));
        assert!(key.len() > 12);
    }
}
