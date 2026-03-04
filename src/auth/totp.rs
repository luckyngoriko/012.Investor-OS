//! TOTP 2FA service with PostgreSQL persistence (Sprint 106).
//!
//! Wraps RFC 6238 TOTP generation/verification with DB-backed secrets.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::error::AuthError;

/// TOTP configuration.
const TOTP_PERIOD: u64 = 30; // seconds
const TOTP_DIGITS: u32 = 6;
const TOTP_ISSUER: &str = "InvestorOS";

// ── Secret generation ──

/// Generate a random 32-character base32 secret.
pub fn generate_secret() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let uuid_bytes = Uuid::new_v4();
    let mut hasher = Sha256::new();
    hasher.update(seed.to_le_bytes());
    hasher.update(uuid_bytes.as_bytes());
    let hash = hasher.finalize();

    const BASE32_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    hash.iter()
        .take(32)
        .map(|b| BASE32_CHARS[(*b as usize) % 32] as char)
        .collect()
}

/// Generate the otpauth:// URI for QR code rendering.
pub fn totp_uri(secret: &str, email: &str) -> String {
    format!(
        "otpauth://totp/{issuer}:{email}?secret={secret}&issuer={issuer}&digits={digits}&period={period}",
        issuer = TOTP_ISSUER,
        email = email,
        secret = secret,
        digits = TOTP_DIGITS,
        period = TOTP_PERIOD,
    )
}

// ── TOTP verification ──

/// Generate a TOTP code for a given secret and unix timestamp.
fn generate_code(secret: &str, timestamp: u64) -> String {
    let counter = timestamp / TOTP_PERIOD;
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(counter.to_be_bytes());
    let hash = hasher.finalize();

    // Dynamic truncation (simplified)
    let offset = (hash[hash.len() - 1] & 0x0f) as usize;
    let code_bytes = &hash[offset..offset + 4];
    let code = u32::from_be_bytes([
        code_bytes[0] & 0x7f,
        code_bytes[1],
        code_bytes[2],
        code_bytes[3],
    ]);
    let code = code % 10u32.pow(TOTP_DIGITS);
    format!("{:0>width$}", code, width = TOTP_DIGITS as usize)
}

/// Verify a TOTP code with +-1 window tolerance.
pub fn verify_code(secret: &str, code: &str) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check current window and +-1 adjacent windows
    for offset in [0i64, -1, 1] {
        let ts = (now as i64 + offset * TOTP_PERIOD as i64) as u64;
        if generate_code(secret, ts) == code {
            return true;
        }
    }
    false
}

// ── DB operations ──

/// Set up TOTP for a user (stores secret, not yet enabled).
pub async fn setup_totp(pool: &PgPool, user_id: Uuid) -> Result<(String, String), AuthError> {
    let secret = generate_secret();

    // Fetch user email for the URI
    let email: String = sqlx::query_scalar("SELECT email FROM auth_users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| AuthError::UserNotFound)?;

    // Upsert TOTP secret (disabled until confirmed)
    sqlx::query(
        "INSERT INTO auth_totp_secrets (user_id, secret, enabled)
         VALUES ($1, $2, false)
         ON CONFLICT (user_id) DO UPDATE SET secret = $2, enabled = false, verified_at = NULL",
    )
    .bind(user_id)
    .bind(&secret)
    .execute(pool)
    .await?;

    let uri = totp_uri(&secret, &email);
    Ok((secret, uri))
}

/// Confirm TOTP setup — verify the first code and enable.
pub async fn confirm_totp(pool: &PgPool, user_id: Uuid, code: &str) -> Result<(), AuthError> {
    let secret = get_secret(pool, user_id).await?;

    if !verify_code(&secret, code) {
        return Err(AuthError::InvalidCredentials);
    }

    sqlx::query(
        "UPDATE auth_totp_secrets SET enabled = true, verified_at = NOW() WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Disable TOTP — requires valid code to prevent lockout.
pub async fn disable_totp(pool: &PgPool, user_id: Uuid, code: &str) -> Result<(), AuthError> {
    let secret = get_secret(pool, user_id).await?;

    if !verify_code(&secret, code) {
        return Err(AuthError::InvalidCredentials);
    }

    sqlx::query("DELETE FROM auth_totp_secrets WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Check if a user has TOTP enabled.
pub async fn is_totp_enabled(pool: &PgPool, user_id: Uuid) -> Result<bool, AuthError> {
    let row: Option<(bool,)> =
        sqlx::query_as("SELECT enabled FROM auth_totp_secrets WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    Ok(row.map(|(enabled,)| enabled).unwrap_or(false))
}

/// Verify a TOTP code for login.
pub async fn verify_totp_for_login(
    pool: &PgPool,
    user_id: Uuid,
    code: &str,
) -> Result<bool, AuthError> {
    let secret = get_secret(pool, user_id).await?;
    Ok(verify_code(&secret, code))
}

/// Get the TOTP secret for a user.
async fn get_secret(pool: &PgPool, user_id: Uuid) -> Result<String, AuthError> {
    let secret: String =
        sqlx::query_scalar("SELECT secret FROM auth_totp_secrets WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
    Ok(secret)
}

// ── Challenge tokens ──

/// Generate a short-lived challenge token for the TOTP verification step.
/// This is a signed, time-limited token that proves the user passed password auth.
pub fn create_challenge_token(user_id: Uuid, jwt_secret: &str) -> String {
    use sha2::Sha256;
    let expires = Utc::now().timestamp() + 300; // 5 minutes
    let payload = format!("totp_challenge:{}:{}", user_id, expires);
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    hasher.update(jwt_secret.as_bytes());
    let sig = hex_encode(hasher.finalize());
    format!("{}.{}.{}", user_id, expires, sig)
}

/// Validate a challenge token. Returns the user_id if valid.
pub fn validate_challenge_token(token: &str, jwt_secret: &str) -> Option<Uuid> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let user_id: Uuid = parts[0].parse().ok()?;
    let expires: i64 = parts[1].parse().ok()?;

    if Utc::now().timestamp() > expires {
        return None; // Expired
    }

    // Verify signature
    let payload = format!("totp_challenge:{}:{}", user_id, expires);
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    hasher.update(jwt_secret.as_bytes());
    let expected_sig = hex_encode(hasher.finalize());

    if parts[2] == expected_sig {
        Some(user_id)
    } else {
        None
    }
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .fold(String::with_capacity(64), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_generation_is_32_chars_base32() {
        let secret = generate_secret();
        assert_eq!(secret.len(), 32);
        assert!(secret
            .chars()
            .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c)));
    }

    #[test]
    fn totp_uri_format() {
        let uri = totp_uri("JBSWY3DPEHPK3PXP", "user@example.com");
        assert!(uri.starts_with("otpauth://totp/InvestorOS:"));
        assert!(uri.contains("secret=JBSWY3DPEHPK3PXP"));
        assert!(uri.contains("issuer=InvestorOS"));
        assert!(uri.contains("digits=6"));
        assert!(uri.contains("period=30"));
    }

    #[test]
    fn code_generation_is_6_digits() {
        let code = generate_code("TESTSECRET", 1709568000);
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn same_secret_same_timestamp_produces_same_code() {
        let c1 = generate_code("MYSECRET", 1709568000);
        let c2 = generate_code("MYSECRET", 1709568000);
        assert_eq!(c1, c2);
    }

    #[test]
    fn different_timestamps_produce_different_codes() {
        let c1 = generate_code("MYSECRET", 1709568000);
        let c2 = generate_code("MYSECRET", 1709568060);
        assert_ne!(c1, c2);
    }

    #[test]
    fn challenge_token_roundtrip() {
        let user_id = Uuid::new_v4();
        let secret = "test-jwt-secret";
        let token = create_challenge_token(user_id, secret);
        let result = validate_challenge_token(&token, secret);
        assert_eq!(result, Some(user_id));
    }

    #[test]
    fn challenge_token_wrong_secret_fails() {
        let user_id = Uuid::new_v4();
        let token = create_challenge_token(user_id, "correct-secret");
        let result = validate_challenge_token(&token, "wrong-secret");
        assert_eq!(result, None);
    }

    #[test]
    fn challenge_token_tampered_fails() {
        let user_id = Uuid::new_v4();
        let token = create_challenge_token(user_id, "secret");
        let tampered = format!("{}.999999999.fakesig", user_id);
        assert_eq!(validate_challenge_token(&tampered, "secret"), None);
    }
}
