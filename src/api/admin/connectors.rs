//! API Connectors
//!
//! Реални имплементации на връзки с външни API-та
//! Този модул ще бъде разширен когато се добавят реални интеграции

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Trait за всички API конектори
#[async_trait]
pub trait ApiConnector: Send + Sync {
    /// Тестваме връзката с API-то
    async fn test_connection(&self) -> ConnectionTestResult;

    /// Връща името на конектора
    fn name(&self) -> &str;

    /// Връща статуса на конектора
    fn status(&self) -> ConnectorStatus;
}

/// Резултат от тест на връзка
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub response_time_ms: u64,
    pub message: String,
    pub errors: Vec<String>,
}

/// Статус на конектор
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectorStatus {
    Connected,
    Disconnected,
    Error(String),
    NotConfigured,
}

/// Interactive Brokers Connector (placeholder)
pub struct InteractiveBrokersConnector {
    config: IBKRConfig,
    status: ConnectorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IBKRConfig {
    pub api_key: String,
    pub api_secret: String,
    pub base_url: String,
    pub paper_trading: bool,
}

impl InteractiveBrokersConnector {
    pub fn new(config: IBKRConfig) -> Self {
        Self {
            config,
            status: ConnectorStatus::NotConfigured,
        }
    }
}

#[async_trait]
impl ApiConnector for InteractiveBrokersConnector {
    async fn test_connection(&self) -> ConnectionTestResult {
        let started = Instant::now();

        if self.config.api_key.trim().is_empty() || self.config.api_secret.trim().is_empty() {
            return ConnectionTestResult {
                success: false,
                response_time_ms: started.elapsed().as_millis() as u64,
                message: "IBKR credentials are required for validation".to_string(),
                errors: vec!["Missing IBKR_API_KEY or IBKR_API_SECRET".to_string()],
            };
        }

        let endpoint = format!(
            "{}/v1/api/iserver/marketdata/snapshot",
            self.config.base_url.trim_end_matches('/'),
        );

        match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(client) => {
                match client.get(&endpoint).send().await {
                    Ok(response) => {
                        if response.status().is_client_error() {
                            return ConnectionTestResult {
                                success: false,
                                response_time_ms: started.elapsed().as_millis() as u64,
                                message: format!(
                                    "IBKR endpoint responded with status {}",
                                    response.status()
                                ),
                                errors: vec!["Credentials or endpoint path may require authenticated session".to_string()],
                            };
                        }

                        return ConnectionTestResult {
                            success: response.status().is_success()
                                || response.status().is_redirection(),
                            response_time_ms: started.elapsed().as_millis() as u64,
                            message: "IBKR connectivity probe completed".to_string(),
                            errors: if response.status().is_success()
                                || response.status().is_redirection()
                            {
                                vec![]
                            } else {
                                vec![format!("Unexpected status: {}", response.status())]
                            },
                        };
                    }
                    Err(err) => {
                        return ConnectionTestResult {
                            success: false,
                            response_time_ms: started.elapsed().as_millis() as u64,
                            message: "IBKR connectivity probe failed".to_string(),
                            errors: vec![err.to_string()],
                        };
                    }
                }
            }
            Err(err) => {
                return ConnectionTestResult {
                    success: false,
                    response_time_ms: started.elapsed().as_millis() as u64,
                    message: "Unable to build HTTP client for IBKR probe".to_string(),
                    errors: vec![err.to_string()],
                };
            }
        }
    }

    fn name(&self) -> &str {
        "Interactive Brokers"
    }

    fn status(&self) -> ConnectorStatus {
        self.status.clone()
    }
}

/// Polygon.io Market Data Connector (placeholder)
pub struct PolygonConnector {
    config: PolygonConfig,
    status: ConnectorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolygonConfig {
    pub api_key: String,
    pub rate_limit: u32,
}

impl PolygonConnector {
    pub fn new(config: PolygonConfig) -> Self {
        Self {
            config,
            status: ConnectorStatus::NotConfigured,
        }
    }
}

#[async_trait]
impl ApiConnector for PolygonConnector {
    async fn test_connection(&self) -> ConnectionTestResult {
        let started = Instant::now();

        if self.config.api_key.trim().is_empty() {
            return ConnectionTestResult {
                success: false,
                response_time_ms: started.elapsed().as_millis() as u64,
                message: "Polygon API key is required for validation".to_string(),
                errors: vec!["Missing POLYGON_API_KEY".to_string()],
            };
        }

        let endpoint = format!(
            "https://api.polygon.io/v3/reference/tickers/AAPL?apiKey={}",
            self.config.api_key
        );

        match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(client) => match client.get(&endpoint).send().await {
                Ok(response) => {
                    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                        return ConnectionTestResult {
                            success: false,
                            response_time_ms: started.elapsed().as_millis() as u64,
                            message: "Polygon API key is not valid or missing privileges"
                                .to_string(),
                            errors: vec!["UNAUTHORIZED".to_string()],
                        };
                    }

                    return ConnectionTestResult {
                        success: response.status().is_success()
                            || response.status().is_redirection(),
                        response_time_ms: started.elapsed().as_millis() as u64,
                        message: "Polygon connectivity probe completed".to_string(),
                        errors: if response.status().is_success()
                            || response.status().is_redirection()
                        {
                            vec![]
                        } else {
                            vec![format!("Unexpected status: {}", response.status())]
                        },
                    };
                }
                Err(err) => {
                    return ConnectionTestResult {
                        success: false,
                        response_time_ms: started.elapsed().as_millis() as u64,
                        message: "Polygon connectivity probe failed".to_string(),
                        errors: vec![err.to_string()],
                    };
                }
            },
            Err(err) => {
                return ConnectionTestResult {
                    success: false,
                    response_time_ms: started.elapsed().as_millis() as u64,
                    message: "Unable to build HTTP client for Polygon probe".to_string(),
                    errors: vec![err.to_string()],
                };
            }
        }
    }

    fn name(&self) -> &str {
        "Polygon.io"
    }

    fn status(&self) -> ConnectorStatus {
        self.status.clone()
    }
}

/// Factory за създаване на конектори
pub struct ConnectorFactory;

impl ConnectorFactory {
    /// Създава конектор по тип
    pub fn create_connector(
        connector_type: &str,
        config: HashMap<String, String>,
    ) -> Result<Box<dyn ApiConnector>, String> {
        match connector_type {
            "ibkr" => {
                let api_key = config.get("IBKR_API_KEY").ok_or("Missing IBKR_API_KEY")?;
                let api_secret = config
                    .get("IBKR_API_SECRET")
                    .ok_or("Missing IBKR_API_SECRET")?;
                let base_url = config
                    .get("IBKR_BASE_URL")
                    .cloned()
                    .unwrap_or_else(|| "https://paper-api.ibkr.com".to_string());

                let ibkr_config = IBKRConfig {
                    api_key: api_key.clone(),
                    api_secret: api_secret.clone(),
                    base_url,
                    paper_trading: true,
                };

                Ok(Box::new(InteractiveBrokersConnector::new(ibkr_config)))
            }
            "polygon" => {
                let api_key = config
                    .get("POLYGON_API_KEY")
                    .ok_or("Missing POLYGON_API_KEY")?;

                let polygon_config = PolygonConfig {
                    api_key: api_key.clone(),
                    rate_limit: 5,
                };

                Ok(Box::new(PolygonConnector::new(polygon_config)))
            }
            _ => Err(format!("Unknown connector type: {}", connector_type)),
        }
    }
}

/// Списък с налични конектори
pub fn list_available_connectors() -> Vec<ConnectorInfo> {
    vec![
        ConnectorInfo {
            id: "ibkr".to_string(),
            name: "Interactive Brokers".to_string(),
            description: "Trading and account management".to_string(),
            required_fields: vec![
                "IBKR_API_KEY".to_string(),
                "IBKR_API_SECRET".to_string(),
                "IBKR_BASE_URL".to_string(),
            ],
        },
        ConnectorInfo {
            id: "polygon".to_string(),
            name: "Polygon.io".to_string(),
            description: "Market data and historical prices".to_string(),
            required_fields: vec!["POLYGON_API_KEY".to_string()],
        },
        ConnectorInfo {
            id: "alpha_vantage".to_string(),
            name: "Alpha Vantage".to_string(),
            description: "Free market data (limited requests)".to_string(),
            required_fields: vec!["ALPHA_VANTAGE_KEY".to_string()],
        },
        ConnectorInfo {
            id: "fireblocks".to_string(),
            name: "Fireblocks".to_string(),
            description: "Crypto custody and transactions".to_string(),
            required_fields: vec![
                "FIREBLOCKS_API_KEY".to_string(),
                "FIREBLOCKS_SECRET".to_string(),
            ],
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub required_fields: Vec<String>,
}
