//! OANDA REST API v20 Connector for Forex Trading
//!
//! Wave 2 Task 9: Full-featured HTTP REST client for OANDA's v20 API.
//! Supports account info, open positions, live pricing, market orders,
//! and position closing.
//!
//! Configuration:
//! - `OANDA_API_KEY` env var (Bearer token for Authorization header)
//! - `OANDA_ACCOUNT_ID` env var
//! - `OANDA_PRACTICE` env var (`"true"` = demo, `"false"` = live)
//!
//! Base URLs:
//! - Practice: `https://api-fxpractice.oanda.com`
//! - Live:     `https://api-fxtrade.oanda.com`

use reqwest::{Client, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, warn};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Practice/demo API base URL
const PRACTICE_URL: &str = "https://api-fxpractice.oanda.com";

/// Live/real trading API base URL
const LIVE_URL: &str = "https://api-fxtrade.oanda.com";

/// Major forex pairs available on OANDA
pub const MAJOR_PAIRS: &[&str] = &[
    "EUR_USD", "GBP_USD", "USD_JPY", "USD_CHF", "AUD_USD", "USD_CAD", "NZD_USD",
];

/// Extended set of forex pairs
pub const ALL_PAIRS: &[&str] = &[
    "EUR_USD", "GBP_USD", "USD_JPY", "USD_CHF", "AUD_USD", "USD_CAD", "NZD_USD", "EUR_GBP",
    "EUR_JPY", "EUR_CHF", "EUR_AUD", "EUR_CAD", "EUR_NZD", "GBP_JPY", "GBP_CHF", "GBP_AUD",
    "GBP_CAD", "GBP_NZD", "AUD_JPY", "AUD_CHF", "AUD_CAD", "AUD_NZD", "CAD_JPY", "CAD_CHF",
    "CHF_JPY", "NZD_JPY", "NZD_CHF", "NZD_CAD",
];

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// OANDA-specific errors
#[derive(Debug, thiserror::Error)]
pub enum OandaError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Order rejected: {0}")]
    OrderRejected(String),
}

/// Result alias for OANDA operations
pub type Result<T> = std::result::Result<T, OandaError>;

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// OANDA REST v20 API client.
///
/// Communicates with the OANDA REST API using Bearer token authentication.
/// Supports both practice (demo) and live environments.
#[derive(Debug, Clone)]
pub struct OandaClient {
    api_key: String,
    account_id: String,
    http: Client,
    base_url: String,
    /// Whether this client targets the practice/demo environment.
    practice: bool,
}

impl OandaClient {
    /// Create a new client from environment variables.
    ///
    /// Reads:
    /// - `OANDA_API_KEY` – Bearer API key (required, defaults to empty)
    /// - `OANDA_ACCOUNT_ID` – Account ID (required, defaults to empty)
    /// - `OANDA_PRACTICE` – `"true"` (default) or `"false"`
    pub fn from_env() -> Self {
        let api_key = std::env::var("OANDA_API_KEY").unwrap_or_default();
        let account_id = std::env::var("OANDA_ACCOUNT_ID").unwrap_or_default();
        let is_practice = std::env::var("OANDA_PRACTICE")
            .map(|v| v != "false")
            .unwrap_or(true);

        Self::new(api_key, account_id, is_practice)
    }

    /// Create a new OANDA client with explicit parameters.
    ///
    /// - `api_key` – OANDA API key (v20 personal access token)
    /// - `account_id` – OANDA account ID (e.g. `"101-001-12345678-001"`)
    /// - `is_practice` – `true` for demo/practice, `false` for live trading
    pub fn new(
        api_key: impl Into<String>,
        account_id: impl Into<String>,
        is_practice: bool,
    ) -> Self {
        let base_url = if is_practice {
            PRACTICE_URL.to_string()
        } else {
            LIVE_URL.to_string()
        };

        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client for OANDA");

        Self {
            api_key: api_key.into(),
            account_id: account_id.into(),
            http,
            base_url,
            practice: is_practice,
        }
    }

    /// Whether this client is configured for the practice/demo environment.
    pub fn is_practice(&self) -> bool {
        self.practice
    }

    /// The API base URL in use.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The account ID in use.
    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    // -----------------------------------------------------------------------
    // Public API methods
    // -----------------------------------------------------------------------

    /// Get account information.
    ///
    /// `GET /v3/accounts/{account_id}`
    pub async fn get_account(&self) -> Result<OandaAccountInfo> {
        let path = format!("/v3/accounts/{}", self.account_id);
        let wrapper: AccountInfoResponse = self.get(&path).await?;
        Ok(wrapper.account)
    }

    /// Get all open positions for the account.
    ///
    /// `GET /v3/accounts/{account_id}/openPositions`
    pub async fn get_positions(&self) -> Result<Vec<OandaPosition>> {
        let path = format!("/v3/accounts/{}/openPositions", self.account_id);
        let wrapper: PositionsResponse = self.get(&path).await?;
        Ok(wrapper.positions)
    }

    /// Get live pricing for one or more instruments.
    ///
    /// `GET /v3/accounts/{account_id}/pricing?instruments=EUR_USD,GBP_USD`
    ///
    /// `instruments` is a slice of OANDA instrument names (e.g. `["EUR_USD", "GBP_USD"]`).
    pub async fn get_pricing(&self, instruments: &[&str]) -> Result<Vec<OandaPrice>> {
        let joined = instruments.join(",");
        let path = format!(
            "/v3/accounts/{}/pricing?instruments={}",
            self.account_id, joined
        );
        let wrapper: PricingResponse = self.get(&path).await?;
        Ok(wrapper.prices)
    }

    /// Place a market order.
    ///
    /// `POST /v3/accounts/{account_id}/orders`
    ///
    /// - `instrument` – e.g. `"EUR_USD"`
    /// - `units` – number of units (positive = buy, negative = sell in OANDA convention,
    ///   but this method uses `side` for clarity)
    /// - `side` – `"buy"` or `"sell"`
    pub async fn place_market_order(
        &self,
        instrument: &str,
        units: Decimal,
        side: &str,
    ) -> Result<OandaOrderResponse> {
        let path = format!("/v3/accounts/{}/orders", self.account_id);

        // OANDA uses signed units: positive = long, negative = short
        let signed_units = match side.to_lowercase().as_str() {
            "buy" | "long" => units.to_string(),
            "sell" | "short" => format!("-{}", units),
            _ => return Err(OandaError::Api(format!("Invalid side: {}", side))),
        };

        let body = OandaOrderRequest {
            order: MarketOrderBody {
                instrument: instrument.to_string(),
                units: signed_units,
                order_type: "MARKET".to_string(),
                time_in_force: "FOK".to_string(),
                position_fill: "DEFAULT".to_string(),
            },
        };

        self.post(&path, &body).await
    }

    /// Close an open position for an instrument.
    ///
    /// `PUT /v3/accounts/{account_id}/positions/{instrument}/close`
    ///
    /// Sends `longUnits: "ALL"` and `shortUnits: "ALL"` to close the entire position.
    pub async fn close_position(&self, instrument: &str) -> Result<OandaCloseResponse> {
        let path = format!(
            "/v3/accounts/{}/positions/{}/close",
            self.account_id, instrument
        );

        let body = ClosePositionBody {
            long_units: "ALL".to_string(),
            short_units: "ALL".to_string(),
        };

        self.put(&path, &body).await
    }

    // -----------------------------------------------------------------------
    // HTTP helpers
    // -----------------------------------------------------------------------

    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("OANDA GET {}", url);

        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| OandaError::Network(format!("OANDA GET {} failed: {}", path, e)))?;

        self.handle_response(resp, path).await
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("OANDA POST {}", url);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| OandaError::Network(format!("OANDA POST {} failed: {}", path, e)))?;

        self.handle_response(resp, path).await
    }

    async fn put<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("OANDA PUT {}", url);

        let resp = self
            .http
            .put(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| OandaError::Network(format!("OANDA PUT {} failed: {}", path, e)))?;

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
                .map_err(|e| OandaError::Parse(format!("OANDA parse error on {}: {}", path, e))),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                let text = resp.text().await.unwrap_or_default();
                Err(OandaError::Authentication(format!(
                    "OANDA auth failed on {}: {}",
                    path, text
                )))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("OANDA rate limit hit on {}", path);
                Err(OandaError::RateLimit)
            }
            StatusCode::BAD_REQUEST => {
                let text = resp.text().await.unwrap_or_default();
                Err(OandaError::OrderRejected(format!(
                    "OANDA bad request on {}: {}",
                    path, text
                )))
            }
            StatusCode::NOT_FOUND => {
                let text = resp.text().await.unwrap_or_default();
                Err(OandaError::Api(format!(
                    "OANDA resource not found on {}: {}",
                    path, text
                )))
            }
            _ => {
                let text = resp.text().await.unwrap_or_default();
                error!("OANDA error {} on {}: {}", status, path, text);
                Err(OandaError::Api(format!(
                    "OANDA HTTP {} on {}: {}",
                    status, path, text
                )))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// API request types
// ---------------------------------------------------------------------------

/// Top-level order request body sent to `POST /v3/accounts/{id}/orders`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OandaOrderRequest {
    pub order: MarketOrderBody,
}

/// Market order body inside the order request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOrderBody {
    /// Instrument name, e.g. `"EUR_USD"`
    pub instrument: String,

    /// Signed units string: positive = buy, negative = sell.
    /// OANDA API requires this as a string.
    pub units: String,

    /// Order type: `"MARKET"`, `"LIMIT"`, `"STOP"`, etc.
    #[serde(rename = "type")]
    pub order_type: String,

    /// Time in force: `"FOK"`, `"GTC"`, `"IOC"`, etc.
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,

    /// Position fill behavior: `"DEFAULT"`, `"REDUCE_FIRST"`, `"REDUCE_ONLY"`, `"OPEN_ONLY"`
    #[serde(rename = "positionFill")]
    pub position_fill: String,
}

/// Body for `PUT /v3/accounts/{id}/positions/{instrument}/close`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClosePositionBody {
    /// Long units to close. `"ALL"` closes entire long side.
    #[serde(rename = "longUnits")]
    long_units: String,

    /// Short units to close. `"ALL"` closes entire short side.
    #[serde(rename = "shortUnits")]
    short_units: String,
}

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

/// Wrapper for `GET /v3/accounts/{id}`.
#[derive(Debug, Clone, Deserialize)]
struct AccountInfoResponse {
    account: OandaAccountInfo,
}

/// OANDA account information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OandaAccountInfo {
    /// Account ID
    #[serde(default)]
    pub id: String,

    /// Account balance
    #[serde(default)]
    pub balance: Decimal,

    /// Total profit/loss
    #[serde(default)]
    pub pl: Decimal,

    /// Unrealized P&L on open positions
    #[serde(rename = "unrealizedPL", default)]
    pub unrealized_pl: Decimal,

    /// Margin used
    #[serde(rename = "marginUsed", default)]
    pub margin_used: Decimal,

    /// Margin available
    #[serde(rename = "marginAvailable", default)]
    pub margin_available: Decimal,

    /// Net asset value (balance + unrealized P&L)
    #[serde(rename = "NAV", default)]
    pub nav: Decimal,

    /// Account currency (e.g. `"USD"`, `"EUR"`)
    #[serde(default)]
    pub currency: String,

    /// Number of open trades
    #[serde(rename = "openTradeCount", default)]
    pub open_trade_count: i64,

    /// Number of open positions
    #[serde(rename = "openPositionCount", default)]
    pub open_position_count: i64,
}

/// Wrapper for `GET /v3/accounts/{id}/openPositions`.
#[derive(Debug, Clone, Deserialize)]
struct PositionsResponse {
    positions: Vec<OandaPosition>,
}

/// An open position on OANDA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OandaPosition {
    /// Instrument name (e.g. `"EUR_USD"`)
    pub instrument: String,

    /// Profit/loss on this position
    #[serde(default)]
    pub pl: Decimal,

    /// Unrealized P&L
    #[serde(rename = "unrealizedPL", default)]
    pub unrealized_pl: Decimal,

    /// Long side details
    #[serde(default, rename = "long")]
    pub long_side: Option<PositionSide>,

    /// Short side details
    #[serde(default, rename = "short")]
    pub short_side: Option<PositionSide>,
}

/// One side (long or short) of a position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSide {
    /// Number of units (string in OANDA API)
    #[serde(default)]
    pub units: String,

    /// Average entry price
    #[serde(rename = "averagePrice", default)]
    pub average_price: Option<Decimal>,

    /// Unrealized P&L for this side
    #[serde(rename = "unrealizedPL", default)]
    pub unrealized_pl: Option<Decimal>,

    /// P&L for this side
    #[serde(default)]
    pub pl: Option<Decimal>,
}

/// Wrapper for `GET /v3/accounts/{id}/pricing`.
#[derive(Debug, Clone, Deserialize)]
struct PricingResponse {
    prices: Vec<OandaPrice>,
}

/// Live price for an instrument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OandaPrice {
    /// Instrument name
    pub instrument: String,

    /// Best ask price (for buying)
    #[serde(rename = "closeoutAsk", default)]
    pub closeout_ask: Decimal,

    /// Best bid price (for selling)
    #[serde(rename = "closeoutBid", default)]
    pub closeout_bid: Decimal,

    /// Price status: `"tradeable"`, `"non-tradeable"`, `"invalid"`
    #[serde(default)]
    pub status: Option<String>,

    /// Whether the instrument is currently tradeable
    #[serde(default)]
    pub tradeable: Option<bool>,
}

/// Response from placing an order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OandaOrderResponse {
    /// The created order transaction
    #[serde(rename = "orderCreateTransaction", default)]
    pub order_create_transaction: Option<OrderTransaction>,

    /// The fill transaction (if order was immediately filled)
    #[serde(rename = "orderFillTransaction", default)]
    pub order_fill_transaction: Option<FillTransaction>,

    /// Related transaction IDs
    #[serde(rename = "relatedTransactionIDs", default)]
    pub related_transaction_ids: Vec<String>,
}

/// Order creation transaction details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderTransaction {
    /// Transaction ID
    pub id: Option<String>,

    /// Transaction type (e.g. `"MARKET_ORDER"`)
    #[serde(rename = "type", default)]
    pub transaction_type: Option<String>,

    /// Instrument
    #[serde(default)]
    pub instrument: Option<String>,

    /// Units (signed string)
    #[serde(default)]
    pub units: Option<String>,

    /// Time in force
    #[serde(rename = "timeInForce", default)]
    pub time_in_force: Option<String>,

    /// Reason for the order
    #[serde(default)]
    pub reason: Option<String>,
}

/// Order fill transaction details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillTransaction {
    /// Transaction ID
    pub id: Option<String>,

    /// Fill price
    #[serde(default)]
    pub price: Option<Decimal>,

    /// Units filled (signed)
    #[serde(default)]
    pub units: Option<String>,

    /// Profit/loss realized by this fill
    #[serde(default)]
    pub pl: Option<Decimal>,

    /// Instrument
    #[serde(default)]
    pub instrument: Option<String>,

    /// Account balance after fill
    #[serde(rename = "accountBalance", default)]
    pub account_balance: Option<Decimal>,
}

/// Response from closing a position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OandaCloseResponse {
    /// Long-side close transaction
    #[serde(rename = "longOrderFillTransaction", default)]
    pub long_order_fill_transaction: Option<FillTransaction>,

    /// Short-side close transaction
    #[serde(rename = "shortOrderFillTransaction", default)]
    pub short_order_fill_transaction: Option<FillTransaction>,

    /// Related transaction IDs
    #[serde(rename = "relatedTransactionIDs", default)]
    pub related_transaction_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn test_client_practice_url() {
        let client = OandaClient::new("test-key", "101-001-12345-001", true);
        assert_eq!(client.base_url(), PRACTICE_URL);
        assert!(client.is_practice());
        assert_eq!(client.account_id(), "101-001-12345-001");
    }

    #[test]
    fn test_client_live_url() {
        let client = OandaClient::new("test-key", "101-001-12345-001", false);
        assert_eq!(client.base_url(), LIVE_URL);
        assert!(!client.is_practice());
    }

    #[test]
    fn test_order_request_serialization_buy() {
        let req = OandaOrderRequest {
            order: MarketOrderBody {
                instrument: "EUR_USD".to_string(),
                units: "10000".to_string(),
                order_type: "MARKET".to_string(),
                time_in_force: "FOK".to_string(),
                position_fill: "DEFAULT".to_string(),
            },
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["order"]["instrument"], "EUR_USD");
        assert_eq!(json["order"]["units"], "10000");
        assert_eq!(json["order"]["type"], "MARKET");
        assert_eq!(json["order"]["timeInForce"], "FOK");
        assert_eq!(json["order"]["positionFill"], "DEFAULT");
    }

    #[test]
    fn test_order_request_serialization_sell() {
        let req = OandaOrderRequest {
            order: MarketOrderBody {
                instrument: "GBP_USD".to_string(),
                units: "-5000".to_string(),
                order_type: "MARKET".to_string(),
                time_in_force: "FOK".to_string(),
                position_fill: "DEFAULT".to_string(),
            },
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["order"]["instrument"], "GBP_USD");
        assert_eq!(json["order"]["units"], "-5000");
        assert_eq!(json["order"]["type"], "MARKET");
    }

    #[test]
    fn test_order_request_roundtrip() {
        let req = OandaOrderRequest {
            order: MarketOrderBody {
                instrument: "USD_JPY".to_string(),
                units: "25000".to_string(),
                order_type: "MARKET".to_string(),
                time_in_force: "FOK".to_string(),
                position_fill: "DEFAULT".to_string(),
            },
        };

        let serialized = serde_json::to_string(&req).unwrap();
        let deserialized: OandaOrderRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.order.instrument, "USD_JPY");
        assert_eq!(deserialized.order.units, "25000");
        assert_eq!(deserialized.order.order_type, "MARKET");
    }

    #[test]
    fn test_account_info_deserialization() {
        let json = r#"{
            "account": {
                "id": "101-001-12345-001",
                "balance": "10250.45",
                "pl": "-125.30",
                "unrealizedPL": "340.20",
                "marginUsed": "2100.00",
                "marginAvailable": "8150.45",
                "NAV": "10590.65",
                "currency": "USD",
                "openTradeCount": 3,
                "openPositionCount": 2
            }
        }"#;
        let resp: AccountInfoResponse = serde_json::from_str(json).unwrap();
        let acct = resp.account;
        assert_eq!(acct.id, "101-001-12345-001");
        assert_eq!(acct.balance, dec!(10250.45));
        assert_eq!(acct.pl, dec!(-125.30));
        assert_eq!(acct.unrealized_pl, dec!(340.20));
        assert_eq!(acct.margin_used, dec!(2100.00));
        assert_eq!(acct.margin_available, dec!(8150.45));
        assert_eq!(acct.nav, dec!(10590.65));
        assert_eq!(acct.currency, "USD");
        assert_eq!(acct.open_trade_count, 3);
        assert_eq!(acct.open_position_count, 2);
    }

    #[test]
    fn test_position_deserialization() {
        let json = r#"{
            "positions": [
                {
                    "instrument": "EUR_USD",
                    "pl": "-45.20",
                    "unrealizedPL": "120.50",
                    "long": {
                        "units": "10000",
                        "averagePrice": "1.08500",
                        "unrealizedPL": "120.50",
                        "pl": "-45.20"
                    },
                    "short": {
                        "units": "0",
                        "unrealizedPL": "0.0000",
                        "pl": "0.0000"
                    }
                }
            ]
        }"#;
        let resp: PositionsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.positions.len(), 1);
        let pos = &resp.positions[0];
        assert_eq!(pos.instrument, "EUR_USD");
        assert_eq!(pos.unrealized_pl, dec!(120.50));

        let long = pos.long_side.as_ref().unwrap();
        assert_eq!(long.units, "10000");
        assert_eq!(long.average_price, Some(dec!(1.08500)));
    }

    #[test]
    fn test_pricing_deserialization() {
        let json = r#"{
            "prices": [
                {
                    "instrument": "EUR_USD",
                    "closeoutAsk": "1.08750",
                    "closeoutBid": "1.08730",
                    "status": "tradeable",
                    "tradeable": true
                },
                {
                    "instrument": "GBP_USD",
                    "closeoutAsk": "1.27100",
                    "closeoutBid": "1.27080",
                    "status": "tradeable",
                    "tradeable": true
                }
            ]
        }"#;
        let resp: PricingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.prices.len(), 2);

        assert_eq!(resp.prices[0].instrument, "EUR_USD");
        assert_eq!(resp.prices[0].closeout_ask, dec!(1.08750));
        assert_eq!(resp.prices[0].closeout_bid, dec!(1.08730));
        assert_eq!(resp.prices[0].status.as_deref(), Some("tradeable"));
        assert_eq!(resp.prices[0].tradeable, Some(true));

        assert_eq!(resp.prices[1].instrument, "GBP_USD");
        assert_eq!(resp.prices[1].closeout_ask, dec!(1.27100));
    }

    #[test]
    fn test_order_response_deserialization() {
        let json = r#"{
            "orderCreateTransaction": {
                "id": "6356",
                "type": "MARKET_ORDER",
                "instrument": "EUR_USD",
                "units": "10000",
                "timeInForce": "FOK",
                "reason": "CLIENT_ORDER"
            },
            "orderFillTransaction": {
                "id": "6357",
                "price": "1.08750",
                "units": "10000",
                "pl": "0.0000",
                "instrument": "EUR_USD",
                "accountBalance": "10250.45"
            },
            "relatedTransactionIDs": ["6356", "6357"]
        }"#;
        let resp: OandaOrderResponse = serde_json::from_str(json).unwrap();

        let create = resp.order_create_transaction.unwrap();
        assert_eq!(create.id.as_deref(), Some("6356"));
        assert_eq!(create.transaction_type.as_deref(), Some("MARKET_ORDER"));
        assert_eq!(create.instrument.as_deref(), Some("EUR_USD"));
        assert_eq!(create.units.as_deref(), Some("10000"));

        let fill = resp.order_fill_transaction.unwrap();
        assert_eq!(fill.id.as_deref(), Some("6357"));
        assert_eq!(fill.price, Some(dec!(1.08750)));
        assert_eq!(fill.units.as_deref(), Some("10000"));
        assert_eq!(fill.account_balance, Some(dec!(10250.45)));

        assert_eq!(resp.related_transaction_ids.len(), 2);
    }

    #[test]
    fn test_close_response_deserialization() {
        let json = r#"{
            "longOrderFillTransaction": {
                "id": "6358",
                "price": "1.08760",
                "units": "-10000",
                "pl": "15.50",
                "instrument": "EUR_USD",
                "accountBalance": "10265.95"
            },
            "relatedTransactionIDs": ["6358"]
        }"#;
        let resp: OandaCloseResponse = serde_json::from_str(json).unwrap();

        let fill = resp.long_order_fill_transaction.unwrap();
        assert_eq!(fill.id.as_deref(), Some("6358"));
        assert_eq!(fill.price, Some(dec!(1.08760)));
        assert_eq!(fill.pl, Some(dec!(15.50)));
        assert_eq!(fill.account_balance, Some(dec!(10265.95)));

        assert!(resp.short_order_fill_transaction.is_none());
        assert_eq!(resp.related_transaction_ids.len(), 1);
    }

    #[test]
    fn test_close_position_body_serialization() {
        let body = ClosePositionBody {
            long_units: "ALL".to_string(),
            short_units: "ALL".to_string(),
        };
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["longUnits"], "ALL");
        assert_eq!(json["shortUnits"], "ALL");
    }

    #[test]
    fn test_major_pairs_count() {
        assert_eq!(MAJOR_PAIRS.len(), 7);
        assert!(MAJOR_PAIRS.contains(&"EUR_USD"));
        assert!(MAJOR_PAIRS.contains(&"GBP_USD"));
    }

    #[test]
    fn test_all_pairs_contains_majors() {
        for pair in MAJOR_PAIRS {
            assert!(
                ALL_PAIRS.contains(pair),
                "ALL_PAIRS should contain major pair {}",
                pair
            );
        }
    }
}
