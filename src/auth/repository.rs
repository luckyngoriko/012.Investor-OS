use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::auth::error::AuthError;
use crate::auth::types::AuthUserRow;

// ── User queries ──

/// Map a sqlx Row to AuthUserRow (runtime, no compile-time DB check needed).
fn map_user_row(row: sqlx::postgres::PgRow) -> Result<AuthUserRow, sqlx::Error> {
    Ok(AuthUserRow {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        name: row.try_get("name")?,
        password_hash: row.try_get("password_hash")?,
        role: row.try_get("role")?,
        permissions: row.try_get("permissions")?,
        is_active: row.try_get("is_active")?,
        failed_login_attempts: row.try_get("failed_login_attempts")?,
        locked_until: row.try_get("locked_until")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_login_at: row.try_get("last_login_at")?,
    })
}

pub async fn find_user_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<AuthUserRow>, AuthError> {
    let row = sqlx::query(
        "SELECT id, email, name, password_hash, role,
                permissions, is_active, failed_login_attempts, locked_until,
                created_at, updated_at, last_login_at
         FROM auth_users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(map_user_row(r)?)),
        None => Ok(None),
    }
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<AuthUserRow>, AuthError> {
    let row = sqlx::query(
        "SELECT id, email, name, password_hash, role,
                permissions, is_active, failed_login_attempts, locked_until,
                created_at, updated_at, last_login_at
         FROM auth_users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(map_user_row(r)?)),
        None => Ok(None),
    }
}

pub async fn list_users(pool: &PgPool) -> Result<Vec<AuthUserRow>, AuthError> {
    let rows = sqlx::query(
        "SELECT id, email, name, password_hash, role,
                permissions, is_active, failed_login_attempts, locked_until,
                created_at, updated_at, last_login_at
         FROM auth_users ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| map_user_row(r).map_err(AuthError::Database))
        .collect()
}

pub async fn insert_user(
    pool: &PgPool,
    email: &str,
    name: &str,
    password_hash: &str,
    role: &str,
    permissions: &serde_json::Value,
) -> Result<AuthUserRow, AuthError> {
    let row = sqlx::query(
        "INSERT INTO auth_users (email, name, password_hash, role, permissions)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, email, name, password_hash, role,
                   permissions, is_active, failed_login_attempts, locked_until,
                   created_at, updated_at, last_login_at",
    )
    .bind(email)
    .bind(name)
    .bind(password_hash)
    .bind(role)
    .bind(permissions)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.constraint() == Some("auth_users_email_key") {
                return AuthError::DuplicateEmail(email.to_string());
            }
        }
        AuthError::Database(e)
    })?;

    Ok(map_user_row(row)?)
}

pub async fn update_user(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    role: Option<&str>,
    permissions: Option<&serde_json::Value>,
    is_active: Option<bool>,
    password_hash: Option<&str>,
) -> Result<Option<AuthUserRow>, AuthError> {
    let row = sqlx::query(
        "UPDATE auth_users SET
            name = COALESCE($2, name),
            role = COALESCE($3, role),
            permissions = COALESCE($4, permissions),
            is_active = COALESCE($5, is_active),
            password_hash = COALESCE($6, password_hash),
            updated_at = NOW()
         WHERE id = $1
         RETURNING id, email, name, password_hash, role,
                   permissions, is_active, failed_login_attempts, locked_until,
                   created_at, updated_at, last_login_at",
    )
    .bind(id)
    .bind(name)
    .bind(role)
    .bind(permissions)
    .bind(is_active)
    .bind(password_hash)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(map_user_row(r)?)),
        None => Ok(None),
    }
}

pub async fn update_last_login(pool: &PgPool, id: Uuid) -> Result<(), AuthError> {
    sqlx::query("UPDATE auth_users SET last_login_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn count_users(pool: &PgPool) -> Result<i64, AuthError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM auth_users")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// ── Lockout queries ──

pub async fn increment_failed_logins(pool: &PgPool, id: Uuid) -> Result<i32, AuthError> {
    let row: (i32,) = sqlx::query_as(
        "UPDATE auth_users SET failed_login_attempts = failed_login_attempts + 1, updated_at = NOW()
         WHERE id = $1
         RETURNING failed_login_attempts",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn lock_user(
    pool: &PgPool,
    id: Uuid,
    locked_until: DateTime<Utc>,
) -> Result<(), AuthError> {
    sqlx::query("UPDATE auth_users SET locked_until = $2, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .bind(locked_until)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reset_failed_logins(pool: &PgPool, id: Uuid) -> Result<(), AuthError> {
    sqlx::query(
        "UPDATE auth_users SET failed_login_attempts = 0, locked_until = NULL, updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Session queries ──

pub async fn insert_session(
    pool: &PgPool,
    user_id: Uuid,
    refresh_token_hash: &str,
    client_ip: Option<&str>,
    user_agent: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<Uuid, AuthError> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO auth_sessions (user_id, refresh_token_hash, client_ip, user_agent, expires_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(user_id)
    .bind(refresh_token_hash)
    .bind(client_ip)
    .bind(user_agent)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Find a non-expired, non-revoked session by refresh token hash.
pub async fn find_session_by_refresh_hash(
    pool: &PgPool,
    refresh_token_hash: &str,
) -> Result<Option<SessionRow>, AuthError> {
    let row = sqlx::query(
        "SELECT id, user_id, refresh_token_hash, client_ip, user_agent,
                expires_at, created_at, revoked_at
         FROM auth_sessions
         WHERE refresh_token_hash = $1
           AND revoked_at IS NULL
           AND expires_at > NOW()",
    )
    .bind(refresh_token_hash)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(map_session_row(r)?)),
        None => Ok(None),
    }
}

/// Soft-revoke a session (set revoked_at).
pub async fn revoke_session(pool: &PgPool, session_id: Uuid) -> Result<(), AuthError> {
    sqlx::query("UPDATE auth_sessions SET revoked_at = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Revoke all sessions for a given user.
pub async fn revoke_all_sessions_for_user(pool: &PgPool, user_id: Uuid) -> Result<u64, AuthError> {
    let result = sqlx::query(
        "UPDATE auth_sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

fn map_session_row(row: sqlx::postgres::PgRow) -> Result<SessionRow, sqlx::Error> {
    Ok(SessionRow {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        refresh_token_hash: row.try_get("refresh_token_hash")?,
        client_ip: row.try_get("client_ip")?,
        user_agent: row.try_get("user_agent")?,
        expires_at: row.try_get("expires_at")?,
        created_at: row.try_get("created_at")?,
        revoked_at: row.try_get("revoked_at")?,
    })
}

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub refresh_token_hash: String,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
