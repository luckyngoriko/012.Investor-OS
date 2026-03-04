//! Audit event logging (Sprint 104).
//!
//! Persists security-relevant events to PostgreSQL `audit_events` table.
//! Non-blocking: failures are logged via tracing but never fail the request.

use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

/// Event types for the audit log.
#[derive(Debug, Clone, Copy)]
pub enum AuditEvent {
    LoginSuccess,
    LoginFailed,
    AccountLocked,
    Logout,
    TokenRefresh,
    PasswordChanged,
    UserCreated,
    UserUpdated,
    UserDisabled,
}

impl AuditEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LoginSuccess => "login_success",
            Self::LoginFailed => "login_failed",
            Self::AccountLocked => "account_locked",
            Self::Logout => "logout",
            Self::TokenRefresh => "token_refresh",
            Self::PasswordChanged => "password_changed",
            Self::UserCreated => "user_created",
            Self::UserUpdated => "user_updated",
            Self::UserDisabled => "user_disabled",
        }
    }
}

/// Write an audit event to the database.
/// This is fire-and-forget — errors are logged but never propagated.
pub async fn log_audit_event(
    pool: &PgPool,
    event: AuditEvent,
    user_id: Option<Uuid>,
    client_ip: Option<&str>,
    details: serde_json::Value,
) {
    let result = sqlx::query(
        "INSERT INTO audit_events (event_type, user_id, client_ip, details)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(event.as_str())
    .bind(user_id)
    .bind(client_ip)
    .bind(&details)
    .execute(pool)
    .await;

    if let Err(e) = result {
        warn!(
            event_type = event.as_str(),
            "audit log write failed (non-fatal): {e}"
        );
    }
}

/// Query recent audit events (admin use).
pub async fn recent_events(pool: &PgPool, limit: i64) -> Result<Vec<AuditEventRow>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, event_type, user_id, client_ip, details, created_at
         FROM audit_events
         ORDER BY created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(AuditEventRow {
                id: r.try_get("id")?,
                event_type: r.try_get("event_type")?,
                user_id: r.try_get("user_id")?,
                client_ip: r.try_get("client_ip")?,
                details: r.try_get("details")?,
                created_at: r.try_get("created_at")?,
            })
        })
        .collect()
}

#[derive(Debug, serde::Serialize)]
pub struct AuditEventRow {
    pub id: i64,
    pub event_type: String,
    pub user_id: Option<Uuid>,
    pub client_ip: Option<String>,
    pub details: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_strings() {
        assert_eq!(AuditEvent::LoginSuccess.as_str(), "login_success");
        assert_eq!(AuditEvent::LoginFailed.as_str(), "login_failed");
        assert_eq!(AuditEvent::AccountLocked.as_str(), "account_locked");
        assert_eq!(AuditEvent::Logout.as_str(), "logout");
        assert_eq!(AuditEvent::PasswordChanged.as_str(), "password_changed");
        assert_eq!(AuditEvent::UserCreated.as_str(), "user_created");
    }
}
