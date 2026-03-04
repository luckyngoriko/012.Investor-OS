//! Data Source Connectors
//!
//! Connectors for various data providers

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

use crate::data_sources::{ConnectionTest, DataFetchRequest, DataFetchResponse};

fn response_error(error: String, fetch_time_ms: i64) -> DataFetchResponse {
    DataFetchResponse {
        success: false,
        data: Value::Null,
        records_count: 0,
        fetch_time_ms,
        error: Some(error),
    }
}

fn response_success(data: Value, fetch_time_ms: i64) -> DataFetchResponse {
    let records_count = match &data {
        Value::Array(values) => values.len(),
        Value::Object(obj) => {
            if let Some(records) = obj.get("data") {
                match records {
                    Value::Array(values) => values.len(),
                    _ => 1,
                }
            } else if obj.values().any(|value| value.is_array()) {
                obj.values()
                    .find_map(|value| value.as_array().map(|values| values.len()))
                    .unwrap_or(1)
            } else {
                1
            }
        }
        _ => 0,
    };

    DataFetchResponse {
        success: true,
        data,
        records_count,
        fetch_time_ms,
        error: None,
    }
}

fn extract_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|value| value.as_str().map(str::to_string))
}

fn collect_query_pairs(params: &Value, skip: &[&str]) -> Vec<(String, String)> {
    let skip: HashMap<&str, ()> = skip.iter().copied().map(|key| (key, ())).collect();
    params
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    if skip.contains_key(key.as_str()) {
                        return None;
                    }

                    value
                        .as_str()
                        .map(|value| (key.clone(), value.to_string()))
                        .or_else(|| value.as_i64().map(|value| (key.clone(), value.to_string())))
                        .or_else(|| value.as_f64().map(|value| (key.clone(), value.to_string())))
                        .or_else(|| {
                            value
                                .as_bool()
                                .map(|value| (key.clone(), value.to_string()))
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn expand_placeholder_path(
    template: &str,
    params: &Value,
    placeholders: &[(&str, &str)],
) -> String {
    placeholders
        .iter()
        .fold(template.to_string(), |path, (token, fallback)| {
            let replacement = extract_string(params, token).unwrap_or_else(|| fallback.to_string());
            path.replace(token, &replacement)
        })
}

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
        let api_key = config["api_key"]
            .as_str()
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
        let url = format!(
            "{}/query?function=GLOBAL_QUOTE&symbol=IBM&apikey={}",
            self.base_url, self.api_key
        );

        let start = std::time::Instant::now();
        let response = self.client.get(&url).send().await?;
        let elapsed = start.elapsed().as_millis() as i64;

        let text = response.text().await?;
        let success = text.contains("Global Quote");

        Ok(ConnectionTest {
            success,
            response_time_ms: elapsed,
            message: if success {
                "Connected".to_string()
            } else {
                "Invalid API key".to_string()
            },
            details: None,
        })
    }

    async fn fetch(&self, _request: DataFetchRequest) -> anyhow::Result<DataFetchResponse> {
        let start = Instant::now();
        let mut query = collect_query_pairs(
            &_request.params,
            &["path", "endpoint_id", "provider", "method"],
        );

        if self.api_key.is_empty() {
            return Ok(response_error(
                "Missing Alpha Vantage API key".to_string(),
                start.elapsed().as_millis() as i64,
            ));
        }

        if !query.iter().any(|(key, _)| key == "function") {
            query.push(("function".to_string(), "TIME_SERIES_DAILY".to_string()));
        }

        if !query.iter().any(|(key, _)| key == "symbol") {
            query.push(("symbol".to_string(), "IBM".to_string()));
        }

        query.push(("apikey".to_string(), self.api_key.clone()));

        let url = format!("{}/query", self.base_url.trim_end_matches('/'));
        let response = self.client.get(&url).query(&query).send().await?;
        let fetch_time_ms = start.elapsed().as_millis() as i64;

        if !response.status().is_success() {
            return Ok(response_error(
                format!("Alpha Vantage request failed: HTTP {}", response.status()),
                fetch_time_ms,
            ));
        }

        let payload = response.json::<Value>().await?;

        if payload.get("Error Message").is_some() || payload.get("Note").is_some() {
            return Ok(response_error(
                payload
                    .get("Error Message")
                    .or_else(|| payload.get("Note"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("Alpha Vantage returned an error")
                    .to_string(),
                fetch_time_ms,
            ));
        }

        Ok(response_success(payload, fetch_time_ms))
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
        let response = self
            .client
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
        let start = Instant::now();
        let params = _request.params;
        let path = extract_string(&params, "path")
            .unwrap_or_else(|| "/history".to_string())
            .to_lowercase();
        let symbol = extract_string(&params, "symbol")
            .or_else(|| extract_string(&params, "ticker"))
            .unwrap_or_else(|| "AAPL".to_string());

        match path.as_str() {
            "/info" => {
                let mut query = collect_query_pairs(
                    &params,
                    &[
                        "path",
                        "endpoint_id",
                        "provider",
                        "symbol",
                        "ticker",
                        "method",
                    ],
                );
                if !query.iter().any(|(key, _)| key == "modules") {
                    query.push((
                        "modules".to_string(),
                        "assetProfile,summaryProfile,summaryDetail".to_string(),
                    ));
                }
                let url = format!(
                    "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}",
                    symbol
                );
                let response = self.client.get(&url).query(&query).send().await?;
                let fetch_time_ms = start.elapsed().as_millis() as i64;
                if !response.status().is_success() {
                    return Ok(response_error(
                        format!("Yahoo Finance request failed: HTTP {}", response.status()),
                        fetch_time_ms,
                    ));
                }
                Ok(response_success(
                    response.json::<Value>().await?,
                    fetch_time_ms,
                ))
            }
            "/financials" => {
                let mut query = collect_query_pairs(
                    &params,
                    &[
                        "path",
                        "endpoint_id",
                        "provider",
                        "symbol",
                        "ticker",
                        "method",
                    ],
                );
                if !query.iter().any(|(key, _)| key == "modules") {
                    query.push((
                        "modules".to_string(),
                        "incomeStatementHistory,cashflowStatementHistory,balanceSheetHistory"
                            .to_string(),
                    ));
                }
                let url = format!(
                    "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}",
                    symbol
                );
                let response = self.client.get(&url).query(&query).send().await?;
                let fetch_time_ms = start.elapsed().as_millis() as i64;
                if !response.status().is_success() {
                    return Ok(response_error(
                        format!("Yahoo Finance request failed: HTTP {}", response.status()),
                        fetch_time_ms,
                    ));
                }
                Ok(response_success(
                    response.json::<Value>().await?,
                    fetch_time_ms,
                ))
            }
            "/options" => {
                let mut query = collect_query_pairs(
                    &params,
                    &[
                        "path",
                        "endpoint_id",
                        "provider",
                        "symbol",
                        "ticker",
                        "method",
                    ],
                );
                if let Some(date) = extract_string(&params, "date") {
                    query.push(("date".to_string(), date));
                }
                let url = format!(
                    "https://query2.finance.yahoo.com/v7/finance/options/{}",
                    symbol
                );
                let response = self.client.get(&url).query(&query).send().await?;
                let fetch_time_ms = start.elapsed().as_millis() as i64;
                if !response.status().is_success() {
                    return Ok(response_error(
                        format!("Yahoo Finance request failed: HTTP {}", response.status()),
                        fetch_time_ms,
                    ));
                }
                Ok(response_success(
                    response.json::<Value>().await?,
                    fetch_time_ms,
                ))
            }
            _ => {
                let mut query = collect_query_pairs(
                    &params,
                    &[
                        "path",
                        "endpoint_id",
                        "provider",
                        "symbol",
                        "ticker",
                        "method",
                    ],
                );
                if !query.iter().any(|(key, _)| key == "range") {
                    query.push(("range".to_string(), "1mo".to_string()));
                }
                if !query.iter().any(|(key, _)| key == "interval") {
                    query.push(("interval".to_string(), "1d".to_string()));
                }
                let url = format!(
                    "https://query1.finance.yahoo.com/v8/finance/chart/{}",
                    symbol
                );
                let response = self.client.get(&url).query(&query).send().await?;
                let fetch_time_ms = start.elapsed().as_millis() as i64;
                if !response.status().is_success() {
                    return Ok(response_error(
                        format!("Yahoo Finance request failed: HTTP {}", response.status()),
                        fetch_time_ms,
                    ));
                }
                Ok(response_success(
                    response.json::<Value>().await?,
                    fetch_time_ms,
                ))
            }
        }
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
        let api_key = config["api_key"]
            .as_str()
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
        let url = format!(
            "{}/series/observations?series_id=GNPCA&api_key={}&file_type=json&limit=1",
            self.base_url, self.api_key
        );

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
        let start = Instant::now();
        let path = extract_string(&_request.params, "path")
            .unwrap_or_else(|| "/fred/series/observations".to_string());
        let mut query = collect_query_pairs(
            &_request.params,
            &["path", "endpoint_id", "provider", "method"],
        );

        let mut url = format!("{}{}", self.base_url.trim_end_matches('/'), path);

        if !url.contains("api_key") && !self.api_key.is_empty() {
            query.push(("api_key".to_string(), self.api_key.clone()));
        }
        query.push(("file_type".to_string(), "json".to_string()));

        if let Some(series_id) = extract_string(&_request.params, "series_id") {
            query.push(("series_id".to_string(), series_id));
        }

        let response = self.client.get(&url).query(&query).send().await?;
        let fetch_time_ms = start.elapsed().as_millis() as i64;

        if !response.status().is_success() {
            return Ok(response_error(
                format!("FRED request failed: HTTP {}", response.status()),
                fetch_time_ms,
            ));
        }

        let payload = response.json::<Value>().await?;

        if let Some(error_msg) = payload.get("error_code").and_then(|v| v.as_str()) {
            return Ok(response_error(error_msg.to_string(), fetch_time_ms));
        }

        Ok(response_success(payload, fetch_time_ms))
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
        let response = self
            .client
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
        let start = Instant::now();
        let base_url = "https://api.worldbank.org/v2";
        let mut path =
            extract_string(&_request.params, "path").unwrap_or_else(|| "/country".to_string());
        if path.contains("{country}") || path.contains("{indicator}") {
            path = expand_placeholder_path(
                &path,
                &_request.params,
                &[("{country}", "USA"), ("{indicator}", "NY.GDP.MKTP.CD")],
            );
        };
        let normalized_path = if path.starts_with("http://") || path.starts_with("https://") {
            path
        } else {
            format!(
                "{}/{}",
                base_url.trim_end_matches('/'),
                path.trim_start_matches('/')
            )
        };

        let mut query = collect_query_pairs(
            &_request.params,
            &["path", "endpoint_id", "provider", "method"],
        );
        query.push(("format".to_string(), "json".to_string()));

        let response = self
            .client
            .get(&normalized_path)
            .query(&query)
            .send()
            .await?;
        let fetch_time_ms = start.elapsed().as_millis() as i64;

        if !response.status().is_success() {
            return Ok(response_error(
                format!("World Bank request failed: HTTP {}", response.status()),
                fetch_time_ms,
            ));
        }

        Ok(response_success(
            response.json::<Value>().await?,
            fetch_time_ms,
        ))
    }
}

/// CoinGecko connector
pub struct CoinGeckoConnector {
    client: reqwest::Client,
    base_url: String,
}

impl CoinGeckoConnector {
    /// Create new connector
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.coingecko.com/api/v3".to_string(),
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
        let response = self
            .client
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
        let start = Instant::now();
        let path = expand_placeholder_path(
            &extract_string(&_request.params, "path").unwrap_or_else(|| "/coins/list".to_string()),
            &_request.params,
            &[("{id}", "bitcoin")],
        );
        let api_path = if path.starts_with("/api/v3") {
            path.clone()
        } else if path.starts_with("http") {
            path.clone()
        } else {
            format!("/api/v3/{}", path.trim_start_matches('/'))
        };
        let endpoint_path = api_path.as_str();
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), endpoint_path);
        let mut query = collect_query_pairs(
            &_request.params,
            &["path", "endpoint_id", "provider", "method", "id"],
        );

        if (path == "/coins/markets" || endpoint_path == "/api/v3/coins/markets")
            && !query.iter().any(|(key, _)| key == "vs_currency")
        {
            query.push(("vs_currency".to_string(), "usd".to_string()));
        }

        let is_history_endpoint = path.ends_with("/history") || endpoint_path.ends_with("/history");
        if is_history_endpoint && !query.iter().any(|(key, _)| key == "date") {
            return Ok(response_error(
                "Missing required date for coin history".to_string(),
                start.elapsed().as_millis() as i64,
            ));
        }

        let response = self.client.get(&url).query(&query).send().await?;
        let fetch_time_ms = start.elapsed().as_millis() as i64;

        if !response.status().is_success() {
            return Ok(response_error(
                format!("CoinGecko request failed: HTTP {}", response.status()),
                fetch_time_ms,
            ));
        }

        Ok(response_success(
            response.json::<Value>().await?,
            fetch_time_ms,
        ))
    }
}
