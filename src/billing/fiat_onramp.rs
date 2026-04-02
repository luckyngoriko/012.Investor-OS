//! Fiat On-Ramp via Stripe Connect (Wave 4 Task 24).
//!
//! Provides deposit / withdrawal flows against the Stripe API and
//! persists transaction state in the `fiat_transactions` table.
//! Uses runtime `sqlx::query()` + `Row::try_get()` (no compile-time macros).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

// ───────────────── Types ─────────────────

/// Request body for `POST /api/fiat/deposit`.
#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    pub amount_cents: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
}

/// Request body for `POST /api/fiat/withdraw`.
#[derive(Debug, Deserialize)]
pub struct WithdrawalRequest {
    pub amount_cents: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "USD".to_string()
}

/// Persisted fiat transaction record.
#[derive(Debug, Serialize)]
pub struct FiatTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tx_type: String,
    pub amount_cents: i64,
    pub currency: String,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_payout_id: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stripe payment intent status (subset relevant to our flow).
#[derive(Debug, Serialize)]
pub struct PaymentStatus {
    pub id: String,
    pub status: String,
    pub amount: i64,
    pub currency: String,
}

/// Domain errors for fiat on-ramp operations.
#[derive(Debug, thiserror::Error)]
pub enum FiatError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Stripe API error: {0}")]
    Stripe(String),
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
}

// ───────────────── Stripe Connect Client ─────────────────

/// HTTP client for Stripe Connect API v1.
#[derive(Debug, Clone)]
pub struct StripeConnectClient {
    http: reqwest::Client,
    secret_key: String,
    base_url: String,
}

impl StripeConnectClient {
    /// Create a new client.  `secret_key` is the Stripe secret key (`sk_...`).
    pub fn new(secret_key: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            secret_key: secret_key.to_string(),
            base_url: "https://api.stripe.com/v1".to_string(),
        }
    }

    /// Build from env var `STRIPE_SECRET_KEY`.
    /// Returns `None` if the var is unset/empty (graceful degradation).
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("STRIPE_SECRET_KEY").ok()?;
        if key.is_empty() {
            return None;
        }
        Some(Self::new(&key))
    }

    /// Create a Stripe PaymentIntent for a deposit.
    pub async fn create_payment_intent(
        &self,
        user_id: Uuid,
        amount_cents: i64,
        currency: &str,
    ) -> Result<PaymentStatus, FiatError> {
        if amount_cents <= 0 {
            return Err(FiatError::InvalidAmount(
                "amount_cents must be positive".to_string(),
            ));
        }

        let params = [
            ("amount", amount_cents.to_string()),
            ("currency", currency.to_lowercase()),
            ("metadata[user_id]", user_id.to_string()),
            ("automatic_payment_methods[enabled]", "true".to_string()),
        ];

        let resp = self
            .http
            .post(format!("{}/payment_intents", self.base_url))
            .bearer_auth(&self.secret_key)
            .form(&params)
            .send()
            .await
            .map_err(|e| FiatError::Stripe(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "unknown".to_string());
            return Err(FiatError::Stripe(format!(
                "Stripe PaymentIntent creation failed: {body}"
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| FiatError::Stripe(e.to_string()))?;

        Ok(PaymentStatus {
            id: body["id"].as_str().unwrap_or_default().to_string(),
            status: body["status"].as_str().unwrap_or_default().to_string(),
            amount: body["amount"].as_i64().unwrap_or(amount_cents),
            currency: body["currency"].as_str().unwrap_or(currency).to_string(),
        })
    }

    /// Create a Stripe Payout for a withdrawal.
    pub async fn create_payout(
        &self,
        user_id: Uuid,
        amount_cents: i64,
    ) -> Result<PaymentStatus, FiatError> {
        if amount_cents <= 0 {
            return Err(FiatError::InvalidAmount(
                "amount_cents must be positive".to_string(),
            ));
        }

        let params = [
            ("amount", amount_cents.to_string()),
            ("currency", "usd".to_string()),
            ("metadata[user_id]", user_id.to_string()),
        ];

        let resp = self
            .http
            .post(format!("{}/payouts", self.base_url))
            .bearer_auth(&self.secret_key)
            .form(&params)
            .send()
            .await
            .map_err(|e| FiatError::Stripe(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "unknown".to_string());
            return Err(FiatError::Stripe(format!(
                "Stripe Payout creation failed: {body}"
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| FiatError::Stripe(e.to_string()))?;

        Ok(PaymentStatus {
            id: body["id"].as_str().unwrap_or_default().to_string(),
            status: body["status"].as_str().unwrap_or_default().to_string(),
            amount: body["amount"].as_i64().unwrap_or(amount_cents),
            currency: body["currency"].as_str().unwrap_or("usd").to_string(),
        })
    }

    /// Retrieve current status of a payment intent.
    pub async fn get_payment_status(
        &self,
        payment_intent_id: &str,
    ) -> Result<PaymentStatus, FiatError> {
        let resp = self
            .http
            .get(format!(
                "{}/payment_intents/{payment_intent_id}",
                self.base_url
            ))
            .bearer_auth(&self.secret_key)
            .send()
            .await
            .map_err(|e| FiatError::Stripe(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "unknown".to_string());
            return Err(FiatError::Stripe(format!(
                "Stripe PaymentIntent retrieval failed: {body}"
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| FiatError::Stripe(e.to_string()))?;

        Ok(PaymentStatus {
            id: body["id"].as_str().unwrap_or_default().to_string(),
            status: body["status"].as_str().unwrap_or_default().to_string(),
            amount: body["amount"].as_i64().unwrap_or(0),
            currency: body["currency"].as_str().unwrap_or("usd").to_string(),
        })
    }
}

// ───────────────── Database persistence ─────────────────

/// Insert a new fiat transaction row.
pub async fn insert_transaction(
    pool: &PgPool,
    user_id: Uuid,
    tx_type: &str,
    amount_cents: i64,
    currency: &str,
    stripe_pi_id: Option<&str>,
    stripe_payout_id: Option<&str>,
    status: &str,
) -> Result<FiatTransaction, FiatError> {
    let row = sqlx::query(
        r#"
        INSERT INTO fiat_transactions
            (user_id, tx_type, amount_cents, currency,
             stripe_payment_intent_id, stripe_payout_id, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, user_id, tx_type, amount_cents, currency,
                  stripe_payment_intent_id, stripe_payout_id, status,
                  created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(tx_type)
    .bind(amount_cents)
    .bind(currency)
    .bind(stripe_pi_id)
    .bind(stripe_payout_id)
    .bind(status)
    .fetch_one(pool)
    .await?;

    Ok(row_to_tx(&row))
}

/// List fiat transactions for a user, newest first.
pub async fn list_transactions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<FiatTransaction>, FiatError> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, tx_type, amount_cents, currency,
               stripe_payment_intent_id, stripe_payout_id, status,
               created_at, updated_at
        FROM fiat_transactions
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(row_to_tx).collect())
}

/// Update transaction status (used by webhook handler).
pub async fn update_transaction_status(
    pool: &PgPool,
    stripe_payment_intent_id: &str,
    new_status: &str,
) -> Result<Option<FiatTransaction>, FiatError> {
    let row = sqlx::query(
        r#"
        UPDATE fiat_transactions
        SET status = $1, updated_at = NOW()
        WHERE stripe_payment_intent_id = $2
        RETURNING id, user_id, tx_type, amount_cents, currency,
                  stripe_payment_intent_id, stripe_payout_id, status,
                  created_at, updated_at
        "#,
    )
    .bind(new_status)
    .bind(stripe_payment_intent_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.as_ref().map(row_to_tx))
}

fn row_to_tx(row: &sqlx::postgres::PgRow) -> FiatTransaction {
    FiatTransaction {
        id: row.try_get("id").unwrap_or_default(),
        user_id: row.try_get("user_id").unwrap_or_default(),
        tx_type: row.try_get("tx_type").unwrap_or_default(),
        amount_cents: row.try_get("amount_cents").unwrap_or_default(),
        currency: row.try_get("currency").unwrap_or_default(),
        stripe_payment_intent_id: row.try_get("stripe_payment_intent_id").ok(),
        stripe_payout_id: row.try_get("stripe_payout_id").ok(),
        status: row.try_get("status").unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

// ───────────────── Tests ─────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stripe_client_new() {
        let client = StripeConnectClient::new("sk_test_123");
        assert_eq!(client.secret_key, "sk_test_123");
        assert_eq!(client.base_url, "https://api.stripe.com/v1");
    }

    #[test]
    fn test_stripe_client_from_env_missing() {
        // When STRIPE_SECRET_KEY is not set, returns None
        std::env::remove_var("STRIPE_SECRET_KEY");
        let client = StripeConnectClient::from_env();
        assert!(client.is_none());
    }

    #[test]
    fn test_deposit_request_defaults() {
        let json_str = r#"{"amount_cents": 5000}"#;
        let req: DepositRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.amount_cents, 5000);
        assert_eq!(req.currency, "USD");
    }

    #[test]
    fn test_deposit_request_with_currency() {
        let json_str = r#"{"amount_cents": 10000, "currency": "EUR"}"#;
        let req: DepositRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.amount_cents, 10000);
        assert_eq!(req.currency, "EUR");
    }

    #[test]
    fn test_withdrawal_request_defaults() {
        let json_str = r#"{"amount_cents": 2500}"#;
        let req: WithdrawalRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.amount_cents, 2500);
        assert_eq!(req.currency, "USD");
    }

    #[test]
    fn test_withdrawal_request_with_currency() {
        let json_str = r#"{"amount_cents": 7500, "currency": "GBP"}"#;
        let req: WithdrawalRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.amount_cents, 7500);
        assert_eq!(req.currency, "GBP");
    }

    #[test]
    fn test_fiat_transaction_serialize() {
        let tx = FiatTransaction {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            tx_type: "deposit".to_string(),
            amount_cents: 5000,
            currency: "USD".to_string(),
            stripe_payment_intent_id: Some("pi_test_123".to_string()),
            stripe_payout_id: None,
            status: "pending".to_string(),
            created_at: DateTime::UNIX_EPOCH,
            updated_at: DateTime::UNIX_EPOCH,
        };
        let json = serde_json::to_value(&tx).unwrap();
        assert_eq!(json["tx_type"], "deposit");
        assert_eq!(json["amount_cents"], 5000);
        assert_eq!(json["currency"], "USD");
        assert_eq!(json["stripe_payment_intent_id"], "pi_test_123");
        assert!(json["stripe_payout_id"].is_null());
        assert_eq!(json["status"], "pending");
    }

    #[test]
    fn test_payment_status_serialize() {
        let ps = PaymentStatus {
            id: "pi_abc123".to_string(),
            status: "succeeded".to_string(),
            amount: 10000,
            currency: "usd".to_string(),
        };
        let json = serde_json::to_value(&ps).unwrap();
        assert_eq!(json["id"], "pi_abc123");
        assert_eq!(json["status"], "succeeded");
        assert_eq!(json["amount"], 10000);
        assert_eq!(json["currency"], "usd");
    }

    #[tokio::test]
    async fn test_create_payment_intent_invalid_amount() {
        let client = StripeConnectClient::new("sk_test_dummy");
        let result = client.create_payment_intent(Uuid::nil(), -100, "usd").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("amount_cents must be positive"));
    }

    #[tokio::test]
    async fn test_create_payout_invalid_amount() {
        let client = StripeConnectClient::new("sk_test_dummy");
        let result = client.create_payout(Uuid::nil(), 0).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("amount_cents must be positive"));
    }

    #[test]
    fn test_default_currency() {
        assert_eq!(default_currency(), "USD");
    }
}
