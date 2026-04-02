//! Fireblocks MPC Custody REST API Client (Wave 4 Task 22).
//!
//! Supports sandbox and production environments. Authentication
//! currently uses a Bearer token placeholder; full RSA JWT signing
//! will be wired when a real Fireblocks API key pair is provisioned.

use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

// ── Error type ────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum CustodyError {
    #[error("Fireblocks HTTP error: {0}")]
    Http(String),

    #[error("Fireblocks API error: {status} – {message}")]
    Api { status: u16, message: String },

    #[error("Serialization error: {0}")]
    Serde(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
}

pub type Result<T> = std::result::Result<T, CustodyError>;

// ── Fireblocks response types ─────────────────────────────────────────

/// Vault account returned by Fireblocks POST /v1/vault/accounts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultAccount {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub hidden_on_ui: bool,
    #[serde(default)]
    pub auto_fuel: bool,
}

/// Deposit address returned by Fireblocks
/// GET /v1/vault/accounts/{vaultId}/{assetId}/addresses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddress {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default, rename = "legacyAddress")]
    pub legacy_address: Option<String>,
}

/// Transaction response from Fireblocks POST /v1/transactions or
/// GET /v1/transactions/{txId}.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FireblocksTransaction {
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "txHash", default)]
    pub tx_hash: Option<String>,
    #[serde(rename = "assetId", default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub amount: Option<f64>,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<i64>,
}

/// Balance entry for a single asset inside a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalance {
    pub id: String,
    #[serde(default)]
    pub total: String,
    #[serde(default)]
    pub available: String,
    #[serde(default)]
    pub pending: String,
    #[serde(default, rename = "blockHeight")]
    pub block_height: Option<String>,
}

/// Vault account with asset balances from
/// GET /v1/vault/accounts/{vaultId}.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultAccountWithBalances {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub assets: Vec<AssetBalance>,
}

// ── Withdrawal request body ───────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CreateTransactionBody {
    #[serde(rename = "assetId")]
    asset_id: String,
    source: TransactionEndpoint,
    destination: TransactionEndpoint,
    amount: String,
}

#[derive(Debug, Serialize)]
struct TransactionEndpoint {
    #[serde(rename = "type")]
    endpoint_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "oneTimeAddress", skip_serializing_if = "Option::is_none")]
    one_time_address: Option<OneTimeAddress>,
}

#[derive(Debug, Serialize)]
struct OneTimeAddress {
    address: String,
}

// ── Client ────────────────────────────────────────────────────────────

/// Fireblocks REST API client.
///
/// In sandbox mode the base URL points to `https://sandbox-api.fireblocks.io`.
/// Production uses `https://api.fireblocks.io`.
///
/// Authentication is currently a Bearer token placeholder.  Full RSA JWT
/// signing (as required by the real Fireblocks API) will be added when
/// the private key file is provisioned.
#[derive(Debug, Clone)]
pub struct FireblocksClient {
    api_key: String,
    /// The RSA private key PEM (not used yet — placeholder for JWT signing).
    _api_secret: String,
    base_url: String,
    http: Client,
}

impl FireblocksClient {
    /// Create a new Fireblocks client.
    ///
    /// `base_url` examples:
    /// - Sandbox: `https://sandbox-api.fireblocks.io`
    /// - Production: `https://api.fireblocks.io`
    pub fn new(
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            _api_secret: api_secret.into(),
            base_url: base_url.into(),
            http: Client::new(),
        }
    }

    /// Construct from environment variables.
    ///
    /// - `FIREBLOCKS_API_KEY`
    /// - `FIREBLOCKS_API_SECRET` (RSA PEM — placeholder)
    /// - `FIREBLOCKS_SANDBOX` (default `true`)
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("FIREBLOCKS_API_KEY").ok()?;
        let api_secret = std::env::var("FIREBLOCKS_API_SECRET").unwrap_or_default();
        let sandbox = std::env::var("FIREBLOCKS_SANDBOX")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);
        let base_url = if sandbox {
            "https://sandbox-api.fireblocks.io"
        } else {
            "https://api.fireblocks.io"
        };
        info!("Fireblocks client initialised (sandbox={})", sandbox);
        Some(Self::new(api_key, api_secret, base_url))
    }

    // ── Helper ────────────────────────────────────────────────────────

    /// Build an authenticated GET/POST request. Uses Bearer token for now.
    fn auth_header(&self) -> String {
        // TODO: Replace with RSA JWT when real key is provisioned
        format!("Bearer {}", self.api_key)
    }

    // ── Vault operations ──────────────────────────────────────────────

    /// Create a new vault account for a user.
    ///
    /// POST /v1/vault/accounts
    pub async fn create_vault(&self, _user_id: &str, name: &str) -> Result<VaultAccount> {
        let url = format!("{}/v1/vault/accounts", self.base_url);
        let body = serde_json::json!({ "name": name });

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        if status >= 400 {
            warn!("Fireblocks create_vault failed: {} – {}", status, text);
            return Err(CustodyError::Api {
                status,
                message: text,
            });
        }

        serde_json::from_str::<VaultAccount>(&text)
            .map_err(|e| CustodyError::Serde(format!("{e}: {text}")))
    }

    /// Get a deposit address for an asset inside a vault.
    ///
    /// GET /v1/vault/accounts/{vaultId}/{assetId}/addresses
    pub async fn get_deposit_address(
        &self,
        vault_id: &str,
        asset: &str,
    ) -> Result<Vec<DepositAddress>> {
        let url = format!(
            "{}/v1/vault/accounts/{}/{}/addresses",
            self.base_url, vault_id, asset
        );

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        if status >= 400 {
            warn!(
                "Fireblocks get_deposit_address failed: {} – {}",
                status, text
            );
            return Err(CustodyError::Api {
                status,
                message: text,
            });
        }

        serde_json::from_str::<Vec<DepositAddress>>(&text)
            .map_err(|e| CustodyError::Serde(format!("{e}: {text}")))
    }

    /// Create a withdrawal (outgoing transaction).
    ///
    /// POST /v1/transactions
    pub async fn create_withdrawal(
        &self,
        vault_id: &str,
        asset: &str,
        amount: Decimal,
        dest_address: &str,
    ) -> Result<FireblocksTransaction> {
        let url = format!("{}/v1/transactions", self.base_url);

        let body = CreateTransactionBody {
            asset_id: asset.to_string(),
            source: TransactionEndpoint {
                endpoint_type: "VAULT_ACCOUNT".to_string(),
                id: Some(vault_id.to_string()),
                one_time_address: None,
            },
            destination: TransactionEndpoint {
                endpoint_type: "ONE_TIME_ADDRESS".to_string(),
                id: None,
                one_time_address: Some(OneTimeAddress {
                    address: dest_address.to_string(),
                }),
            },
            amount: amount.to_string(),
        };

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        if status >= 400 {
            warn!("Fireblocks create_withdrawal failed: {} – {}", status, text);
            return Err(CustodyError::Api {
                status,
                message: text,
            });
        }

        serde_json::from_str::<FireblocksTransaction>(&text)
            .map_err(|e| CustodyError::Serde(format!("{e}: {text}")))
    }

    /// Get a single transaction by Fireblocks TX id.
    ///
    /// GET /v1/transactions/{txId}
    pub async fn get_transaction(&self, tx_id: &str) -> Result<FireblocksTransaction> {
        let url = format!("{}/v1/transactions/{}", self.base_url, tx_id);

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        if status >= 400 {
            warn!("Fireblocks get_transaction failed: {} – {}", status, text);
            return Err(CustodyError::Api {
                status,
                message: text,
            });
        }

        serde_json::from_str::<FireblocksTransaction>(&text)
            .map_err(|e| CustodyError::Serde(format!("{e}: {text}")))
    }

    /// Get all asset balances for a vault.
    ///
    /// GET /v1/vault/accounts/{vaultId}
    pub async fn get_balance(&self, vault_id: &str) -> Result<VaultAccountWithBalances> {
        let url = format!("{}/v1/vault/accounts/{}", self.base_url, vault_id);

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| CustodyError::Http(e.to_string()))?;

        if status >= 400 {
            warn!("Fireblocks get_balance failed: {} – {}", status, text);
            return Err(CustodyError::Api {
                status,
                message: text,
            });
        }

        serde_json::from_str::<VaultAccountWithBalances>(&text)
            .map_err(|e| CustodyError::Serde(format!("{e}: {text}")))
    }
}

// ── Unit tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_construction() {
        let client = FireblocksClient::new(
            "test-key",
            "test-secret",
            "https://sandbox-api.fireblocks.io",
        );
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.base_url, "https://sandbox-api.fireblocks.io");
    }

    #[test]
    fn client_sandbox_url() {
        let client = FireblocksClient::new("k", "s", "https://sandbox-api.fireblocks.io");
        assert!(client.base_url.contains("sandbox"));
    }

    #[test]
    fn client_production_url() {
        let client = FireblocksClient::new("k", "s", "https://api.fireblocks.io");
        assert!(!client.base_url.contains("sandbox"));
    }

    #[test]
    fn auth_header_format() {
        let client = FireblocksClient::new("my-api-key", "s", "https://sandbox-api.fireblocks.io");
        assert_eq!(client.auth_header(), "Bearer my-api-key");
    }

    #[test]
    fn vault_account_deserializes() {
        let json = r#"{"id":"42","name":"User Vault","hiddenOnUi":false,"autoFuel":false}"#;
        let vault: VaultAccount = serde_json::from_str(json).expect("valid json");
        assert_eq!(vault.id, "42");
        assert_eq!(vault.name, "User Vault");
    }

    #[test]
    fn deposit_address_deserializes() {
        let json = r#"{"address":"0xABCDEF","tag":null,"legacyAddress":null}"#;
        let addr: DepositAddress = serde_json::from_str(json).expect("valid json");
        assert_eq!(addr.address, "0xABCDEF");
        assert!(addr.tag.is_none());
    }

    #[test]
    fn transaction_deserializes() {
        let json = r#"{"id":"tx-1","status":"COMPLETED","txHash":"0x123","assetId":"BTC","amount":0.5,"createdAt":1700000000}"#;
        let tx: FireblocksTransaction = serde_json::from_str(json).expect("valid json");
        assert_eq!(tx.id, "tx-1");
        assert_eq!(tx.status, "COMPLETED");
        assert_eq!(tx.tx_hash.as_deref(), Some("0x123"));
    }

    #[test]
    fn asset_balance_deserializes() {
        let json = r#"{"id":"BTC","total":"1.5","available":"1.0","pending":"0.5","blockHeight":"800000"}"#;
        let balance: AssetBalance = serde_json::from_str(json).expect("valid json");
        assert_eq!(balance.id, "BTC");
        assert_eq!(balance.total, "1.5");
        assert_eq!(balance.available, "1.0");
    }

    #[test]
    fn vault_with_balances_deserializes() {
        let json = r#"{"id":"42","name":"Vault","assets":[{"id":"BTC","total":"1.0","available":"0.8","pending":"0.2"}]}"#;
        let vault: VaultAccountWithBalances = serde_json::from_str(json).expect("valid json");
        assert_eq!(vault.id, "42");
        assert_eq!(vault.assets.len(), 1);
        assert_eq!(vault.assets[0].id, "BTC");
    }

    #[test]
    fn transaction_roundtrip() {
        let tx = FireblocksTransaction {
            id: "tx-99".to_string(),
            status: "PENDING".to_string(),
            tx_hash: None,
            asset_id: Some("ETH".to_string()),
            amount: Some(2.5),
            created_at: Some(1700000000),
        };
        let json = serde_json::to_string(&tx).unwrap();
        let back: FireblocksTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "tx-99");
        assert_eq!(back.asset_id, Some("ETH".to_string()));
    }

    #[test]
    fn custody_error_display() {
        let err = CustodyError::Api {
            status: 403,
            message: "Forbidden".to_string(),
        };
        assert!(err.to_string().contains("403"));
        assert!(err.to_string().contains("Forbidden"));
    }

    #[test]
    fn custody_error_variants() {
        let e1 = CustodyError::Http("timeout".to_string());
        assert!(e1.to_string().contains("timeout"));

        let e2 = CustodyError::WalletNotFound("abc".to_string());
        assert!(e2.to_string().contains("abc"));

        let e3 = CustodyError::TransactionNotFound("tx-1".to_string());
        assert!(e3.to_string().contains("tx-1"));

        let e4 = CustodyError::Database("connection refused".to_string());
        assert!(e4.to_string().contains("connection refused"));

        let e5 = CustodyError::Serde("missing field".to_string());
        assert!(e5.to_string().contains("missing field"));
    }
}
