use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::auth::error::AuthError;
use crate::auth::types::AccessClaims;

/// JWT configuration — loaded once at startup.
#[derive(Clone)]
pub struct JwtConfig {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    /// Raw secret — needed for TOTP challenge token HMAC.
    pub secret_key: String,
    /// Access token lifetime in seconds.
    pub access_ttl_seconds: i64,
    /// Refresh token lifetime in seconds.
    pub refresh_ttl_seconds: i64,
}

impl JwtConfig {
    pub fn from_env() -> Self {
        let secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set — no hardcoded fallback allowed");
        assert!(
            secret.len() >= 32,
            "JWT_SECRET must be at least 32 characters"
        );

        let access_ttl_seconds = std::env::var("AUTH_ACCESS_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(15 * 60); // 15 minutes

        let refresh_ttl_seconds = std::env::var("AUTH_REFRESH_TTL_SECONDS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(7 * 24 * 60 * 60); // 7 days

        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            secret_key: secret,
            access_ttl_seconds,
            refresh_ttl_seconds,
        }
    }

    /// Create a JwtConfig from an explicit secret (for tests).
    #[cfg(test)]
    pub fn from_secret(secret: &str, access_ttl: i64, refresh_ttl: i64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            secret_key: secret.to_string(),
            access_ttl_seconds: access_ttl,
            refresh_ttl_seconds: refresh_ttl,
        }
    }

    /// Encode an access token JWT for the given user.
    pub fn encode_access_token(
        &self,
        user_id: &str,
        email: &str,
        role: &str,
    ) -> Result<(String, chrono::DateTime<Utc>), AuthError> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.access_ttl_seconds);

        let claims = AccessClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)?;
        Ok((token, exp))
    }

    /// Decode and validate an access token JWT.
    pub fn decode_access_token(&self, token: &str) -> Result<AccessClaims, AuthError> {
        let data = decode::<AccessClaims>(token, &self.decoding_key, &Validation::default())?;
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JwtConfig {
        JwtConfig::from_secret("test-secret-key-that-is-long-enough-32+", 300, 86400)
    }

    #[test]
    fn encode_decode_roundtrip() {
        let cfg = test_config();
        let (token, _exp) = cfg
            .encode_access_token("user-123", "test@example.com", "trader")
            .unwrap();

        let claims = cfg.decode_access_token(&token).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.role, "trader");
    }

    #[test]
    fn expired_token_rejected() {
        // TTL of -120s puts exp well past the default 60s leeway
        let cfg = JwtConfig::from_secret("test-secret-key-that-is-long-enough-32+", -120, 86400);
        let (token, _) = cfg
            .encode_access_token("user-123", "test@example.com", "admin")
            .unwrap();

        let result = cfg.decode_access_token(&token);
        assert!(result.is_err(), "Expired token should be rejected");
    }

    #[test]
    fn wrong_secret_rejected() {
        let cfg1 = JwtConfig::from_secret("secret-key-number-one-long-enough-32+!", 300, 86400);
        let cfg2 = JwtConfig::from_secret("secret-key-number-two-long-enough-32+!", 300, 86400);

        let (token, _) = cfg1
            .encode_access_token("user-123", "test@example.com", "viewer")
            .unwrap();

        let result = cfg2.decode_access_token(&token);
        assert!(
            result.is_err(),
            "Token signed with different secret should be rejected"
        );
    }
}
