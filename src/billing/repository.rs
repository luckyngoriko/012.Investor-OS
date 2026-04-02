//! Subscription persistence — read/write user tier data from PostgreSQL.
//!
//! Uses runtime `sqlx::query()` + `Row::try_get()` (no compile-time macros).

use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::tiers::Tier;

/// Billing repository errors.
#[derive(Debug, thiserror::Error)]
pub enum BillingError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
}

/// Optional Stripe identifiers to store alongside the tier.
#[derive(Debug, Default)]
pub struct StripeIds {
    pub customer_id: Option<String>,
    pub subscription_id: Option<String>,
}

/// Retrieve the subscription tier for a user.
///
/// Returns [`Tier::Free`] if no subscription row exists.
pub async fn get_user_tier(pool: &PgPool, user_id: Uuid) -> Result<Tier, BillingError> {
    let row = sqlx::query(
        r#"
        SELECT tier
        FROM subscriptions
        WHERE user_id = $1 AND status IN ('active', 'trialing')
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let tier_str: String = r.try_get("tier")?;
            Ok(Tier::from_str(&tier_str))
        }
        None => Ok(Tier::Free),
    }
}

/// Upsert a subscription record for a user.
///
/// Creates a new row or updates the existing one (keyed on `user_id` UNIQUE).
pub async fn set_user_tier(
    pool: &PgPool,
    user_id: Uuid,
    tier: &Tier,
    stripe: Option<&StripeIds>,
) -> Result<(), BillingError> {
    let tier_str = tier.as_str();
    let customer_id = stripe.and_then(|s| s.customer_id.as_deref());
    let subscription_id = stripe.and_then(|s| s.subscription_id.as_deref());

    sqlx::query(
        r#"
        INSERT INTO subscriptions
            (user_id, tier, stripe_customer_id, stripe_subscription_id, status)
        VALUES ($1, $2, $3, $4, 'active')
        ON CONFLICT (user_id)
        DO UPDATE SET
            tier                    = EXCLUDED.tier,
            stripe_customer_id      = COALESCE(EXCLUDED.stripe_customer_id, subscriptions.stripe_customer_id),
            stripe_subscription_id  = COALESCE(EXCLUDED.stripe_subscription_id, subscriptions.stripe_subscription_id),
            status                  = 'active',
            updated_at              = NOW()
        "#,
    )
    .bind(user_id)
    .bind(tier_str)
    .bind(customer_id)
    .bind(subscription_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Retrieve the full subscription status for a user (tier + status + period end).
///
/// Returns `None` if no subscription row exists.
pub async fn get_subscription_status(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<SubscriptionInfo>, BillingError> {
    let row = sqlx::query(
        r#"
        SELECT tier, status, current_period_end, stripe_customer_id, stripe_subscription_id
        FROM subscriptions
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            let tier_str: String = r.try_get("tier")?;
            let status: String = r.try_get("status")?;
            let period_end: Option<chrono::DateTime<chrono::Utc>> =
                r.try_get("current_period_end")?;
            let stripe_customer_id: Option<String> = r.try_get("stripe_customer_id")?;
            let stripe_subscription_id: Option<String> = r.try_get("stripe_subscription_id")?;

            Ok(Some(SubscriptionInfo {
                tier: Tier::from_str(&tier_str),
                status,
                current_period_end: period_end,
                stripe_customer_id,
                stripe_subscription_id,
            }))
        }
        None => Ok(None),
    }
}

/// Full subscription info returned by [`get_subscription_status`].
#[derive(Debug)]
pub struct SubscriptionInfo {
    pub tier: Tier,
    pub status: String,
    pub current_period_end: Option<chrono::DateTime<chrono::Utc>>,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
}
