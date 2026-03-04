//! OANDA Forex API Integration - Sprint 11

use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct OandaClient {
    api_key: String,
    account_id: String,
    client: reqwest::Client,
    base_url: String,
}

impl OandaClient {
    pub fn new(api_key: String, account_id: String, practice: bool) -> Self {
        let base_url = if practice {
            "https://api-fxpractice.oanda.com".to_string()
        } else {
            "https://api-fxtrade.oanda.com".to_string()
        };

        Self {
            api_key,
            account_id,
            client: reqwest::Client::new(),
            base_url,
        }
    }

    /// API base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get current price for a forex pair
    pub async fn get_price(&self, instrument: &str) -> Result<Decimal, OandaError> {
        let url = format!(
            "{}/v3/accounts/{}/pricing?instruments={}",
            self.base_url, self.account_id, instrument
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| OandaError::Network(e.to_string()))?;

        let data: PricingResponse = response
            .json()
            .await
            .map_err(|e| OandaError::Parse(e.to_string()))?;

        data.prices
            .first()
            .map(|p| p.closeout_ask)
            .ok_or_else(|| OandaError::Api("No price data".to_string()))
    }

    /// Get account summary
    pub async fn get_account(&self) -> Result<AccountSummary, OandaError> {
        let url = format!("{}/v3/accounts/{}/summary", self.base_url, self.account_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| OandaError::Network(e.to_string()))?;

        let data: AccountResponse = response
            .json()
            .await
            .map_err(|e| OandaError::Parse(e.to_string()))?;

        Ok(data.account)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OandaError {
    #[error("Network: {0}")]
    Network(String),
    #[error("API: {0}")]
    Api(String),
    #[error("Parse: {0}")]
    Parse(String),
}

#[derive(Debug, Deserialize)]
struct PricingResponse {
    prices: Vec<Price>,
}

#[derive(Debug, Deserialize)]
struct Price {
    instrument: String,
    closeout_ask: Decimal,
    closeout_bid: Decimal,
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    account: AccountSummary,
}

#[derive(Debug, Deserialize)]
pub struct AccountSummary {
    pub balance: Decimal,
    pub pl: Decimal,
    pub margin_used: Decimal,
    pub margin_available: Decimal,
    pub currency: String,
}

/// Major forex pairs
pub const MAJOR_PAIRS: &[&str] = &[
    "EUR_USD", "GBP_USD", "USD_JPY", "USD_CHF", "AUD_USD", "USD_CAD", "NZD_USD",
];

/// All 50+ pairs
pub const ALL_PAIRS: &[&str] = &[
    "EUR_USD", "GBP_USD", "USD_JPY", "USD_CHF", "AUD_USD", "USD_CAD", "NZD_USD", "EUR_GBP",
    "EUR_JPY", "EUR_CHF", "EUR_AUD", "EUR_CAD", "EUR_NZD", "GBP_JPY", "GBP_CHF", "GBP_AUD",
    "GBP_CAD", "GBP_NZD", "AUD_JPY", "AUD_CHF", "AUD_CAD", "AUD_NZD", "CAD_JPY", "CAD_CHF",
    "CHF_JPY", "NZD_JPY", "NZD_CHF", "NZD_CAD",
];
