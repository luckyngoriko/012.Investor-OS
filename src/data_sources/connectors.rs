//! Data Source Connectors
//!
//! Connectors for various data providers

use async_trait::async_trait;
use serde_json::Value;

use crate::data_sources::{ConnectionTest, DataFetchRequest, DataFetchResponse};

/// Trait for data source connectors
#[async_trait]
pub trait DataSourceConnector: Send + Sync {
    /// Connector name
    fn name(&self) -> &str;
    
    /// Test connection
    async fn test_connection(&self) -> anyhow::Result<ConnectionTest>;
    
    /// Fetch data
    async fn fetch(&self, request: DataFetchRequest) -> anyhow::Result<DataFetchResponse>;
}

/// Connector factory
pub struct ConnectorFactory;

impl ConnectorFactory {
    /// Create connector for provider
    pub fn create(provider: &str, config: Value) -> Option<Box<dyn DataSourceConnector>> {
        match provider {
            "alpha_vantage" => Some(Box::new(AlphaVantageConnector::new(config))),
            "yahoo_finance" => Some(Box::new(YahooFinanceConnector::new())),
            "fred" => Some(Box::new(FredConnector::new(config))),
            "world_bank" => Some(Box::new(WorldBankConnector::new())),
            "coingecko" => Some(Box::new(CoinGeckoConnector::new())),
            _ => None,
        }
    }
}

/// Alpha Vantage connector
pub struct AlphaVantageConnector {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AlphaVantageConnector {
    /// Create new connector
    pub fn new(config: Value) -> Self {
        let api_key = config["api_key"].as_str()
            .map(|s| s.to_string())
            .or_else(|| std::env::var("ALPHA_VANTAGE_API_KEY").ok())
            .unwrap_or_default();
        
        Self {
            api_key,
            base_url: "https://www.alphavantage.co".to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DataSourceConnector for AlphaVantageConnector {
    fn name(&self) -> &str {
        "Alpha Vantage"
    }
    
    async fn test_connection(&self) -> anyhow::Result<ConnectionTest> {
        let url = format!("{}/query?function=GLOBAL_QUOTE&symbol=IBM&apikey={}",
            self.base_url, self.api_key);
        
        let start = std::time::Instant::now();
        let response = self.client.get(&url).send().await?;
        let elapsed = start.elapsed().as_millis() as i64;
        
        let text = response.text().await?;
        let success = text.contains("Global Quote");
        
        Ok(ConnectionTest {
            success,
            response_time_ms: elapsed,
            message: if success { "Connected".to_string() } else { "Invalid API key".to_string() },
            details: None,
        })
    }
    
    async fn fetch(&self, _request: DataFetchRequest) -> anyhow::Result<DataFetchResponse> {
        // Implementation would fetch specific endpoint
        Ok(DataFetchResponse {
            success: false,
            data: Value::Null,
            records_count: 0,
            fetch_time_ms: 0,
            error: Some("Not implemented".to_string()),
        })
    }
}

/// Yahoo Finance connector
pub struct YahooFinanceConnector {
    client: reqwest::Client,
}

impl YahooFinanceConnector {
    /// Create new connector
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DataSourceConnector for YahooFinanceConnector {
    fn name(&self) -> &str {
        "Yahoo Finance"
    }
    
    async fn test_connection(&self) -> anyhow::Result<ConnectionTest> {
        let start = std::time::Instant::now();
        let response = self.client
            .get("https://finance.yahoo.com/quote/AAPL")
            .send()
            .await?;
        let elapsed = start.elapsed().as_millis() as i64;
        
        Ok(ConnectionTest {
            success: response.status().is_success(),
            response_time_ms: elapsed,
            message: "Yahoo Finance accessible".to_string(),
            details: None,
        })
    }
    
    async fn fetch(&self, _request: DataFetchRequest) -> anyhow::Result<DataFetchResponse> {
        Ok(DataFetchResponse {
            success: false,
            data: Value::Null,
            records_count: 0,
            fetch_time_ms: 0,
            error: Some("Not implemented".to_string()),
        })
    }
}

/// FRED connector
pub struct FredConnector {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl FredConnector {
    /// Create new connector
    pub fn new(config: Value) -> Self {
        let api_key = config["api_key"].as_str()
            .map(|s| s.to_string())
            .or_else(|| std::env::var("FRED_API_KEY").ok())
            .unwrap_or_default();
        
        Self {
            api_key,
            base_url: "https://api.stlouisfed.org/fred".to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DataSourceConnector for FredConnector {
    fn name(&self) -> &str {
        "FRED"
    }
    
    async fn test_connection(&self) -> anyhow::Result<ConnectionTest> {
        let url = format!("{}/series/observations?series_id=GNPCA&api_key={}&file_type=json&limit=1",
            self.base_url, self.api_key);
        
        let start = std::time::Instant::now();
        let response = self.client.get(&url).send().await?;
        let elapsed = start.elapsed().as_millis() as i64;
        
        Ok(ConnectionTest {
            success: response.status().is_success(),
            response_time_ms: elapsed,
            message: "FRED API accessible".to_string(),
            details: None,
        })
    }
    
    async fn fetch(&self, _request: DataFetchRequest) -> anyhow::Result<DataFetchResponse> {
        Ok(DataFetchResponse {
            success: false,
            data: Value::Null,
            records_count: 0,
            fetch_time_ms: 0,
            error: Some("Not implemented".to_string()),
        })
    }
}

/// World Bank connector
pub struct WorldBankConnector {
    client: reqwest::Client,
}

impl WorldBankConnector {
    /// Create new connector
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DataSourceConnector for WorldBankConnector {
    fn name(&self) -> &str {
        "World Bank"
    }
    
    async fn test_connection(&self) -> anyhow::Result<ConnectionTest> {
        let start = std::time::Instant::now();
        let response = self.client
            .get("https://api.worldbank.org/v2/country/US?format=json")
            .send()
            .await?;
        let elapsed = start.elapsed().as_millis() as i64;
        
        Ok(ConnectionTest {
            success: response.status().is_success(),
            response_time_ms: elapsed,
            message: "World Bank API accessible".to_string(),
            details: None,
        })
    }
    
    async fn fetch(&self, _request: DataFetchRequest) -> anyhow::Result<DataFetchResponse> {
        Ok(DataFetchResponse {
            success: false,
            data: Value::Null,
            records_count: 0,
            fetch_time_ms: 0,
            error: Some("Not implemented".to_string()),
        })
    }
}

/// CoinGecko connector
pub struct CoinGeckoConnector {
    client: reqwest::Client,
}

impl CoinGeckoConnector {
    /// Create new connector
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DataSourceConnector for CoinGeckoConnector {
    fn name(&self) -> &str {
        "CoinGecko"
    }
    
    async fn test_connection(&self) -> anyhow::Result<ConnectionTest> {
        let start = std::time::Instant::now();
        let response = self.client
            .get("https://api.coingecko.com/api/v3/ping")
            .send()
            .await?;
        let elapsed = start.elapsed().as_millis() as i64;
        
        Ok(ConnectionTest {
            success: response.status().is_success(),
            response_time_ms: elapsed,
            message: "CoinGecko API accessible".to_string(),
            details: None,
        })
    }
    
    async fn fetch(&self, _request: DataFetchRequest) -> anyhow::Result<DataFetchResponse> {
        Ok(DataFetchResponse {
            success: false,
            data: Value::Null,
            records_count: 0,
            fetch_time_ms: 0,
            error: Some("Not implemented".to_string()),
        })
    }
}
