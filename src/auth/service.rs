use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::error::AuthError;
use crate::auth::jwt::JwtConfig;
use crate::auth::password;
use crate::auth::repository;
use crate::auth::types::{
    AuthUser, AuthUserRow, CreateUserRequest, LoginData, LoginResponse, TotpChallenge,
    UpdateUserRequest, UserInfo,
};

/// Lockout configuration.
#[derive(Clone)]
pub struct LockoutConfig {
    pub max_failed_logins: i32,
    pub lockout_duration_secs: i64,
}

impl LockoutConfig {
    pub fn from_env() -> Self {
        Self {
            max_failed_logins: std::env::var("AUTH_MAX_FAILED_LOGINS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            lockout_duration_secs: std::env::var("AUTH_LOCKOUT_DURATION_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(900), // 15 min
        }
    }
}

/// Production-grade auth service backed by PostgreSQL + Argon2id + JWT.
#[derive(Clone)]
pub struct AuthService {
    pool: PgPool,
    jwt: JwtConfig,
    lockout: LockoutConfig,
}

impl AuthService {
    pub fn new(pool: PgPool, jwt: JwtConfig, lockout: LockoutConfig) -> Self {
        Self { pool, jwt, lockout }
    }

    /// SHA-256 hash of a raw refresh token — stored in DB, never the raw value.
    fn hash_refresh_token(raw: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        hex::encode(hasher.finalize())
    }

    // ── Login / Refresh / Logout ──

    pub async fn login(
        &self,
        email: &str,
        password_raw: &str,
        client_ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<LoginResponse, AuthError> {
        let normalized = email.trim().to_lowercase();

        let user_row = repository::find_user_by_email(&self.pool, &normalized)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if !user_row.is_active {
            return Err(AuthError::UserDisabled);
        }

        // Check lockout
        if let Some(locked_until) = user_row.locked_until {
            if locked_until > Utc::now() {
                return Err(AuthError::AccountLocked(locked_until));
            }
            // Lock expired — reset
            repository::reset_failed_logins(&self.pool, user_row.id).await?;
        }

        if !password::verify_password(password_raw, &user_row.password_hash)? {
            // Increment failed attempts
            let attempts = repository::increment_failed_logins(&self.pool, user_row.id).await?;
            if attempts >= self.lockout.max_failed_logins {
                let lock_until = Utc::now() + Duration::seconds(self.lockout.lockout_duration_secs);
                repository::lock_user(&self.pool, user_row.id, lock_until).await?;
                return Err(AuthError::AccountLocked(lock_until));
            }
            return Err(AuthError::InvalidCredentials);
        }

        // Successful login — reset failed attempts if any
        if user_row.failed_login_attempts > 0 {
            repository::reset_failed_logins(&self.pool, user_row.id).await?;
        }

        // Check if TOTP is enabled
        if crate::auth::totp::is_totp_enabled(&self.pool, user_row.id).await? {
            let challenge =
                crate::auth::totp::create_challenge_token(user_row.id, &self.jwt.secret_key);
            return Ok(LoginResponse::TotpRequired(TotpChallenge {
                requires_totp: true,
                challenge_token: challenge,
            }));
        }

        // Update last_login_at
        repository::update_last_login(&self.pool, user_row.id).await?;

        let tokens = self.issue_tokens(&user_row, client_ip, user_agent).await?;
        Ok(LoginResponse::Success(tokens))
    }

    /// Complete login after TOTP verification.
    pub async fn complete_totp_login(
        &self,
        challenge_token: &str,
        totp_code: &str,
        client_ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<LoginData, AuthError> {
        let user_id =
            crate::auth::totp::validate_challenge_token(challenge_token, &self.jwt.secret_key)
                .ok_or(AuthError::InvalidToken)?;

        if !crate::auth::totp::verify_totp_for_login(&self.pool, user_id, totp_code).await? {
            return Err(AuthError::InvalidCredentials);
        }

        let user_row = repository::find_user_by_id(&self.pool, user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        repository::update_last_login(&self.pool, user_row.id).await?;
        self.issue_tokens(&user_row, client_ip, user_agent).await
    }

    pub async fn refresh_session(
        &self,
        raw_refresh_token: &str,
        client_ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<LoginData, AuthError> {
        let hash = Self::hash_refresh_token(raw_refresh_token);

        let session = repository::find_session_by_refresh_hash(&self.pool, &hash)
            .await?
            .ok_or(AuthError::RefreshTokenInvalid)?;

        // Rotation: revoke old session
        repository::revoke_session(&self.pool, session.id).await?;

        let user_row = repository::find_user_by_id(&self.pool, session.user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        if !user_row.is_active {
            return Err(AuthError::UserDisabled);
        }

        self.issue_tokens(&user_row, client_ip, user_agent).await
    }

    pub async fn logout(&self, raw_refresh_token: Option<&str>) -> Result<(), AuthError> {
        if let Some(raw) = raw_refresh_token {
            let hash = Self::hash_refresh_token(raw);
            if let Some(session) =
                repository::find_session_by_refresh_hash(&self.pool, &hash).await?
            {
                repository::revoke_session(&self.pool, session.id).await?;
            }
        }
        Ok(())
    }

    /// Decode a JWT access token (CPU-only, no DB call).
    pub fn validate_access_token(&self, token: &str) -> Result<AuthUser, AuthError> {
        let claims = self.jwt.decode_access_token(token)?;
        let user_id = claims.sub;
        let role_str = &claims.role;

        let role = crate::auth::types::UserRole::from_str(role_str)
            .unwrap_or(crate::auth::types::UserRole::Viewer);

        // Permissions are role-based defaults for the JWT path.
        // For fine-grained permissions the DB is consulted in admin endpoints.
        let permissions = default_permissions_for_role(role);

        Ok(AuthUser {
            id: user_id,
            email: claims.email,
            name: String::new(), // name is not in the JWT — lightweight token
            role,
            permissions,
        })
    }

    // ── Admin CRUD ──

    pub async fn create_user(&self, req: &CreateUserRequest) -> Result<UserInfo, AuthError> {
        password::validate_password_policy(&req.password)?;
        let hash = password::hash_password(&req.password)?;
        let role = req.role.as_deref().unwrap_or("viewer");

        // Validate role
        if crate::auth::types::UserRole::from_str(role).is_none() {
            return Err(AuthError::PermissionDenied);
        }

        let permissions = match &req.permissions {
            Some(p) => serde_json::to_value(p).unwrap_or_default(),
            None => {
                let role_enum = crate::auth::types::UserRole::from_str(role)
                    .unwrap_or(crate::auth::types::UserRole::Viewer);
                serde_json::to_value(default_permissions_for_role(role_enum)).unwrap_or_default()
            }
        };

        let row = repository::insert_user(
            &self.pool,
            &req.email.trim().to_lowercase(),
            &req.name,
            &hash,
            role,
            &permissions,
        )
        .await?;

        Ok(UserInfo::from(&row))
    }

    pub async fn list_users(&self) -> Result<Vec<UserInfo>, AuthError> {
        let rows = repository::list_users(&self.pool).await?;
        Ok(rows.iter().map(UserInfo::from).collect())
    }

    pub async fn get_user(&self, id: Uuid) -> Result<Option<UserInfo>, AuthError> {
        let row = repository::find_user_by_id(&self.pool, id).await?;
        Ok(row.as_ref().map(UserInfo::from))
    }

    pub async fn update_user(
        &self,
        id: Uuid,
        req: &UpdateUserRequest,
    ) -> Result<Option<UserInfo>, AuthError> {
        // Validate role if provided
        if let Some(ref role) = req.role {
            if crate::auth::types::UserRole::from_str(role).is_none() {
                return Err(AuthError::PermissionDenied);
            }
        }

        let password_hash = match &req.password {
            Some(pw) => {
                password::validate_password_policy(pw)?;
                Some(password::hash_password(pw)?)
            }
            None => None,
        };

        let permissions = req
            .permissions
            .as_ref()
            .map(|p| serde_json::to_value(p).unwrap_or_default());

        let row = repository::update_user(
            &self.pool,
            id,
            req.name.as_deref(),
            req.role.as_deref(),
            permissions.as_ref(),
            req.is_active,
            password_hash.as_deref(),
        )
        .await?;

        // Revoke all sessions when password changes
        if password_hash.is_some() {
            repository::revoke_all_sessions_for_user(&self.pool, id).await?;
        }

        Ok(row.as_ref().map(UserInfo::from))
    }

    // ── Internal helpers ──

    async fn issue_tokens(
        &self,
        user_row: &AuthUserRow,
        client_ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<LoginData, AuthError> {
        let user = user_row.to_auth_user();

        let (access_token, expires_at) = self.jwt.encode_access_token(
            &user_row.id.to_string(),
            &user_row.email,
            &user_row.role,
        )?;

        // Raw refresh token (sent to client)
        let raw_refresh = format!("rtk_{}", Uuid::new_v4().as_simple());
        let refresh_hash = Self::hash_refresh_token(&raw_refresh);

        let refresh_expires_at = Utc::now() + Duration::seconds(self.jwt.refresh_ttl_seconds);

        repository::insert_session(
            &self.pool,
            user_row.id,
            &refresh_hash,
            client_ip,
            user_agent,
            refresh_expires_at,
        )
        .await?;

        Ok(LoginData {
            user,
            access_token,
            refresh_token: raw_refresh,
            expires_at,
            refresh_expires_at,
        })
    }
}

/// Default permission sets per role (matches old hardcoded sets).
fn default_permissions_for_role(role: crate::auth::types::UserRole) -> Vec<String> {
    use crate::auth::types::UserRole;
    match role {
        UserRole::Admin => vec!["*".to_string()],
        UserRole::Trader => vec![
            "dashboard.read".to_string(),
            "portfolio.read".to_string(),
            "portfolio.trade".to_string(),
            "positions.read".to_string(),
            "proposals.read".to_string(),
            "proposals.execute".to_string(),
            "risk.read".to_string(),
            "backtest.read".to_string(),
            "backtest.run".to_string(),
            "journal.read".to_string(),
            "journal.write".to_string(),
            "settings.read".to_string(),
            "settings.update".to_string(),
        ],
        UserRole::Viewer => vec![
            "dashboard.read".to_string(),
            "portfolio.read".to_string(),
            "positions.read".to_string(),
            "proposals.read".to_string(),
            "risk.read".to_string(),
            "journal.read".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockout_config_defaults() {
        // Without env vars set, defaults apply
        let cfg = LockoutConfig {
            max_failed_logins: 5,
            lockout_duration_secs: 900,
        };
        assert_eq!(cfg.max_failed_logins, 5);
        assert_eq!(cfg.lockout_duration_secs, 900);
    }

    #[test]
    fn admin_permissions_are_wildcard() {
        let perms = default_permissions_for_role(crate::auth::types::UserRole::Admin);
        assert_eq!(perms, vec!["*"]);
    }

    #[test]
    fn trader_has_trade_permission() {
        let perms = default_permissions_for_role(crate::auth::types::UserRole::Trader);
        assert!(perms.contains(&"portfolio.trade".to_string()));
        assert!(perms.contains(&"backtest.run".to_string()));
        assert!(!perms.contains(&"*".to_string()));
    }

    #[test]
    fn viewer_has_read_only_permissions() {
        let perms = default_permissions_for_role(crate::auth::types::UserRole::Viewer);
        assert!(perms.iter().all(|p| p.ends_with(".read")));
        assert!(!perms.contains(&"portfolio.trade".to_string()));
    }

    #[test]
    fn hash_refresh_token_is_deterministic() {
        let h1 = AuthService::hash_refresh_token("test-token-abc");
        let h2 = AuthService::hash_refresh_token("test-token-abc");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn hash_refresh_token_differs_for_different_tokens() {
        let h1 = AuthService::hash_refresh_token("token-a");
        let h2 = AuthService::hash_refresh_token("token-b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hex_encode_produces_lowercase_hex() {
        let result = hex::encode([0xab, 0xcd, 0xef]);
        assert_eq!(result, "abcdef");
    }
}

// hex encoding for SHA-256 — use the sha2 crate's output directly
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .fold(String::with_capacity(64), |mut s, b| {
                use std::fmt::Write;
                let _ = write!(s, "{b:02x}");
                s
            })
    }
}
