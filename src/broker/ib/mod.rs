//! Interactive Brokers Client Portal API Integration
//!
//! Implements REST API client for IB Client Portal
//! Documentation: https://interactivebrokers.github.io/cpwebapi/

use async_trait::async_trait;
use reqwest::{Client, ClientBuilder, StatusCode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::broker::{
    AccountInfo, Broker, BrokerConfig, BrokerError, Execution, Order, OrderStatus, Position, Result,
};

// Re-export client
mod client;
mod models;

pub use client::IbClient;
pub use models::*;

/// Interactive Brokers Client Portal implementation
pub struct InteractiveBrokers {
    config: BrokerConfig,
    http_client: Client,
    auth_status: Arc<RwLock<AuthStatus>>,
    account_id: Arc<RwLock<Option<String>>>,
    is_paper: bool,
    /// Cached connection state (updated on authenticate/disconnect)
    connected: Arc<std::sync::atomic::AtomicBool>,
    /// Ticker → conid resolution cache
    conid_cache: Arc<RwLock<std::collections::HashMap<String, i64>>>,
}

#[derive(Debug, Clone)]
enum AuthStatus {
    Disconnected,
    Authenticating,
    Authenticated,
    Failed(String),
}

impl InteractiveBrokers {
    /// Create a new IB client
    pub fn new(config: BrokerConfig) -> Self {
        let is_paper = config.paper_trading;

        // Create HTTP client with appropriate timeouts
        let http_client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true) // IB uses self-signed certs
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            auth_status: Arc::new(RwLock::new(AuthStatus::Disconnected)),
            account_id: Arc::new(RwLock::new(None)),
            is_paper,
            connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            conid_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Check if using paper trading
    pub fn is_paper(&self) -> bool {
        self.is_paper
    }

    /// Get base API URL
    fn base_url(&self) -> &str {
        &self.config.api_url
    }

    /// Make authenticated request
    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url(), path);

        let mut request = self.http_client.request(method, &url);

        // Add auth token if available
        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Add body for POST/PUT
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| BrokerError::ExternalApi(format!("Request failed: {}", e)))?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => response
                .json::<T>()
                .await
                .map_err(|e| BrokerError::ExternalApi(format!("Failed to parse response: {}", e))),
            StatusCode::UNAUTHORIZED => Err(BrokerError::Authentication(
                "Invalid credentials".to_string(),
            )),
            StatusCode::TOO_MANY_REQUESTS => Err(BrokerError::RateLimit),
            status => {
                let text = response.text().await.unwrap_or_default();
                Err(BrokerError::ExternalApi(format!(
                    "HTTP {}: {}",
                    status, text
                )))
            }
        }
    }

    /// Authenticate with IB
    async fn authenticate(&self) -> Result<()> {
        info!("Authenticating with Interactive Brokers...");

        let mut status = self.auth_status.write().await;
        *status = AuthStatus::Authenticating;

        // For IB Client Portal, authentication is done via the web UI
        // We check if already authenticated by calling /iserver/auth/status
        match self.check_auth_status().await {
            Ok(true) => {
                info!("Already authenticated with IB");
                *status = AuthStatus::Authenticated;
                self.connected
                    .store(true, std::sync::atomic::Ordering::Relaxed);

                // Get account ID
                if let Ok(accounts) = self.get_accounts().await {
                    if let Some(account) = accounts.first() {
                        let mut account_id = self.account_id.write().await;
                        *account_id = Some(account.account_id.clone());
                    }
                }

                Ok(())
            }
            Ok(false) => {
                warn!("Not authenticated with IB. Please authenticate via Client Portal.");
                *status = AuthStatus::Failed("Not authenticated".to_string());
                self.connected
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                Err(BrokerError::Authentication(
                    "Please authenticate via IB Client Portal".to_string(),
                ))
            }
            Err(e) => {
                error!("Authentication check failed: {}", e);
                *status = AuthStatus::Failed(e.to_string());
                self.connected
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                Err(e)
            }
        }
    }

    /// Check authentication status
    async fn check_auth_status(&self) -> Result<bool> {
        // Call a simple endpoint that requires auth
        match self
            .request::<serde_json::Value>(reqwest::Method::GET, "/iserver/auth/status", None)
            .await
        {
            Ok(_) => Ok(true),
            Err(BrokerError::Authentication(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get accounts
    async fn get_accounts(&self) -> Result<Vec<IbAccount>> {
        self.request::<Vec<IbAccount>>(reqwest::Method::GET, "/portfolio/accounts", None)
            .await
    }

    /// Resolve ticker symbol to IB contract ID (conid).
    /// Uses cache first, then queries IB API `/iserver/secdef/search`.
    async fn resolve_conid(&self, ticker: &str) -> Result<i64> {
        // Check cache first
        {
            let cache = self.conid_cache.read().await;
            if let Some(&conid) = cache.get(ticker) {
                return Ok(conid);
            }
        }

        // Query IB API
        let body = serde_json::json!({ "symbol": ticker, "secType": "STK", "name": true });
        let results: Vec<IbSecDefResult> = self
            .request(reqwest::Method::POST, "/iserver/secdef/search", Some(body))
            .await?;

        let conid = results
            .first()
            .ok_or_else(|| {
                BrokerError::InvalidOrder(format!("No contract found for ticker '{}'", ticker))
            })?
            .conid;

        // Cache for future lookups
        {
            let mut cache = self.conid_cache.write().await;
            cache.insert(ticker.to_string(), conid);
        }

        debug!("Resolved ticker {} → conid {}", ticker, conid);
        Ok(conid)
    }

    /// Place order with IB
    async fn place_ib_order(&self, order: &Order) -> Result<IbOrderResponse> {
        let account_id = self
            .account_id
            .read()
            .await
            .clone()
            .ok_or_else(|| BrokerError::Authentication("No account ID".to_string()))?;

        let conid = self.resolve_conid(&order.ticker).await?;

        let ib_order = IbOrderRequest {
            acct_id: account_id,
            conid,
            sec_type: "STK".to_string(),
            c_oid: Some(order.id.to_string()),
            parent_id: None,
            order_type: order.order_type.as_str().to_string(),
            listing_exchange: None,
            is_single_group: true,
            outside_rth: false,
            price: order.limit_price.map(|p| p.to_string()),
            aux_price: order.stop_price.map(|p| p.to_string()),
            side: order.side.as_str().to_string(),
            ticker: order.ticker.clone(),
            tif: format!("{:?}", order.time_in_force),
            quantity: order.quantity.to_string(),
            use_adaptive: true,
        };

        self.request::<IbOrderResponse>(
            reqwest::Method::POST,
            "/iserver/account/order",
            Some(
                serde_json::to_value(ib_order)
                    .map_err(|e| BrokerError::InvalidOrder(e.to_string()))?,
            ),
        )
        .await
    }

    /// Cancel order with IB
    async fn cancel_ib_order(&self, broker_order_id: &str) -> Result<()> {
        let account_id = self
            .account_id
            .read()
            .await
            .clone()
            .ok_or_else(|| BrokerError::Authentication("No account ID".to_string()))?;

        let path = format!("/iserver/account/{}/order/{}", account_id, broker_order_id);

        self.request::<serde_json::Value>(reqwest::Method::DELETE, &path, None)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Broker for InteractiveBrokers {
    async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Interactive Brokers...");

        // Test connection by checking auth status
        self.authenticate().await?;

        info!("Connected to Interactive Brokers");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        info!("Disconnecting from Interactive Brokers...");

        let mut status = self.auth_status.write().await;
        *status = AuthStatus::Disconnected;
        self.connected
            .store(false, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn get_account_info(&self) -> Result<AccountInfo> {
        let account_id = self
            .account_id
            .read()
            .await
            .clone()
            .ok_or_else(|| BrokerError::Authentication("No account ID".to_string()))?;

        let summary: IbAccountSummary = self
            .request(
                reqwest::Method::GET,
                &format!("/portfolio/{}/summary", account_id),
                None,
            )
            .await?;

        Ok(AccountInfo {
            account_id: summary.account_id,
            cash_balance: summary.cash_balance,
            buying_power: summary.buying_power,
            equity_with_loan: summary.equity_with_loan_value,
            net_liquidation: summary.net_liquidation_value,
            unrealized_pnl: summary.unrealized_pnl,
            realized_pnl: summary.realized_pnl,
            currency: summary.currency,
            updated_at: chrono::Utc::now(),
        })
    }

    async fn get_positions(&self) -> Result<Vec<Position>> {
        let account_id = self
            .account_id
            .read()
            .await
            .clone()
            .ok_or_else(|| BrokerError::Authentication("No account ID".to_string()))?;

        let ib_positions: Vec<IbPosition> = self
            .request(
                reqwest::Method::GET,
                &format!("/portfolio/{}/positions", account_id),
                None,
            )
            .await?;

        let positions: Vec<Position> = ib_positions
            .into_iter()
            .map(|p| Position {
                id: uuid::Uuid::new_v4(),
                ticker: p.ticker,
                quantity: p.position,
                avg_cost: p.avg_cost,
                market_price: p.market_price,
                market_value: p.market_value,
                unrealized_pnl: p.unrealized_pnl,
                realized_pnl: Decimal::ZERO,
                portfolio_id: uuid::Uuid::parse_str(&account_id)
                    .unwrap_or_else(|_| uuid::Uuid::new_v4()),
                opened_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .collect();

        Ok(positions)
    }

    async fn get_position(&self, ticker: &str) -> Result<Option<Position>> {
        let positions = self.get_positions().await?;
        Ok(positions.into_iter().find(|p| p.ticker == ticker))
    }

    async fn place_order(&self, order: &mut Order) -> Result<()> {
        debug!("Placing order: {:?}", order);

        let response = self.place_ib_order(order).await?;

        order.broker_order_id = Some(response.order_id.clone());
        order.status = OrderStatus::PreSubmitted;
        order.updated_at = chrono::Utc::now();

        info!("Order placed: {} -> {}", order.id, response.order_id);
        Ok(())
    }

    async fn cancel_order(&self, order: &mut Order) -> Result<()> {
        let broker_order_id = order
            .broker_order_id
            .as_ref()
            .ok_or_else(|| BrokerError::InvalidOrder("No broker order ID".to_string()))?;

        self.cancel_ib_order(broker_order_id).await?;

        order.status = OrderStatus::PendingCancel;
        order.updated_at = chrono::Utc::now();

        info!("Order cancelled: {}", order.id);
        Ok(())
    }

    async fn modify_order(
        &self,
        _order: &mut Order,
        _new_quantity: Option<Decimal>,
        _new_price: Option<Decimal>,
    ) -> Result<()> {
        // IB requires cancel + replace for modifications
        Err(BrokerError::InvalidOrder(
            "Use cancel + new order for modifications".to_string(),
        ))
    }

    async fn get_order_status(&self, order: &mut Order) -> Result<OrderStatus> {
        let broker_order_id = order
            .broker_order_id
            .as_ref()
            .ok_or_else(|| BrokerError::InvalidOrder("No broker order ID".to_string()))?;

        let account_id = self
            .account_id
            .read()
            .await
            .clone()
            .ok_or_else(|| BrokerError::Authentication("No account ID".to_string()))?;

        let orders: Vec<IbOrderStatus> = self
            .request(
                reqwest::Method::GET,
                &format!("/iserver/account/{}/orders", account_id),
                None,
            )
            .await?;

        let status = orders
            .into_iter()
            .find(|o| o.order_id == *broker_order_id)
            .map(|o| o.status.into())
            .unwrap_or(order.status);

        order.status = status;
        Ok(status)
    }

    async fn get_executions(&self, order_id: uuid::Uuid) -> Result<Vec<Execution>> {
        let account_id = self
            .account_id
            .read()
            .await
            .clone()
            .ok_or_else(|| BrokerError::Authentication("No account ID".to_string()))?;

        let ib_executions: Vec<IbExecution> = self
            .request(
                reqwest::Method::GET,
                &format!("/iserver/account/{}/trades", account_id),
                None,
            )
            .await?;

        // Filter executions matching this order and convert
        let order_id_str = order_id.to_string();
        let executions = ib_executions
            .into_iter()
            .filter(|e| {
                e.order_ref
                    .as_deref()
                    .map(|r| r == order_id_str)
                    .unwrap_or(false)
            })
            .map(|e| Execution {
                id: uuid::Uuid::new_v4(),
                order_id,
                broker_execution_id: e._execution_id.clone().unwrap_or_default(),
                ticker: String::new(),               // filled by caller context
                side: crate::broker::OrderSide::Buy, // determined by execution context
                quantity: e.size,
                price: e.price,
                commission: e.commission.unwrap_or(Decimal::ZERO),
                timestamp: e
                    .trade_time
                    .as_deref()
                    .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now),
            })
            .collect();

        Ok(executions)
    }

    async fn get_market_price(&self, ticker: &str) -> Result<Decimal> {
        // Get snapshot for ticker
        let snapshot: serde_json::Value = self
            .request(
                reqwest::Method::GET,
                "/iserver/marketdata/snapshot?conids=0&fields=31",
                None,
            )
            .await?;

        // Extract price from snapshot
        snapshot
            .get("31")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str_exact(s).ok())
            .ok_or_else(|| BrokerError::ExternalApi(format!("Could not get price for {}", ticker)))
    }
}

// IB-specific models
#[derive(Debug, Clone, Deserialize)]
struct IbAccount {
    #[serde(rename = "accountId")]
    account_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct IbAccountSummary {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "cashbalance")]
    cash_balance: Decimal,
    #[serde(rename = "buyingpower")]
    buying_power: Decimal,
    #[serde(rename = "equitywithloanvalue")]
    equity_with_loan_value: Decimal,
    #[serde(rename = "netliquidationvalue")]
    net_liquidation_value: Decimal,
    #[serde(rename = "unrealizedpnl")]
    unrealized_pnl: Decimal,
    #[serde(rename = "realizedpnl")]
    realized_pnl: Decimal,
    currency: String,
}

#[derive(Debug, Clone, Deserialize)]
struct IbPosition {
    #[serde(rename = "contractDesc")]
    ticker: String,

    position: Decimal,
    #[serde(rename = "avgCost")]
    avg_cost: Decimal,
    #[serde(rename = "mktPrice")]
    market_price: Option<Decimal>,
    #[serde(rename = "mktValue")]
    market_value: Option<Decimal>,
    #[serde(rename = "unrealizedPnl")]
    unrealized_pnl: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize)]
struct IbOrderRequest {
    #[serde(rename = "acctId")]
    acct_id: String,
    conid: i64,
    #[serde(rename = "secType")]
    sec_type: String,
    #[serde(rename = "cOID")]
    c_oid: Option<String>,
    #[serde(rename = "parentId")]
    parent_id: Option<String>,
    #[serde(rename = "orderType")]
    order_type: String,
    #[serde(rename = "listingExchange")]
    listing_exchange: Option<String>,
    #[serde(rename = "isSingleGroup")]
    is_single_group: bool,
    #[serde(rename = "outsideRTH")]
    outside_rth: bool,
    price: Option<String>,
    #[serde(rename = "auxPrice")]
    aux_price: Option<String>,
    side: String,
    ticker: String,
    #[serde(rename = "tif")]
    tif: String,
    quantity: String,
    #[serde(rename = "useAdaptive")]
    use_adaptive: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct IbOrderResponse {
    #[serde(rename = "order_id")]
    order_id: String,
    #[serde(rename = "local_order_id")]
    local_order_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct IbOrderStatus {
    #[serde(rename = "orderId")]
    order_id: String,
    status: String,
}

#[derive(Debug, Clone, Deserialize)]
struct IbSecDefResult {
    conid: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct IbExecution {
    #[serde(rename = "execution_id")]
    _execution_id: Option<String>,
    price: Decimal,
    size: Decimal,
    commission: Option<Decimal>,
    #[serde(rename = "trade_time")]
    trade_time: Option<String>,
    /// Client order ref (maps to our order UUID)
    #[serde(rename = "order_ref")]
    order_ref: Option<String>,
}

impl From<String> for OrderStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "PendingSubmit" => OrderStatus::PendingSubmit,
            "PreSubmitted" => OrderStatus::PreSubmitted,
            "Submitted" => OrderStatus::Submitted,
            "Filled" => OrderStatus::Filled,
            "PartiallyFilled" => OrderStatus::PartiallyFilled,
            "Cancelled" => OrderStatus::Cancelled,
            "PendingCancel" => OrderStatus::PendingCancel,
            "ApiCancelled" => OrderStatus::ApiCancelled,
            "Inactive" => OrderStatus::Inactive,
            _ => OrderStatus::ApiPending,
        }
    }
}
