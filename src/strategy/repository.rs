//! Database operations for the Strategy Engine (Wave 1b Task 5).
//!
//! Uses runtime sqlx queries (no compile-time macros) to match project convention.

use sqlx::PgPool;
use uuid::Uuid;

use super::types::{CreateStrategyRequest, Strategy, UpdateStrategyRequest};

/// Insert a new strategy into `user_strategies`.
pub async fn create_strategy(
    pool: &PgPool,
    user_id: Uuid,
    req: &CreateStrategyRequest,
) -> Result<Strategy, String> {
    let id = Uuid::new_v4();
    let mode = req.mode.as_deref().unwrap_or("signal");
    let risk_limits = req.risk_limits.clone().unwrap_or_else(|| {
        serde_json::json!({
            "max_position_pct": 5,
            "max_daily_loss_pct": 2,
            "max_drawdown_pct": 10
        })
    });

    sqlx::query(
        r#"
        INSERT INTO user_strategies (id, user_id, name, symbols, models, mode, risk_limits)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&req.name)
    .bind(&req.symbols)
    .bind(&req.models)
    .bind(mode)
    .bind(&risk_limits)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(Strategy {
        id,
        user_id,
        name: req.name.clone(),
        symbols: req.symbols.clone(),
        models: req.models.clone(),
        mode: mode.to_string(),
        risk_limits,
        is_active: true,
    })
}

/// List all strategies for a given user.
pub async fn get_strategies(pool: &PgPool, user_id: Uuid) -> Result<Vec<Strategy>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, name, symbols, models, mode, risk_limits, is_active
        FROM user_strategies
        WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    let strategies = rows
        .iter()
        .map(|r| Strategy {
            id: r.try_get::<Uuid, _>("id").unwrap_or_default(),
            user_id: r.try_get::<Uuid, _>("user_id").unwrap_or_default(),
            name: r.try_get::<String, _>("name").unwrap_or_default(),
            symbols: r.try_get::<Vec<String>, _>("symbols").unwrap_or_default(),
            models: r.try_get::<Vec<String>, _>("models").unwrap_or_default(),
            mode: r.try_get::<String, _>("mode").unwrap_or_default(),
            risk_limits: r
                .try_get::<serde_json::Value, _>("risk_limits")
                .unwrap_or_default(),
            is_active: r.try_get::<bool, _>("is_active").unwrap_or(true),
        })
        .collect();

    Ok(strategies)
}

/// Get a single strategy by ID, scoped to a user.
pub async fn get_strategy(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<Strategy>, String> {
    let row = sqlx::query(
        r#"
        SELECT id, user_id, name, symbols, models, mode, risk_limits, is_active
        FROM user_strategies
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    Ok(row.map(|r| Strategy {
        id: r.try_get::<Uuid, _>("id").unwrap_or_default(),
        user_id: r.try_get::<Uuid, _>("user_id").unwrap_or_default(),
        name: r.try_get::<String, _>("name").unwrap_or_default(),
        symbols: r.try_get::<Vec<String>, _>("symbols").unwrap_or_default(),
        models: r.try_get::<Vec<String>, _>("models").unwrap_or_default(),
        mode: r.try_get::<String, _>("mode").unwrap_or_default(),
        risk_limits: r
            .try_get::<serde_json::Value, _>("risk_limits")
            .unwrap_or_default(),
        is_active: r.try_get::<bool, _>("is_active").unwrap_or(true),
    }))
}

/// Update a strategy by ID, scoped to a user. Returns None if not found.
pub async fn update_strategy(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    req: &UpdateStrategyRequest,
) -> Result<Option<Strategy>, String> {
    // Build dynamic UPDATE with COALESCE to keep existing values for unset fields
    let result = sqlx::query(
        r#"
        UPDATE user_strategies SET
            name       = COALESCE($3, name),
            symbols    = COALESCE($4, symbols),
            models     = COALESCE($5, models),
            mode       = COALESCE($6, mode),
            risk_limits = COALESCE($7, risk_limits),
            is_active  = COALESCE($8, is_active),
            updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&req.name)
    .bind(&req.symbols)
    .bind(&req.models)
    .bind(&req.mode)
    .bind(&req.risk_limits)
    .bind(req.is_active)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }

    // Re-fetch the updated row
    get_strategy(pool, id, user_id).await
}

/// Delete a strategy by ID, scoped to a user. Returns true if deleted.
pub async fn delete_strategy(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, String> {
    let result = sqlx::query(
        r#"
        DELETE FROM user_strategies
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(result.rows_affected() > 0)
}
