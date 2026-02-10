//! Binance API Integration - Sprint 11

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
    
    pub async fn get_price(&self, symbol: &str) -> Result<Decimal, BinanceError> {
        let url = format!("{}/api/v3/ticker/price?symbol={}", self.base_url, symbol);
        
        let response = self.client.get(&url).send().await
            .map_err(|e| BinanceError::Network(e.to_string()))?;
        
        let data: SymbolPrice = response.json().await
            .map_err(|e| BinanceError::Parse(e.to_string()))?;
        
        Ok(data.price)
    }
    
    pub async fn get_account(&self) -> Result<AccountInfo, BinanceError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let query = format!("timestamp={}", timestamp);
        let signature = self.sign(&query);
        
        let url = format!("{}/api/v3/account?{}&signature={}", self.base_url, query, signature);
        
        let response = self.client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| BinanceError::Network(e.to_string()))?;
        
        response.json().await
            .map_err(|e| BinanceError::Parse(e.to_string()))
    }
    
    fn sign(&self, query: &str) -> String {
        // Simple placeholder - would use hmac-sha256 in production
        format!("sig_{}", query.len())
    }
}

#[derive(Debug, thiserror::Error)]
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
        // Placeholder implementation
        Ok(OrderResponse {
            orderId: 12345,
            status: "FILLED".to_string(),
            executedQty: order.quantity.unwrap_or(Decimal::ONE),
            price: order.price.unwrap_or(Decimal::from(50000)),
        })
    }
}

/// Top crypto symbols for trading
pub const TOP_CRYPTOS: &[&str] = &["BTCUSDT", "ETHUSDT", "SOLUSDT", "ADAUSDT", "DOTUSDT"];
