//! Interactive Brokers Client Portal REST Connector
//!
//! Wave 2 Task 8: Lightweight HTTP REST client for IBKR Client Portal API.
//! Supports stocks + options order placement, position retrieval, and account info.
//!
//! Configuration:
//! - `IBKR_GATEWAY_URL` env var (default: `https://localhost:5000/v1/api`)
//! - Paper trading mode by default (TWS paper port 7497, live port 7496)

use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::broker::{BrokerError, Result};

/// Default Client Portal API base URL
const DEFAULT_GATEWAY_URL: &str = "https://localhost:5000/v1/api";

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// IBKR Client Portal REST client.
///
/// Communicates with the IB Client Portal Gateway over HTTPS.
/// The gateway must be running locally (or reachable via `IBKR_GATEWAY_URL`).
pub struct IbkrClient {
    http: Client,
    base_url: String,
    /// Currently selected account ID (populated after `connect()`).
    account_id: Option<String>,
    /// Whether we are in paper-trading mode.
    paper: bool,
}

impl IbkrClient {
    /// Create a new client from environment variables.
    ///
    /// Reads:
    /// - `IBKR_GATEWAY_URL` – base URL (default `https://localhost:5000/v1/api`)
    /// - `IBKR_PAPER` – `"true"` (default) or `"false"`
    pub fn from_env() -> Self {
        let base_url =
            std::env::var("IBKR_GATEWAY_URL").unwrap_or_else(|_| DEFAULT_GATEWAY_URL.to_string());
        let paper = std::env::var("IBKR_PAPER")
            .map(|v| v != "false")
            .unwrap_or(true);

        Self::new(base_url, paper)
    }

    /// Create a new client with explicit parameters.
    pub fn new(base_url: impl Into<String>, paper: bool) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true) // IB gateway uses self-signed certs
            .build()
            .expect("Failed to create HTTP client for IBKR");

        Self {
            http,
            base_url: base_url.into(),
            account_id: None,
            paper,
        }
    }

    /// Whether the client is configured for paper trading.
    pub fn is_paper(&self) -> bool {
        self.paper
    }

    /// Currently selected account ID (available after `connect()`).
    pub fn account_id(&self) -> Option<&str> {
        self.account_id.as_deref()
    }

    // -----------------------------------------------------------------------
    // Public API methods
    // -----------------------------------------------------------------------

    /// Connect: verify authentication and select the first account.
    pub async fn connect(&mut self) -> Result<()> {
        info!(
            "IBKR connecting to {} (paper={})",
            self.base_url, self.paper
        );

        // 1. Check auth status
        let status: IbkrAuthStatus = self.get("/iserver/auth/status").await?;
        if !status.authenticated {
            warn!("IBKR gateway reports not authenticated – please log in via the web UI");
            return Err(BrokerError::Authentication(
                "Not authenticated with IBKR Client Portal – open the gateway web UI to log in"
                    .to_string(),
            ));
        }

        // 2. Fetch accounts and pick the first one
        let accounts: Vec<IbkrAccount> = self.get("/portfolio/accounts").await?;
        let account = accounts.first().ok_or_else(|| {
            BrokerError::Connection("No accounts returned from IBKR gateway".to_string())
        })?;

        self.account_id = Some(account.account_id.clone());
        info!(
            "IBKR connected – account {} ({})",
            account.account_id,
            if self.paper { "paper" } else { "live" }
        );

        Ok(())
    }

    /// Retrieve account summary for the connected account.
    pub async fn get_account(&self) -> Result<IbkrAccountSummary> {
        let acct = self.require_account()?;
        let path = format!("/portfolio/{}/summary", acct);
        self.get(&path).await
    }

    /// Retrieve all positions for the connected account.
    /// Uses page 0 which returns all positions.
    pub async fn get_positions(&self) -> Result<Vec<IbkrPosition>> {
        let acct = self.require_account()?;
        let path = format!("/portfolio/{}/positions/0", acct);
        self.get(&path).await
    }

    /// Place an order via the Client Portal iserver endpoint.
    pub async fn place_order(&self, request: &IbkrOrderRequest) -> Result<Vec<IbkrOrderReply>> {
        let acct = self.require_account()?;
        let path = format!("/iserver/account/{}/orders", acct);

        // The API wraps orders in an `orders` array
        let body = IbkrOrdersPayload {
            orders: vec![request.clone()],
        };

        self.post(&path, &body).await
    }

    /// Cancel an existing order.
    pub async fn cancel_order(&self, order_id: &str) -> Result<IbkrCancelReply> {
        let acct = self.require_account()?;
        let path = format!("/iserver/account/{}/order/{}", acct, order_id);
        self.delete(&path).await
    }

    // -----------------------------------------------------------------------
    // HTTP helpers
    // -----------------------------------------------------------------------

    fn require_account(&self) -> Result<String> {
        self.account_id.clone().ok_or_else(|| {
            BrokerError::Authentication(
                "Not connected – call connect() first to select an account".to_string(),
            )
        })
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("IBKR GET {}", url);

        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| BrokerError::Connection(format!("IBKR GET {} failed: {}", path, e)))?;

        self.handle_response(resp, path).await
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("IBKR POST {}", url);

        let resp =
            self.http.post(&url).json(body).send().await.map_err(|e| {
                BrokerError::Connection(format!("IBKR POST {} failed: {}", path, e))
            })?;

        self.handle_response(resp, path).await
    }

    async fn delete<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("IBKR DELETE {}", url);

        let resp =
            self.http.delete(&url).send().await.map_err(|e| {
                BrokerError::Connection(format!("IBKR DELETE {} failed: {}", path, e))
            })?;

        self.handle_response(resp, path).await
    }

    async fn handle_response<T: for<'de> Deserialize<'de>>(
        &self,
        resp: reqwest::Response,
        path: &str,
    ) -> Result<T> {
        let status = resp.status();
        match status {
            StatusCode::OK | StatusCode::CREATED => resp
                .json::<T>()
                .await
                .map_err(|e| BrokerError::ExternalApi(format!("IBKR parse error: {}", e))),
            StatusCode::UNAUTHORIZED => Err(BrokerError::Authentication(
                "IBKR unauthorized – session may have expired".to_string(),
            )),
            StatusCode::FORBIDDEN => Err(BrokerError::Authentication(
                "IBKR access forbidden".to_string(),
            )),
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("IBKR rate limit hit on {}", path);
                Err(BrokerError::RateLimit)
            }
            StatusCode::BAD_REQUEST => {
                let text = resp.text().await.unwrap_or_default();
                Err(BrokerError::InvalidOrder(format!(
                    "IBKR bad request on {}: {}",
                    path, text
                )))
            }
            _ => {
                let text = resp.text().await.unwrap_or_default();
                error!("IBKR error {} on {}: {}", status, path, text);
                Err(BrokerError::ExternalApi(format!(
                    "IBKR HTTP {} on {}: {}",
                    status, path, text
                )))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// API response / request types
// ---------------------------------------------------------------------------

/// Authentication status from `/iserver/auth/status`.
#[derive(Debug, Clone, Deserialize)]
pub struct IbkrAuthStatus {
    pub authenticated: bool,
    #[serde(default)]
    pub competing: bool,
    #[serde(default)]
    pub connected: bool,
    pub message: Option<String>,
}

/// Account entry from `/portfolio/accounts`.
#[derive(Debug, Clone, Deserialize)]
pub struct IbkrAccount {
    #[serde(alias = "id", alias = "accountId")]
    pub account_id: String,
    #[serde(rename = "accountTitle")]
    pub account_title: Option<String>,
    #[serde(rename = "accountAlias")]
    pub account_alias: Option<String>,
    #[serde(rename = "type")]
    pub account_type: Option<String>,
}

/// Account summary from `/portfolio/{accountId}/summary`.
#[derive(Debug, Clone, Deserialize)]
pub struct IbkrAccountSummary {
    /// Available cash
    #[serde(alias = "totalcashvalue", alias = "cashbalance", default)]
    pub total_cash_value: Option<IbkrAmountField>,

    /// Net liquidation value
    #[serde(alias = "netliquidationvalue", default)]
    pub net_liquidation_value: Option<IbkrAmountField>,

    /// Buying power
    #[serde(alias = "buyingpower", default)]
    pub buying_power: Option<IbkrAmountField>,

    /// Unrealized P&L
    #[serde(alias = "unrealizedpnl", default)]
    pub unrealized_pnl: Option<IbkrAmountField>,

    /// Realized P&L
    #[serde(alias = "realizedpnl", default)]
    pub realized_pnl: Option<IbkrAmountField>,
}

/// An amount field in the IBKR summary response.
/// The API returns `{ "amount": 12345.67, "currency": "USD", ... }` for each metric.
#[derive(Debug, Clone, Deserialize)]
pub struct IbkrAmountField {
    pub amount: Option<Decimal>,
    pub currency: Option<String>,
}

/// Position from `/portfolio/{accountId}/positions/0`.
#[derive(Debug, Clone, Deserialize)]
pub struct IbkrPosition {
    /// Contract ID
    pub conid: Option<i64>,

    /// Ticker / contract description
    #[serde(alias = "contractDesc", alias = "ticker", default)]
    pub contract_desc: Option<String>,

    /// Position quantity (positive = long, negative = short)
    #[serde(default)]
    pub position: Decimal,

    /// Average cost per share
    #[serde(rename = "avgCost", default)]
    pub avg_cost: Decimal,

    /// Current market price
    #[serde(rename = "mktPrice", default)]
    pub mkt_price: Option<Decimal>,

    /// Current market value
    #[serde(rename = "mktValue", default)]
    pub mkt_value: Option<Decimal>,

    /// Unrealized P&L
    #[serde(rename = "unrealizedPnl", default)]
    pub unrealized_pnl: Option<Decimal>,

    /// Realized P&L
    #[serde(rename = "realizedPnl", default)]
    pub realized_pnl: Option<Decimal>,

    /// Asset class (STK, OPT, FUT, etc.)
    #[serde(rename = "assetClass", default)]
    pub asset_class: Option<String>,

    /// Currency
    #[serde(default)]
    pub currency: Option<String>,
}

/// Order request body for `/iserver/account/{accountId}/orders`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbkrOrderRequest {
    /// IB contract ID
    pub conid: i64,

    /// Order side: `"BUY"` or `"SELL"`
    pub side: String,

    /// Order type: `"MKT"`, `"LMT"`, `"STP"`, `"STP_LMT"`, `"TRAIL"`
    #[serde(rename = "orderType")]
    pub order_type: String,

    /// Quantity
    #[serde(rename = "quantity")]
    pub quantity: Decimal,

    /// Limit price (required for LMT / STP_LMT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,

    /// Aux / stop price (required for STP / STP_LMT / TRAIL)
    #[serde(rename = "auxPrice", skip_serializing_if = "Option::is_none")]
    pub aux_price: Option<Decimal>,

    /// Time in force: `"DAY"`, `"GTC"`, `"IOC"`, `"FOK"`
    pub tif: String,

    /// Whether to trade outside regular trading hours
    #[serde(rename = "outsideRTH", default)]
    pub outside_rth: bool,

    /// Ticker symbol (informational, conid is authoritative)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticker: Option<String>,

    /// Client order reference (our internal order UUID)
    #[serde(rename = "cOID", skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,

    /// Security type: `"STK"`, `"OPT"`, etc.
    #[serde(rename = "secType", skip_serializing_if = "Option::is_none")]
    pub sec_type: Option<String>,

    /// Listing exchange (optional)
    #[serde(rename = "listingExchange", skip_serializing_if = "Option::is_none")]
    pub listing_exchange: Option<String>,
}

/// Wrapper for the orders array sent to the API.
#[derive(Debug, Clone, Serialize)]
struct IbkrOrdersPayload {
    orders: Vec<IbkrOrderRequest>,
}

/// A single reply from the order placement endpoint.
/// The API may return order confirmations or message prompts.
#[derive(Debug, Clone, Deserialize)]
pub struct IbkrOrderReply {
    /// Assigned order ID (present on success)
    pub order_id: Option<String>,

    /// Status/order status string
    pub order_status: Option<String>,

    /// Warning or confirmation message ID (requires reply)
    pub id: Option<String>,

    /// Human-readable message
    pub message: Option<Vec<String>>,
}

/// Reply from the cancel order endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct IbkrCancelReply {
    /// Order ID that was cancelled
    pub order_id: Option<String>,

    /// Result message
    pub msg: Option<String>,

    /// Whether the cancel succeeded
    #[serde(default)]
    pub conid: Option<i64>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn test_ibkr_client_default_url() {
        let client = IbkrClient::new(DEFAULT_GATEWAY_URL, true);
        assert_eq!(client.base_url, "https://localhost:5000/v1/api");
        assert!(client.is_paper());
        assert!(client.account_id().is_none());
    }

    #[test]
    fn test_ibkr_client_custom_url() {
        let client = IbkrClient::new("https://mygateway:5555/v1/api", false);
        assert_eq!(client.base_url, "https://mygateway:5555/v1/api");
        assert!(!client.is_paper());
    }

    #[test]
    fn test_order_request_serialization_market_buy() {
        let req = IbkrOrderRequest {
            conid: 265598,
            side: "BUY".to_string(),
            order_type: "MKT".to_string(),
            quantity: dec!(100),
            price: None,
            aux_price: None,
            tif: "DAY".to_string(),
            outside_rth: false,
            ticker: Some("AAPL".to_string()),
            client_order_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            sec_type: Some("STK".to_string()),
            listing_exchange: None,
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["conid"], 265598);
        assert_eq!(json["side"], "BUY");
        assert_eq!(json["orderType"], "MKT");
        assert_eq!(json["quantity"], "100");
        assert_eq!(json["tif"], "DAY");
        assert_eq!(json["ticker"], "AAPL");
        assert_eq!(json["cOID"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(json["secType"], "STK");
        // price and auxPrice should be absent (skip_serializing_if)
        assert!(json.get("price").is_none());
        assert!(json.get("auxPrice").is_none());
        assert!(json.get("listingExchange").is_none());
    }

    #[test]
    fn test_order_request_serialization_limit_sell() {
        let req = IbkrOrderRequest {
            conid: 265598,
            side: "SELL".to_string(),
            order_type: "LMT".to_string(),
            quantity: dec!(50),
            price: Some(dec!(185.50)),
            aux_price: None,
            tif: "GTC".to_string(),
            outside_rth: true,
            ticker: Some("AAPL".to_string()),
            client_order_id: None,
            sec_type: Some("STK".to_string()),
            listing_exchange: Some("SMART".to_string()),
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["side"], "SELL");
        assert_eq!(json["orderType"], "LMT");
        assert_eq!(json["price"], "185.50");
        assert_eq!(json["tif"], "GTC");
        assert_eq!(json["outsideRTH"], true);
        assert_eq!(json["listingExchange"], "SMART");
        // cOID should be absent
        assert!(json.get("cOID").is_none());
    }

    #[test]
    fn test_order_request_serialization_stop_order() {
        let req = IbkrOrderRequest {
            conid: 8314,
            side: "SELL".to_string(),
            order_type: "STP".to_string(),
            quantity: dec!(200),
            price: None,
            aux_price: Some(dec!(140.00)),
            tif: "DAY".to_string(),
            outside_rth: false,
            ticker: Some("IBM".to_string()),
            client_order_id: None,
            sec_type: Some("STK".to_string()),
            listing_exchange: None,
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["orderType"], "STP");
        assert_eq!(json["auxPrice"], "140.00");
        assert!(json.get("price").is_none());
    }

    #[test]
    fn test_order_request_serialization_options() {
        let req = IbkrOrderRequest {
            conid: 500123456,
            side: "BUY".to_string(),
            order_type: "LMT".to_string(),
            quantity: dec!(10),
            price: Some(dec!(3.50)),
            aux_price: None,
            tif: "DAY".to_string(),
            outside_rth: false,
            ticker: Some("AAPL 20260620C200".to_string()),
            client_order_id: None,
            sec_type: Some("OPT".to_string()),
            listing_exchange: None,
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["secType"], "OPT");
        assert_eq!(json["price"], "3.50");
        assert_eq!(json["quantity"], "10");
    }

    #[test]
    fn test_orders_payload_wraps_in_array() {
        let req = IbkrOrderRequest {
            conid: 265598,
            side: "BUY".to_string(),
            order_type: "MKT".to_string(),
            quantity: dec!(1),
            price: None,
            aux_price: None,
            tif: "DAY".to_string(),
            outside_rth: false,
            ticker: None,
            client_order_id: None,
            sec_type: None,
            listing_exchange: None,
        };

        let payload = IbkrOrdersPayload { orders: vec![req] };

        let json = serde_json::to_value(&payload).unwrap();
        assert!(json["orders"].is_array());
        assert_eq!(json["orders"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_require_account_fails_before_connect() {
        let client = IbkrClient::new(DEFAULT_GATEWAY_URL, true);
        let result = client.require_account();
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_status_deserialization() {
        let json = r#"{"authenticated":true,"competing":false,"connected":true,"message":""}"#;
        let status: IbkrAuthStatus = serde_json::from_str(json).unwrap();
        assert!(status.authenticated);
        assert!(!status.competing);
        assert!(status.connected);
    }

    #[test]
    fn test_position_deserialization() {
        let json = r#"{
            "conid": 265598,
            "contractDesc": "AAPL",
            "position": 100,
            "avgCost": 150.25,
            "mktPrice": 175.50,
            "mktValue": 17550.00,
            "unrealizedPnl": 2525.00,
            "realizedPnl": 0,
            "assetClass": "STK",
            "currency": "USD"
        }"#;
        let pos: IbkrPosition = serde_json::from_str(json).unwrap();
        assert_eq!(pos.conid, Some(265598));
        assert_eq!(pos.contract_desc.as_deref(), Some("AAPL"));
        assert_eq!(pos.position, dec!(100));
        assert_eq!(pos.avg_cost, dec!(150.25));
        assert_eq!(pos.mkt_price, Some(dec!(175.50)));
        assert_eq!(pos.asset_class.as_deref(), Some("STK"));
    }

    #[test]
    fn test_cancel_reply_deserialization() {
        let json = r#"{"order_id":"12345","msg":"Order 12345 cancelled","conid":265598}"#;
        let reply: IbkrCancelReply = serde_json::from_str(json).unwrap();
        assert_eq!(reply.order_id.as_deref(), Some("12345"));
        assert_eq!(reply.msg.as_deref(), Some("Order 12345 cancelled"));
    }

    #[test]
    fn test_order_reply_deserialization() {
        let json = r#"{"order_id":"67890","order_status":"PreSubmitted"}"#;
        let reply: IbkrOrderReply = serde_json::from_str(json).unwrap();
        assert_eq!(reply.order_id.as_deref(), Some("67890"));
        assert_eq!(reply.order_status.as_deref(), Some("PreSubmitted"));
    }

    #[test]
    fn test_account_deserialization() {
        let json = r#"{"accountId":"DU12345","accountTitle":"Paper","type":"individual"}"#;
        let acct: IbkrAccount = serde_json::from_str(json).unwrap();
        assert_eq!(acct.account_id, "DU12345");
        assert_eq!(acct.account_title.as_deref(), Some("Paper"));
        assert_eq!(acct.account_type.as_deref(), Some("individual"));
    }
}
