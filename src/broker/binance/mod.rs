//! Binance API Integration - Sprint 11

use hmac::{Hmac, Mac};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fmt::Write;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct BinanceClient {
    api_key: String,
    api_secret: String,
    client: reqwest::Client,
    base_url: String,
}

impl BinanceClient {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            client: reqwest::Client::new(),
            base_url: "https://api.binance.com".to_string(),
        }
    }

    /// API base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn get_price(&self, symbol: &str) -> Result<Decimal, BinanceError> {
        let url = format!("{}/api/v3/ticker/price?symbol={}", self.base_url, symbol);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| BinanceError::Network(e.to_string()))?;

        let data: SymbolPrice = response
            .json()
            .await
            .map_err(|e| BinanceError::Parse(e.to_string()))?;

        Ok(data.price)
    }

    pub async fn get_account(&self) -> Result<AccountInfo, BinanceError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let query = format!("timestamp={}", timestamp);
        let signature = self.sign(&query);

        let url = format!(
            "{}/api/v3/account?{}&signature={}",
            self.base_url, query, signature
        );

        let response = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| BinanceError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(BinanceError::Api(format!("HTTP {}: {}", status, body)));
        }

        response
            .json()
            .await
            .map_err(|e| BinanceError::Parse(e.to_string()))
    }

    fn sign(&self, query: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take any key length");
        mac.update(query.as_bytes());
        let signature = mac.finalize().into_bytes();

        let mut output = String::with_capacity(signature.len() * 2);
        for byte in signature {
            let _ = write!(output, "{:02x}", byte);
        }
        output
    }
}

#[derive(Debug, Error)]
pub enum BinanceError {
    #[error("Network: {0}")]
    Network(String),
    #[error("API: {0}")]
    Api(String),
    #[error("Parse: {0}")]
    Parse(String),
}

#[derive(Debug, Deserialize)]
struct SymbolPrice {
    price: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct AccountInfo {
    pub balances: Vec<Balance>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Balance {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
}

#[derive(Debug, Serialize)]
pub struct BinanceOrder {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: Option<Decimal>,
    pub price: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub enum OrderSide {
    BUY,
    SELL,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BUY => write!(f, "BUY"),
            Self::SELL => write!(f, "SELL"),
        }
    }
}

#[derive(Debug, Serialize)]
pub enum OrderType {
    LIMIT,
    MARKET,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LIMIT => write!(f, "LIMIT"),
            Self::MARKET => write!(f, "MARKET"),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct OrderResponse {
    pub orderId: u64,
    pub status: String,
    pub executedQty: Decimal,
    pub price: Decimal,
}

impl BinanceClient {
    pub async fn place_order(&self, order: BinanceOrder) -> Result<OrderResponse, BinanceError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let quantity = order
            .quantity
            .ok_or_else(|| BinanceError::Api("quantity is required".to_string()))?;

        let mut query = format!(
            "symbol={}&side={}&type={}&quantity={}&timestamp={}",
            order.symbol, order.side, order.order_type, quantity, timestamp
        );

        if let Some(price) = order.price {
            query.push_str(&format!("&price={}", price));
        }

        query.push_str("&recvWindow=5000");

        let signature = self.sign(&query);

        let full_query = format!("{}&signature={}", query, signature);
        let url = format!("{}/api/v3/order", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(full_query)
            .send()
            .await
            .map_err(|e| BinanceError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(BinanceError::Api(format!("HTTP {}: {}", status, body)));
        }

        response
            .json()
            .await
            .map_err(|e| BinanceError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_is_deterministic_for_same_query() {
        let client = BinanceClient::new("k".to_string(), "secret".to_string());
        let query =
            "symbol=BTCUSDT&side=BUY&type=LIMIT&quantity=1&timestamp=123456&recvWindow=5000";

        let first = client.sign(query);
        let second = client.sign(query);

        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn test_sign_changes_for_different_queries() {
        let client = BinanceClient::new("k".to_string(), "secret".to_string());

        let first = client.sign("symbol=BTCUSDT&side=BUY&type=LIMIT&quantity=1&timestamp=1");
        let second = client.sign("symbol=BTCUSDT&side=SELL&type=LIMIT&quantity=1&timestamp=1");

        assert_ne!(first, second);
    }

    #[test]
    fn test_sign_is_hex_output() {
        let client = BinanceClient::new("k".to_string(), "secret".to_string());

        let signature = client.sign("symbol=ETHUSDT");

        assert!(!signature.is_empty());
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

/// Top crypto symbols for trading
pub const TOP_CRYPTOS: &[&str] = &["BTCUSDT", "ETHUSDT", "SOLUSDT", "ADAUSDT", "DOTUSDT"];
