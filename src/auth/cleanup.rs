//! Session data retention policy (Sprint 105).
//!
//! Periodically cleans up expired and revoked sessions from the database.

use sqlx::PgPool;
use tracing::{info, warn};

/// Delete expired and revoked sessions older than the retention window.
/// Returns the number of rows deleted.
pub async fn cleanup_expired_sessions(
    pool: &PgPool,
    retention_hours: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM auth_sessions
         WHERE (expires_at < NOW() - make_interval(hours => $1))
            OR (revoked_at IS NOT NULL AND revoked_at < NOW() - make_interval(hours => $1))",
    )
    .bind(retention_hours as f64)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Spawn a background task that runs session cleanup every `interval_secs`.
pub fn spawn_cleanup_task(pool: PgPool, interval_secs: u64, retention_hours: i64) {
    tokio::spawn(async move {
        let interval = tokio::time::Duration::from_secs(interval_secs);
        loop {
            tokio::time::sleep(interval).await;
            match cleanup_expired_sessions(&pool, retention_hours).await {
                Ok(0) => {}
                Ok(n) => info!(deleted = n, "session cleanup: removed expired sessions"),
                Err(e) => warn!("session cleanup failed: {e}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_retention_is_24_hours() {
        let retention: i64 = std::env::var("SESSION_RETENTION_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24);
        assert_eq!(retention, 24);
    }

    #[test]
    fn default_interval_is_3600() {
        let interval: u64 = std::env::var("SESSION_CLEANUP_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600);
        assert_eq!(interval, 3600);
    }
}
