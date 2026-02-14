//! Fireblocks Custody Integration
//! 
//! Real crypto custody using Fireblocks API
//! Docs: https://developers.fireblocks.com/

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn, error};

use super::{TreasuryError, TreasuryResult, Transaction, TransactionStatus};

/// Fireblocks API configuration
#[derive(Debug, Clone)]
pub struct FireblocksConfig {
    /// API endpoint (sandbox or production)
    pub api_url: String,
    /// Your API key from Fireblocks Console
    pub api_key: String,
    /// Path to API secret file (.key)
    pub api_secret_path: String,
    /// Default vault account ID
    pub vault_account_id: String,
    /// Request timeout
    pub timeout_secs: u64,
}

impl FireblocksConfig {
    /// Sandbox configuration for testing
    pub fn sandbox(api_key: String, secret_path: String, vault_id: String) -> Self {
        Self {
            api_url: "https://api-sandbox.fireblocks.io".to_string(),
            api_key,
            api_secret_path: secret_path,
            vault_account_id: vault_id,
            timeout_secs: 30,
        }
    }
    
    /// Production configuration
    pub fn production(api_key: String, secret_path: String, vault_id: String) -> Self {
        Self {
            api_url: "https://api.fireblocks.io".to_string(),
            api_key,
            api_secret_path: secret_path,
            vault_account_id: vault_id,
            timeout_secs: 60,
        }
    }
}

/// Fireblocks custody provider
#[derive(Debug)]
pub struct FireblocksCustody {
    config: FireblocksConfig,
    http_client: reqwest::Client,
    // JWT token for authentication (cached)
    jwt_token: String,
}

/// Asset balance in vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBalance {
    pub asset_id: String,      // "BTC", "ETH", "USDT"
    pub total: Decimal,        // Total balance
    pub available: Decimal,    // Available for trading
    pub frozen: Decimal,       // Locked in orders
}

/// Deposit address info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositAddress {
    pub asset_id: String,
    pub address: String,
    pub legacy_address: Option<String>,  // For BTC legacy format
    pub tag: Option<String>,             // For XRP, XLM, etc.
}

impl FireblocksCustody {
    /// Create new Fireblocks custody instance
    /// 
    /// # Errors
    /// Returns error if API credentials are invalid
    pub async fn new(config: FireblocksConfig) -> TreasuryResult<Self> {
        // Load API secret from file
        let secret = tokio::fs::read_to_string(&config.api_secret_path)
            .await
            .map_err(|e| TreasuryError::ConfigError(
                format!("Failed to read API secret: {}", e)
            ))?;
        
        // Create HTTP client
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| TreasuryError::ConfigError(e.to_string()))?;
        
        // Generate JWT token for authentication
        let jwt_token = Self::generate_jwt(&config.api_key, &secret)?;
        
        let custody = Self {
            config,
            http_client,
            jwt_token,
        };
        
        // Test connection
        custody.ping().await?;
        
        info!("Fireblocks custody initialized successfully");
        Ok(custody)
    }
    
    /// Test API connection
    async fn ping(&self) -> TreasuryResult<()> {
        let url = format!("{}/v1/account", self.config.api_url);
        
        let response = self.http_client
            .get(&url)
            .header("X-API-Key", &self.config.api_key)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .send()
            .await
            .map_err(|e| TreasuryError::ConnectionError(e.to_string()))?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err(TreasuryError::AuthenticationError(
                format!("API test failed: {}", response.status())
            ))
        }
    }
    
    /// Get all balances for the vault account
    pub async fn get_balances(&self) -> TreasuryResult<Vec<AssetBalance>> {
        let url = format!(
            "{}/v1/vault/accounts/{}", 
            self.config.api_url,
            self.config.vault_account_id
        );
        
        let response = self.http_client
            .get(&url)
            .header("X-API-Key", &self.config.api_key)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .send()
            .await
            .map_err(|e| TreasuryError::ApiError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TreasuryError::ApiError(error_text));
        }
        
        let vault_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| TreasuryError::ParseError(e.to_string()))?;
        
        // Parse balances from response
        let mut balances = Vec::new();
        
        if let Some(assets) = vault_data.get("assets").and_then(|a| a.as_array()) {
            for asset in assets {
                let balance = AssetBalance {
                    asset_id: asset.get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    total: asset.get("total")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(Decimal::ZERO),
                    available: asset.get("available")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(Decimal::ZERO),
                    frozen: asset.get("frozen")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(Decimal::ZERO),
                };
                balances.push(balance);
            }
        }
        
        info!("Retrieved {} asset balances", balances.len());
        Ok(balances)
    }
    
    /// Get balance for specific asset
    pub async fn get_balance(&self, asset_id: &str) -> TreasuryResult<AssetBalance> {
        let balances: Vec<AssetBalance> = self.get_balances().await?;
        
        balances.into_iter()
            .find(|b| b.asset_id == asset_id)
            .ok_or_else(|| TreasuryError::AssetNotFound(asset_id.to_string()))
    }
    
    /// Create deposit address for asset
    pub async fn create_deposit_address(&self, asset_id: &str) -> TreasuryResult<DepositAddress> {
        let url = format!(
            "{}/v1/vault/accounts/{}/{}/addresses",
            self.config.api_url,
            self.config.vault_account_id,
            asset_id
        );
        
        let response = self.http_client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .json(&serde_json::json!({
                "description": "Trading deposit address"
            }))
            .send()
            .await
            .map_err(|e| TreasuryError::ApiError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TreasuryError::ApiError(format!(
                "Failed to create address: {}", error_text
            )));
        }
        
        let address_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| TreasuryError::ParseError(e.to_string()))?;
        
        let address = DepositAddress {
            asset_id: asset_id.to_string(),
            address: address_data.get("address")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            legacy_address: address_data.get("legacyAddress")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tag: address_data.get("tag")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
        
        info!("Created deposit address for {}: {}", asset_id, address.address);
        Ok(address)
    }
    
    /// Create withdrawal transaction
    /// 
    /// # Security
    /// This requires proper transaction signing and may trigger
    /// Fireblocks policy rules (e.g., whitelist verification)
    pub async fn create_withdrawal(
        &self,
        asset_id: &str,
        amount: Decimal,
        destination_address: &str,
        destination_tag: Option<&str>,
    ) -> TreasuryResult<Transaction> {
        // Validate amount
        if amount <= Decimal::ZERO {
            return Err(TreasuryError::InvalidAmount("Amount must be positive".to_string()));
        }
        
        // Check sufficient balance
        let balance = self.get_balance(asset_id).await?;
        if balance.available < amount {
            return Err(TreasuryError::InsufficientFunds {
                asset: asset_id.to_string(),
                requested: amount,
                available: balance.available,
            });
        }
        
        let url = format!("{}/v1/transactions", self.config.api_url);
        
        let mut request_body = serde_json::json!({
            "assetId": asset_id,
            "source": {
                "type": "VAULT_ACCOUNT",
                "id": self.config.vault_account_id
            },
            "destination": {
                "type": "ONE_TIME_ADDRESS",
                "oneTimeAddress": {
                    "address": destination_address
                }
            },
            "amount": amount.to_string(),
            "note": "Investor OS withdrawal"
        });
        
        // Add destination tag if provided (for XRP, XLM, etc.)
        if let Some(tag) = destination_tag {
            request_body["destination"]["oneTimeAddress"]["tag"] = serde_json::json!(tag);
        }
        
        let response = self.http_client
            .post(&url)
            .header("X-API-Key", &self.config.api_key)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| TreasuryError::ApiError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            // Check for specific errors
            if error_text.contains("whitelist") {
                return Err(TreasuryError::SecurityError(
                    "Destination address not whitelisted".to_string()
                ));
            }
            
            return Err(TreasuryError::TransactionFailed(error_text));
        }
        
        let tx_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| TreasuryError::ParseError(e.to_string()))?;
        
        let tx_id = tx_data.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        
        let transaction = Transaction {
            id: tx_id.clone(),
            asset_id: asset_id.to_string(),
            amount,
            destination: destination_address.to_string(),
            status: TransactionStatus::Pending,
            created_at: chrono::Utc::now(),
            confirmed_at: None,
            tx_hash: None,  // Will be populated when confirmed
        };
        
        info!(
            "Created withdrawal transaction {} for {} {}",
            tx_id, amount, asset_id
        );
        
        Ok(transaction)
    }
    
    /// Get transaction status
    pub async fn get_transaction(&self, tx_id: &str) -> TreasuryResult<Transaction> {
        let url = format!("{}/v1/transactions/{}", self.config.api_url, tx_id);
        
        let response = self.http_client
            .get(&url)
            .header("X-API-Key", &self.config.api_key)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .send()
            .await
            .map_err(|e| TreasuryError::ApiError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(TreasuryError::TransactionNotFound(tx_id.to_string()));
        }
        
        let tx_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| TreasuryError::ParseError(e.to_string()))?;
        
        let status = tx_data.get("status")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "COMPLETED" => TransactionStatus::Confirmed,
                "FAILED" => TransactionStatus::FailedStatus,
                "CANCELLED" => TransactionStatus::Cancelled,
                _ => TransactionStatus::Pending,
            })
            .unwrap_or(TransactionStatus::Pending);
        
        let transaction = Transaction {
            id: tx_id.to_string(),
            asset_id: tx_data.get("assetId")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string(),
            amount: tx_data.get("amount")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(Decimal::ZERO),
            destination: tx_data.get("destination")
                .and_then(|v| v.get("oneTimeAddress"))
                .and_then(|v| v.get("address"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            status,
            created_at: tx_data.get("createdAt")
                .and_then(|v| v.as_i64())
                .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default())
                .unwrap_or_else(chrono::Utc::now),
            confirmed_at: tx_data.get("signedBy")
                .and_then(|v| v.as_array())
                .filter(|arr| !arr.is_empty())
                .map(|_| chrono::Utc::now()),
            tx_hash: tx_data.get("txHash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };
        
        Ok(transaction)
    }
    
    /// List all transactions (with pagination)
    pub async fn list_transactions(
        &self,
        limit: usize,
        before: Option<&str>,
    ) -> TreasuryResult<Vec<Transaction>> {
        let mut url = format!(
            "{}/v1/transactions?limit={}",
            self.config.api_url,
            limit.min(500)  // Fireblocks max is 500
        );
        
        if let Some(before_id) = before {
            url.push_str(&format!("&before={}", before_id));
        }
        
        let response = self.http_client
            .get(&url)
            .header("X-API-Key", &self.config.api_key)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .send()
            .await
            .map_err(|e| TreasuryError::ApiError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TreasuryError::ApiError(error_text));
        }
        
        let txs_data: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| TreasuryError::ParseError(e.to_string()))?;
        
        let transactions: Vec<Transaction> = txs_data
            .into_iter()
            .map(|tx_data| {
                let status = tx_data.get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "COMPLETED" => TransactionStatus::Confirmed,
                        "FAILED" => TransactionStatus::FailedStatus,
                        "CANCELLED" => TransactionStatus::Cancelled,
                        _ => TransactionStatus::Pending,
                    })
                    .unwrap_or(TransactionStatus::Pending);
                
                Transaction {
                    id: tx_data.get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    asset_id: tx_data.get("assetId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    amount: tx_data.get("amount")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(Decimal::ZERO),
                    destination: "unknown".to_string(), // Simplified
                    status,
                    created_at: chrono::Utc::now(), // Simplified
                    confirmed_at: None,
                    tx_hash: None,
                }
            })
            .collect();
        
        Ok(transactions)
    }
    
    /// Generate JWT token for authentication
    fn generate_jwt(api_key: &str, secret: &str) -> TreasuryResult<String> {
        // This is a simplified version
        // Real implementation uses RS256 with private key
        use jsonwebtoken::{encode, EncodingKey, Header};
        
        #[derive(serde::Serialize)]
        struct Claims {
            sub: String,
            iat: i64,
            exp: i64,
        }
        
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: api_key.to_string(),
            iat: now,
            exp: now + 300, // 5 minutes
        };
        
        // Note: Real Fireblocks uses RS256 with their specific key format
        // This is simplified for demonstration
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        ).map_err(|e| TreasuryError::AuthenticationError(e.to_string()))?;
        
        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // These tests require Fireblocks sandbox credentials
    // Set FIREBLOCKS_API_KEY and FIREBLOCKS_SECRET_PATH env vars
    
    #[tokio::test]
    #[ignore = "Requires Fireblocks sandbox credentials"]
    async fn test_fireblocks_integration() {
        let config = FireblocksConfig::sandbox(
            std::env::var("FIREBLOCKS_API_KEY").unwrap(),
            std::env::var("FIREBLOCKS_SECRET_PATH").unwrap(),
            "0".to_string(), // Default vault
        );
        
        let custody = FireblocksCustody::new(config).await.unwrap();
        
        // Test get balances
        let balances = custody.get_balances().await.unwrap();
        println!("Balances: {:?}", balances);
        
        // Test create deposit address
        let address = custody.create_deposit_address("BTC").await.unwrap();
        println!("BTC Address: {}", address.address);
    }
}
