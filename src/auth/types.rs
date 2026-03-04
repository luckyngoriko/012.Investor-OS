use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User roles — maps to the CHECK constraint in auth_users.role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Trader,
    Viewer,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Trader => "trader",
            Self::Viewer => "viewer",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Self::Admin),
            "trader" => Some(Self::Trader),
            "viewer" => Some(Self::Viewer),
            _ => None,
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Authenticated user — returned to handlers via request extensions.
/// Kept identical to the old AuthUser so the API contract doesn't break.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub permissions: Vec<String>,
}

/// Full user row from auth_users (never exposed to API directly).
#[derive(Debug, Clone)]
pub struct AuthUserRow {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role: String,
    pub permissions: serde_json::Value,
    pub is_active: bool,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl AuthUserRow {
    pub fn to_auth_user(&self) -> AuthUser {
        let permissions: Vec<String> =
            serde_json::from_value(self.permissions.clone()).unwrap_or_default();
        AuthUser {
            id: self.id.to_string(),
            email: self.email.clone(),
            name: self.name.clone(),
            role: UserRole::from_str(&self.role).unwrap_or(UserRole::Viewer),
            permissions,
        }
    }
}

/// JWT access-token claims.
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    /// Subject — user UUID
    pub sub: String,
    pub email: String,
    pub role: String,
    /// Issued at (epoch seconds)
    pub iat: i64,
    /// Expiration (epoch seconds)
    pub exp: i64,
}

/// Login response — either full tokens or a TOTP challenge.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum LoginResponse {
    Success(LoginData),
    TotpRequired(TotpChallenge),
}

/// TOTP challenge — sent when 2FA is required.
#[derive(Debug, Serialize)]
pub struct TotpChallenge {
    pub requires_totp: bool,
    pub challenge_token: String,
}

/// Login response data — same shape as old LoginData.
#[derive(Debug, Serialize)]
pub struct LoginData {
    pub user: AuthUser,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

// -- Request / response types for handlers --

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: Option<String>,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct TotpVerifyRequest {
    pub challenge_token: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct TotpCodeRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Option<Vec<String>>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub role: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub password: Option<String>,
}

/// Public user info returned by admin endpoints.
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl From<&AuthUserRow> for UserInfo {
    fn from(row: &AuthUserRow) -> Self {
        let permissions: Vec<String> =
            serde_json::from_value(row.permissions.clone()).unwrap_or_default();
        Self {
            id: row.id.to_string(),
            email: row.email.clone(),
            name: row.name.clone(),
            role: row.role.clone(),
            permissions,
            is_active: row.is_active,
            created_at: row.created_at,
            last_login_at: row.last_login_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_role_roundtrip() {
        for role in [UserRole::Admin, UserRole::Trader, UserRole::Viewer] {
            let s = role.as_str();
            let parsed = UserRole::from_str(s);
            assert_eq!(parsed, Some(role));
        }
    }

    #[test]
    fn user_role_unknown_returns_none() {
        assert_eq!(UserRole::from_str("superadmin"), None);
        assert_eq!(UserRole::from_str(""), None);
    }

    #[test]
    fn user_role_display() {
        assert_eq!(UserRole::Admin.to_string(), "admin");
        assert_eq!(UserRole::Trader.to_string(), "trader");
        assert_eq!(UserRole::Viewer.to_string(), "viewer");
    }

    #[test]
    fn auth_user_row_to_auth_user() {
        let row = AuthUserRow {
            id: uuid::Uuid::new_v4(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password_hash: "hash".to_string(),
            role: "trader".to_string(),
            permissions: serde_json::json!(["dashboard.read", "portfolio.trade"]),
            is_active: true,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
        };

        let user = row.to_auth_user();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, UserRole::Trader);
        assert_eq!(user.permissions.len(), 2);
        assert!(user.permissions.contains(&"portfolio.trade".to_string()));
    }

    #[test]
    fn auth_user_row_unknown_role_defaults_to_viewer() {
        let row = AuthUserRow {
            id: uuid::Uuid::new_v4(),
            email: "x@x.com".to_string(),
            name: "X".to_string(),
            password_hash: "h".to_string(),
            role: "unknown_role".to_string(),
            permissions: serde_json::json!([]),
            is_active: true,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login_at: None,
        };

        assert_eq!(row.to_auth_user().role, UserRole::Viewer);
    }

    #[test]
    fn user_info_from_row() {
        let id = uuid::Uuid::new_v4();
        let now = Utc::now();
        let row = AuthUserRow {
            id,
            email: "info@test.com".to_string(),
            name: "Info Test".to_string(),
            password_hash: "hash".to_string(),
            role: "admin".to_string(),
            permissions: serde_json::json!(["*"]),
            is_active: true,
            failed_login_attempts: 0,
            locked_until: None,
            created_at: now,
            updated_at: now,
            last_login_at: Some(now),
        };

        let info = UserInfo::from(&row);
        assert_eq!(info.id, id.to_string());
        assert_eq!(info.role, "admin");
        assert!(info.is_active);
        assert_eq!(info.permissions, vec!["*"]);
    }

    #[test]
    fn login_response_success_serializes() {
        let user = AuthUser {
            id: "abc".to_string(),
            email: "a@b.com".to_string(),
            name: "A".to_string(),
            role: UserRole::Viewer,
            permissions: vec![],
        };
        let data = LoginData {
            user,
            access_token: "tok".to_string(),
            refresh_token: "ref".to_string(),
            expires_at: Utc::now(),
            refresh_expires_at: Utc::now(),
        };
        let resp = LoginResponse::Success(data);
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("access_token").is_some());
    }

    #[test]
    fn login_response_totp_serializes() {
        let resp = LoginResponse::TotpRequired(TotpChallenge {
            requires_totp: true,
            challenge_token: "ct".to_string(),
        });
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["requires_totp"], true);
        assert_eq!(json["challenge_token"], "ct");
    }
}
