//! Repository for the Copy Trading Marketplace (Wave 3 Task 17).
//!
//! All queries use runtime sqlx (no compile-time macros).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ── Types ──────────────────────────────────────────────────────────────

/// A strategy listing visible in the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyListing {
    pub id: Uuid,
    pub creator_id: Uuid,
    pub strategy_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub performance: serde_json::Value,
    pub price_monthly_usd: i32,
    pub subscribers_count: i32,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body for publishing a strategy to the marketplace.
#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub strategy_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price_monthly_usd: Option<i32>,
}

// ── Queries ────────────────────────────────────────────────────────────

/// List all public strategy listings ordered by subscriber count descending.
pub async fn list_public_strategies(pool: &PgPool) -> Result<Vec<StrategyListing>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, creator_id, strategy_id, name, description,
               performance, price_monthly_usd, subscribers_count,
               is_public, created_at, updated_at
        FROM strategy_listings
        WHERE is_public = true
        ORDER BY subscribers_count DESC, created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to list marketplace strategies: {e}"))?;

    let mut listings = Vec::with_capacity(rows.len());
    for row in &rows {
        use sqlx::Row;
        listings.push(StrategyListing {
            id: row.try_get("id").unwrap_or_default(),
            creator_id: row.try_get("creator_id").unwrap_or_default(),
            strategy_id: row.try_get("strategy_id").unwrap_or_default(),
            name: row.try_get("name").unwrap_or_default(),
            description: row.try_get("description").unwrap_or_default(),
            performance: row
                .try_get("performance")
                .unwrap_or_else(|_| serde_json::json!({})),
            price_monthly_usd: row.try_get("price_monthly_usd").unwrap_or(0),
            subscribers_count: row.try_get("subscribers_count").unwrap_or(0),
            is_public: row.try_get("is_public").unwrap_or(false),
            created_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            updated_at: row.try_get("updated_at").unwrap_or_else(|_| Utc::now()),
        });
    }
    Ok(listings)
}

/// Publish a user's strategy to the marketplace (sets is_public = true).
pub async fn publish_strategy(
    pool: &PgPool,
    creator_id: Uuid,
    req: &PublishRequest,
) -> Result<Uuid, String> {
    let price = req.price_monthly_usd.unwrap_or(0);
    let desc = req.description.clone().unwrap_or_default();

    let row = sqlx::query(
        r#"
        INSERT INTO strategy_listings
            (creator_id, strategy_id, name, description, price_monthly_usd, is_public)
        VALUES ($1, $2, $3, $4, $5, true)
        RETURNING id
        "#,
    )
    .bind(creator_id)
    .bind(req.strategy_id)
    .bind(&req.name)
    .bind(&desc)
    .bind(price)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to publish strategy: {e}"))?;

    use sqlx::Row;
    let id: Uuid = row.try_get("id").map_err(|e| format!("Missing id: {e}"))?;
    Ok(id)
}

/// Subscribe to a marketplace listing (increments subscriber count).
pub async fn subscribe_to_strategy(
    pool: &PgPool,
    _user_id: Uuid,
    listing_id: Uuid,
) -> Result<(), String> {
    // Verify listing exists and is public
    let exists = sqlx::query("SELECT id FROM strategy_listings WHERE id = $1 AND is_public = true")
        .bind(listing_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Failed to check listing: {e}"))?;

    if exists.is_none() {
        return Err("Listing not found or not public".to_string());
    }

    sqlx::query(
        r#"
        UPDATE strategy_listings
        SET subscribers_count = subscribers_count + 1,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(listing_id)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to subscribe: {e}"))?;

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_request_deserializes() {
        let json = r#"{"strategy_id":"00000000-0000-0000-0000-000000000001","name":"My Strategy","description":"desc","price_monthly_usd":29}"#;
        let req: PublishRequest = serde_json::from_str(json).expect("valid json");
        assert_eq!(req.name, "My Strategy");
        assert_eq!(req.price_monthly_usd, Some(29));
    }

    #[test]
    fn publish_request_optional_fields() {
        let json = r#"{"strategy_id":"00000000-0000-0000-0000-000000000001","name":"Minimal"}"#;
        let req: PublishRequest = serde_json::from_str(json).expect("valid json");
        assert!(req.description.is_none());
        assert!(req.price_monthly_usd.is_none());
    }

    #[test]
    fn strategy_listing_serializes() {
        let listing = StrategyListing {
            id: Uuid::nil(),
            creator_id: Uuid::nil(),
            strategy_id: Uuid::nil(),
            name: "Test".to_string(),
            description: Some("desc".to_string()),
            performance: serde_json::json!({"sharpe": 1.5}),
            price_monthly_usd: 49,
            subscribers_count: 10,
            is_public: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&listing).expect("serializable");
        assert!(json.contains("\"name\":\"Test\""));
        assert!(json.contains("\"price_monthly_usd\":49"));
    }

    #[test]
    fn strategy_listing_roundtrip() {
        let listing = StrategyListing {
            id: Uuid::new_v4(),
            creator_id: Uuid::new_v4(),
            strategy_id: Uuid::new_v4(),
            name: "Alpha Hunter".to_string(),
            description: None,
            performance: serde_json::json!({}),
            price_monthly_usd: 0,
            subscribers_count: 0,
            is_public: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&listing).unwrap();
        let back: StrategyListing = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Alpha Hunter");
        assert_eq!(back.id, listing.id);
    }
}
