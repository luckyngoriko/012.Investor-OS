//! Fireblocks MPC Custody Module (Wave 4 Task 22).
//!
//! Provides institutional-grade custody via Fireblocks vault accounts.
//! Supports deposit address generation, withdrawals, balance queries,
//! and transaction tracking with PostgreSQL persistence.

pub mod fireblocks;

pub use fireblocks::{
    AssetBalance, CustodyError, DepositAddress, FireblocksClient, FireblocksTransaction,
    VaultAccount, VaultAccountWithBalances,
};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ── Request / response types for API handlers ─────────────────────────

/// POST /api/custody/deposit-address request body.
#[derive(Debug, Deserialize)]
pub struct DepositAddressRequest {
    pub wallet_id: Uuid,
    pub asset: String,
}

/// POST /api/custody/withdraw request body.
#[derive(Debug, Deserialize)]
pub struct WithdrawRequest {
    pub wallet_id: Uuid,
    pub asset: String,
    pub amount: Decimal,
    pub dest_address: String,
}

/// GET /api/custody/balance query parameters.
#[derive(Debug, Deserialize)]
pub struct BalanceQuery {
    pub wallet_id: Uuid,
}

/// GET /api/custody/transactions query parameters.
#[derive(Debug, Deserialize)]
pub struct TransactionsQuery {
    pub wallet_id: Uuid,
}

/// A custody transaction record from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyTransaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub tx_type: String,
    pub asset: String,
    pub amount: Decimal,
    pub fireblocks_tx_id: Option<String>,
    pub status: String,
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A custody wallet record from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyWallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub fireblocks_vault_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

// ── Repository: wallet queries ────────────────────────────────────────

/// Look up a wallet by its primary key and verify ownership.
pub async fn get_wallet(
    pool: &PgPool,
    wallet_id: Uuid,
    user_id: Uuid,
) -> Result<Option<CustodyWallet>, String> {
    let row = sqlx::query(
        r#"
        SELECT id, user_id, fireblocks_vault_id, name, created_at
        FROM custody_wallets
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(wallet_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Failed to query wallet: {e}"))?;

    match row {
        Some(r) => {
            use sqlx::Row;
            Ok(Some(CustodyWallet {
                id: r.try_get("id").unwrap_or_default(),
                user_id: r.try_get("user_id").unwrap_or_default(),
                fireblocks_vault_id: r.try_get("fireblocks_vault_id").unwrap_or_default(),
                name: r.try_get("name").unwrap_or_default(),
                created_at: r.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            }))
        }
        None => Ok(None),
    }
}

/// Insert a new custody wallet.
pub async fn create_wallet(
    pool: &PgPool,
    user_id: Uuid,
    fireblocks_vault_id: &str,
    name: &str,
) -> Result<CustodyWallet, String> {
    let row = sqlx::query(
        r#"
        INSERT INTO custody_wallets (user_id, fireblocks_vault_id, name)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, fireblocks_vault_id, name, created_at
        "#,
    )
    .bind(user_id)
    .bind(fireblocks_vault_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to insert wallet: {e}"))?;

    use sqlx::Row;
    Ok(CustodyWallet {
        id: row.try_get("id").unwrap_or_default(),
        user_id: row.try_get("user_id").unwrap_or_default(),
        fireblocks_vault_id: row.try_get("fireblocks_vault_id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
    })
}

// ── Repository: transaction queries ───────────────────────────────────

/// Insert a new custody transaction.
pub async fn insert_transaction(
    pool: &PgPool,
    wallet_id: Uuid,
    tx_type: &str,
    asset: &str,
    amount: Decimal,
    fireblocks_tx_id: Option<&str>,
    status: &str,
) -> Result<CustodyTransaction, String> {
    let row = sqlx::query(
        r#"
        INSERT INTO custody_transactions
            (wallet_id, tx_type, asset, amount, fireblocks_tx_id, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, wallet_id, tx_type, asset, amount, fireblocks_tx_id,
                  status, tx_hash, created_at, updated_at
        "#,
    )
    .bind(wallet_id)
    .bind(tx_type)
    .bind(asset)
    .bind(amount)
    .bind(fireblocks_tx_id)
    .bind(status)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Failed to insert custody transaction: {e}"))?;

    use sqlx::Row;
    Ok(CustodyTransaction {
        id: row.try_get("id").unwrap_or_default(),
        wallet_id: row.try_get("wallet_id").unwrap_or_default(),
        tx_type: row.try_get("tx_type").unwrap_or_default(),
        asset: row.try_get("asset").unwrap_or_default(),
        amount: row.try_get("amount").unwrap_or_default(),
        fireblocks_tx_id: row.try_get("fireblocks_tx_id").unwrap_or_default(),
        status: row.try_get("status").unwrap_or_default(),
        tx_hash: row.try_get("tx_hash").unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
        updated_at: row.try_get("updated_at").unwrap_or_else(|_| Utc::now()),
    })
}

/// List transactions for a wallet.
pub async fn list_transactions(
    pool: &PgPool,
    wallet_id: Uuid,
) -> Result<Vec<CustodyTransaction>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, wallet_id, tx_type, asset, amount, fireblocks_tx_id,
               status, tx_hash, created_at, updated_at
        FROM custody_transactions
        WHERE wallet_id = $1
        ORDER BY created_at DESC
        LIMIT 200
        "#,
    )
    .bind(wallet_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to list custody transactions: {e}"))?;

    let mut txs = Vec::with_capacity(rows.len());
    for r in &rows {
        use sqlx::Row;
        txs.push(CustodyTransaction {
            id: r.try_get("id").unwrap_or_default(),
            wallet_id: r.try_get("wallet_id").unwrap_or_default(),
            tx_type: r.try_get("tx_type").unwrap_or_default(),
            asset: r.try_get("asset").unwrap_or_default(),
            amount: r.try_get("amount").unwrap_or_default(),
            fireblocks_tx_id: r.try_get("fireblocks_tx_id").unwrap_or_default(),
            status: r.try_get("status").unwrap_or_default(),
            tx_hash: r.try_get("tx_hash").unwrap_or_default(),
            created_at: r.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            updated_at: r.try_get("updated_at").unwrap_or_else(|_| Utc::now()),
        });
    }
    Ok(txs)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_address_request_deserializes() {
        let json = r#"{"wallet_id":"00000000-0000-0000-0000-000000000001","asset":"BTC"}"#;
        let req: DepositAddressRequest = serde_json::from_str(json).expect("valid json");
        assert_eq!(req.asset, "BTC");
    }

    #[test]
    fn withdraw_request_deserializes() {
        let json = r#"{"wallet_id":"00000000-0000-0000-0000-000000000001","asset":"ETH","amount":"1.5","dest_address":"0xABC"}"#;
        let req: WithdrawRequest = serde_json::from_str(json).expect("valid json");
        assert_eq!(req.asset, "ETH");
        assert_eq!(req.dest_address, "0xABC");
    }

    #[test]
    fn custody_transaction_serializes() {
        let tx = CustodyTransaction {
            id: Uuid::nil(),
            wallet_id: Uuid::nil(),
            tx_type: "withdrawal".to_string(),
            asset: "BTC".to_string(),
            amount: Decimal::new(15, 1), // 1.5
            fireblocks_tx_id: Some("fb-tx-1".to_string()),
            status: "pending".to_string(),
            tx_hash: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&tx).expect("serializable");
        assert!(json.contains("\"tx_type\":\"withdrawal\""));
        assert!(json.contains("\"asset\":\"BTC\""));
    }

    #[test]
    fn custody_wallet_serializes() {
        let w = CustodyWallet {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            fireblocks_vault_id: "vault-42".to_string(),
            name: "My Vault".to_string(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&w).expect("serializable");
        assert!(json.contains("\"fireblocks_vault_id\":\"vault-42\""));
    }

    #[test]
    fn balance_query_deserializes() {
        let json = r#"{"wallet_id":"00000000-0000-0000-0000-000000000001"}"#;
        let q: BalanceQuery = serde_json::from_str(json).expect("valid json");
        assert_eq!(
            q.wallet_id,
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        );
    }

    #[test]
    fn transactions_query_deserializes() {
        let json = r#"{"wallet_id":"00000000-0000-0000-0000-000000000001"}"#;
        let q: TransactionsQuery = serde_json::from_str(json).expect("valid json");
        assert_eq!(
            q.wallet_id,
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
        );
    }
}
