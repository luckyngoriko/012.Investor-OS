use sqlx::PgPool;
use tracing::info;

use crate::auth::error::AuthError;
use crate::auth::password;
use crate::auth::repository;

/// Seed a default admin user if the auth_users table is empty.
/// Called once at startup — safe to re-run (idempotent).
pub async fn seed_admin_if_empty(pool: &PgPool) -> Result<(), AuthError> {
    let count = repository::count_users(pool).await?;
    if count > 0 {
        info!("auth: {} user(s) found — skipping seed", count);
        return Ok(());
    }

    let admin_email =
        std::env::var("AUTH_ADMIN_EMAIL").unwrap_or_else(|_| "admin@investor-os.com".to_string());
    let admin_password = std::env::var("AUTH_ADMIN_PASSWORD")
        .expect("AUTH_ADMIN_PASSWORD must be set for initial admin seed");
    let admin_name = std::env::var("AUTH_ADMIN_NAME").unwrap_or_else(|_| "Admin User".to_string());

    let hash = password::hash_password(&admin_password)?;
    let permissions = serde_json::json!(["*"]);

    repository::insert_user(
        pool,
        &admin_email,
        &admin_name,
        &hash,
        "admin",
        &permissions,
    )
    .await?;

    info!("auth: seeded initial admin user ({})", admin_email);
    Ok(())
}
