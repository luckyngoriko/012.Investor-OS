//! Interactive Brokers Client Portal API Client
//!
//! Low-level HTTP client for IB REST API

use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, trace, warn};

use crate::broker::{BrokerError, Result};

/// IB API Client
pub struct IbClient {
    client: Client,
    base_url: String,
    auth_token: Option<String>,
}

impl IbClient {
    /// Create a new IB client
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true) // IB uses self-signed certs
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.into(),
            auth_token: None,
        }
    }

    /// Set authentication token
    pub fn set_auth_token(&mut self, token: impl Into<String>) {
        self.auth_token = Some(token.into());
    }

    /// Check if API is accessible
    pub async fn ping(&self) -> Result<bool> {
        match self.get::<serde_json::Value>("/iserver/auth/status").await {
            Ok(_) => Ok(true),
            Err(BrokerError::Authentication(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// GET request
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        self.request::<T, ()>(Method::GET, path, None).await
    }

    /// POST request
    pub async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: B,
    ) -> Result<T> {
        self.request::<T, B>(Method::POST, path, Some(body)).await
    }

    /// DELETE request
    pub async fn delete<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        self.request::<T, ()>(Method::DELETE, path, None).await
    }

    // Private request method
    async fn request<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<B>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        trace!("IB API Request: {} {}", method, url);

        let mut request = self.client.request(method, &url);

        // Add auth token if available
        if let Some(token) = &self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Add body for POST/PUT
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.map_err(|e| {
            error!("IB API request failed: {}", e);
            BrokerError::Connection(e.to_string())
        })?;

        let status = response.status();
        trace!("IB API Response: {}", status);

        match status {
            StatusCode::OK | StatusCode::CREATED => response
                .json::<T>()
                .await
                .map_err(|e| BrokerError::ExternalApi(format!("Failed to parse response: {}", e))),
            StatusCode::UNAUTHORIZED => Err(BrokerError::Authentication(
                "Invalid credentials".to_string(),
            )),
            StatusCode::FORBIDDEN => {
                Err(BrokerError::Authentication("Access forbidden".to_string()))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("IB API rate limit exceeded");
                Err(BrokerError::RateLimit)
            }
            StatusCode::BAD_REQUEST => {
                let text = response.text().await.unwrap_or_default();
                Err(BrokerError::InvalidOrder(format!("Bad request: {}", text)))
            }
            _ => {
                let text = response.text().await.unwrap_or_default();
                error!("IB API error: {} - {}", status, text);
                Err(BrokerError::ExternalApi(format!(
                    "HTTP {}: {}",
                    status, text
                )))
            }
        }
    }
}

/// IB Account info response
#[derive(Debug, Clone, Deserialize)]
pub struct IbAccountInfo {
    #[serde(rename = "id")]
    pub account_id: String,
    #[serde(rename = "accountId")]
    pub account_id_alt: Option<String>,
    #[serde(rename = "accountVan")]
    pub account_van: Option<String>,
    #[serde(rename = "accountTitle")]
    pub account_title: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "accountAlias")]
    pub account_alias: Option<String>,
}

/// IB authentication status
#[derive(Debug, Clone, Deserialize)]
pub struct IbAuthStatus {
    pub authenticated: bool,
    pub competing: bool,
    pub connected: bool,
    #[serde(rename = "message")]
    pub message: Option<String>,
}

/// IB tickle response (keep session alive)
#[derive(Debug, Clone, Deserialize)]
pub struct IbTickleResponse {
    pub session: Option<String>,
    #[serde(rename = "ssoExpires")]
    pub sso_expires: Option<i64>,
    pub collission: Option<bool>,
    #[serde(rename = "userId")]
    pub user_id: Option<i64>,
    #[serde(rename = "iserver")]
    pub iserver: Option<IbIserverAuth>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IbIserverAuth {
    pub auth_time: Option<i64>,
    pub compete_time: Option<i64>,
    pub competion_time: Option<i64>,
}

/// IB search result for contracts
#[derive(Debug, Clone, Deserialize)]
pub struct IbContractSearch {
    pub conid: i64,
    #[serde(rename = "companyHeader")]
    pub company_header: String,
    #[serde(rename = "companyName")]
    pub company_name: String,
    pub symbol: String,
    #[serde(rename = "securityType")]
    pub security_type: String,
    pub listing_exchange: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ib_client_creation() {
        let client = IbClient::new("https://localhost:5000/v1/api");
        assert_eq!(client.base_url, "https://localhost:5000/v1/api");
    }

    #[test]
    fn test_auth_token() {
        let mut client = IbClient::new("https://localhost:5000");
        client.set_auth_token("test-token-123");
        assert_eq!(client.auth_token, Some("test-token-123".to_string()));
    }
}
