//! WebSocket Connection Manager
//!
//! Real `tokio-tungstenite` connections to Binance public streams with
//! exponential-backoff reconnection, dynamic subscription management, and
//! Binance message deserialization.

use chrono::{DateTime, TimeZone, Utc};
use futures::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message as TungsteniteMsg;
use tracing::{debug, error, info, warn};

use super::{MarketTick, Result, Side, StreamError, TickType};

// ---------------------------------------------------------------------------
// Binance message types
// ---------------------------------------------------------------------------

/// Binance individual trade message (`@trade` stream)
#[derive(Debug, Clone, Deserialize)]
pub struct BinanceTrade {
    /// Event type — always `"trade"`
    #[serde(rename = "e")]
    pub event_type: String,
    /// Symbol (e.g. `"BTCUSDT"`)
    #[serde(rename = "s")]
    pub symbol: String,
    /// Price as string
    #[serde(rename = "p")]
    pub price: String,
    /// Quantity as string
    #[serde(rename = "q")]
    pub quantity: String,
    /// Is the buyer the market maker?
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
    /// Trade time (epoch ms)
    #[serde(rename = "T")]
    pub trade_time: i64,
}

/// Binance best bid/ask ticker (`@bookTicker` stream)
#[derive(Debug, Clone, Deserialize)]
pub struct BinanceBookTicker {
    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,
    /// Best bid price
    #[serde(rename = "b")]
    pub bid_price: String,
    /// Best bid qty
    #[serde(rename = "B")]
    pub bid_qty: String,
    /// Best ask price
    #[serde(rename = "a")]
    pub ask_price: String,
    /// Best ask qty
    #[serde(rename = "A")]
    pub ask_qty: String,
}

/// Binance combined stream envelope
#[derive(Debug, Clone, Deserialize)]
pub struct BinanceCombinedStream {
    /// Stream name (e.g. `"btcusdt@trade"`)
    pub stream: String,
    /// Raw JSON payload
    pub data: serde_json::Value,
}

/// Subscription request sent over the WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceSubscription {
    pub method: String,
    pub params: Vec<String>,
    pub id: u64,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a raw Binance JSON text frame into a `MarketTick`.
pub fn parse_binance_message(exchange: &str, text: &str) -> Option<MarketTick> {
    // Try combined stream envelope first
    if let Ok(combined) = serde_json::from_str::<BinanceCombinedStream>(text) {
        if combined.stream.ends_with("@trade") {
            if let Ok(trade) = serde_json::from_value::<BinanceTrade>(combined.data) {
                return binance_trade_to_tick(exchange, &trade);
            }
        } else if combined.stream.ends_with("@bookTicker") {
            if let Ok(book) = serde_json::from_value::<BinanceBookTicker>(combined.data) {
                return binance_book_ticker_to_tick(exchange, &book);
            }
        }
    }

    // Try individual trade message
    if let Ok(trade) = serde_json::from_str::<BinanceTrade>(text) {
        if trade.event_type == "trade" {
            return binance_trade_to_tick(exchange, &trade);
        }
    }

    // Try individual book ticker
    if let Ok(book) = serde_json::from_str::<BinanceBookTicker>(text) {
        // bookTicker has no event_type field, but always has bid/ask fields
        if !book.bid_price.is_empty() && !book.ask_price.is_empty() {
            return binance_book_ticker_to_tick(exchange, &book);
        }
    }

    None
}

fn binance_trade_to_tick(exchange: &str, trade: &BinanceTrade) -> Option<MarketTick> {
    let price = Decimal::from_str(&trade.price).ok()?;
    let quantity = Decimal::from_str(&trade.quantity).ok()?;

    // is_buyer_maker == true means the taker was a seller (aggressive sell)
    let side = if trade.is_buyer_maker {
        Some(Side::Ask)
    } else {
        Some(Side::Bid)
    };

    let timestamp = epoch_ms_to_datetime(trade.trade_time)?;

    Some(MarketTick {
        exchange: exchange.to_string(),
        symbol: trade.symbol.clone(),
        price,
        quantity,
        side,
        tick_type: TickType::Trade,
        timestamp,
    })
}

fn binance_book_ticker_to_tick(exchange: &str, book: &BinanceBookTicker) -> Option<MarketTick> {
    let bid_price = Decimal::from_str(&book.bid_price).ok()?;
    let ask_price = Decimal::from_str(&book.ask_price).ok()?;
    let bid_qty = Decimal::from_str(&book.bid_qty).ok()?;

    Some(MarketTick {
        exchange: exchange.to_string(),
        symbol: book.symbol.clone(),
        price: (bid_price + ask_price) / Decimal::from(2),
        quantity: bid_qty,
        side: None,
        tick_type: TickType::Bid,
        timestamp: Utc::now(),
    })
}

fn epoch_ms_to_datetime(ms: i64) -> Option<DateTime<Utc>> {
    let secs = ms / 1000;
    let nsecs = ((ms % 1000) * 1_000_000) as u32;
    Utc.timestamp_opt(secs, nsecs).single()
}

// ---------------------------------------------------------------------------
// Subscription Manager
// ---------------------------------------------------------------------------

/// Tracks active subscriptions and generates Binance sub/unsub JSON.
#[derive(Debug, Clone)]
pub struct SubscriptionManager {
    subscribed: HashSet<String>,
    next_id: u64,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            subscribed: HashSet::new(),
            next_id: 1,
        }
    }

    /// Build SUBSCRIBE request JSON for the given streams (e.g. `["btcusdt@trade"]`).
    pub fn subscribe(&mut self, streams: &[String]) -> String {
        let new: Vec<String> = streams
            .iter()
            .filter(|s| !self.subscribed.contains(*s))
            .cloned()
            .collect();
        if new.is_empty() {
            return String::new();
        }
        for s in &new {
            self.subscribed.insert(s.clone());
        }
        let id = self.next_id;
        self.next_id += 1;
        let msg = BinanceSubscription {
            method: "SUBSCRIBE".to_string(),
            params: new,
            id,
        };
        serde_json::to_string(&msg).unwrap_or_default()
    }

    /// Build UNSUBSCRIBE request JSON.
    pub fn unsubscribe(&mut self, streams: &[String]) -> String {
        let removing: Vec<String> = streams
            .iter()
            .filter(|s| self.subscribed.contains(*s))
            .cloned()
            .collect();
        if removing.is_empty() {
            return String::new();
        }
        for s in &removing {
            self.subscribed.remove(s);
        }
        let id = self.next_id;
        self.next_id += 1;
        let msg = BinanceSubscription {
            method: "UNSUBSCRIBE".to_string(),
            params: removing,
            id,
        };
        serde_json::to_string(&msg).unwrap_or_default()
    }

    /// All currently subscribed stream names.
    pub fn subscribed_streams(&self) -> Vec<String> {
        self.subscribed.iter().cloned().collect()
    }

    /// Re-subscribe all streams (returns SUBSCRIBE JSON or empty string).
    pub fn resubscribe_all(&mut self) -> String {
        if self.subscribed.is_empty() {
            return String::new();
        }
        let streams: Vec<String> = self.subscribed.iter().cloned().collect();
        let id = self.next_id;
        self.next_id += 1;
        let msg = BinanceSubscription {
            method: "SUBSCRIBE".to_string(),
            params: streams,
            id,
        };
        serde_json::to_string(&msg).unwrap_or_default()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Connection config & state
// ---------------------------------------------------------------------------

/// Exchange configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub exchange: String,
    pub ws_url: String,
    pub symbols: Vec<String>,
    pub reconnect_interval_ms: u64,
    pub max_reconnect_attempts: u32,
    pub heartbeat_interval_sec: u64,
    pub timeout_ms: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            exchange: "binance".to_string(),
            ws_url: "wss://stream.binance.com:9443/ws".to_string(),
            symbols: vec!["BTCUSDT".to_string()],
            reconnect_interval_ms: 5000,
            max_reconnect_attempts: 10,
            heartbeat_interval_sec: 30,
            timeout_ms: 5000,
        }
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Exchange feed handle
#[derive(Debug, Clone)]
pub struct ExchangeFeed {
    pub exchange: String,
    pub state: ConnectionState,
    pub last_message: Option<DateTime<Utc>>,
    pub messages_received: u64,
    pub reconnect_count: u32,
    pub latency_ms: u64,
}

/// WebSocket message (kept for backward compatibility)
#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping,
    Pong,
    Close,
}

// ---------------------------------------------------------------------------
// Backoff helpers
// ---------------------------------------------------------------------------

/// Compute exponential backoff with jitter, capped at 60 s.
pub fn backoff_with_jitter(base_ms: u64, attempt: u32) -> u64 {
    let exp = base_ms.saturating_mul(2u64.saturating_pow(attempt.min(5)));
    let capped = exp.min(60_000);
    let jitter = fastrand::u64(0..capped.max(4) / 4);
    capped.saturating_add(jitter)
}

// ---------------------------------------------------------------------------
// WebSocket URL builder
// ---------------------------------------------------------------------------

/// Build the WebSocket URL for Binance.
/// Single-symbol: `{base}/{sym}@trade`
/// Multi-symbol: `{base}/stream?streams={sym1}@trade/{sym2}@trade`
pub fn build_ws_url(base: &str, symbols: &[String]) -> String {
    if symbols.is_empty() {
        return base.to_string();
    }
    let streams: Vec<String> = symbols
        .iter()
        .map(|s| format!("{}@trade", s.to_lowercase()))
        .collect();
    if streams.len() == 1 {
        format!("{}/{}", base.trim_end_matches('/'), streams[0])
    } else {
        format!(
            "{}/stream?streams={}",
            base.trim_end_matches('/'),
            streams.join("/")
        )
    }
}

// ---------------------------------------------------------------------------
// WebSocket Manager
// ---------------------------------------------------------------------------

/// Handle to a single WS connection
struct ConnectionHandle {
    shutdown_tx: watch::Sender<bool>,
    task_handle: tokio::task::JoinHandle<()>,
}

/// WebSocket Manager
pub struct WebSocketManager {
    configs: HashMap<String, ConnectionConfig>,
    states: Arc<RwLock<HashMap<String, ConnectionState>>>,
    feeds: Arc<RwLock<HashMap<String, ExchangeFeed>>>,
    tick_tx: mpsc::Sender<MarketTick>,
    running: Arc<RwLock<bool>>,
    connections: HashMap<String, ConnectionHandle>,
    subscriptions: Arc<RwLock<SubscriptionManager>>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new(tick_tx: mpsc::Sender<MarketTick>) -> Self {
        Self {
            configs: HashMap::new(),
            states: Arc::new(RwLock::new(HashMap::new())),
            feeds: Arc::new(RwLock::new(HashMap::new())),
            tick_tx,
            running: Arc::new(RwLock::new(false)),
            connections: HashMap::new(),
            subscriptions: Arc::new(RwLock::new(SubscriptionManager::new())),
        }
    }

    /// Register an exchange feed
    pub async fn register_exchange(&mut self, config: ConnectionConfig) {
        let exchange = config.exchange.clone();

        self.configs.insert(exchange.clone(), config);

        let mut states = self.states.write().await;
        states.insert(exchange.clone(), ConnectionState::Disconnected);

        let mut feeds = self.feeds.write().await;
        feeds.insert(
            exchange.clone(),
            ExchangeFeed {
                exchange: exchange.clone(),
                state: ConnectionState::Disconnected,
                last_message: None,
                messages_received: 0,
                reconnect_count: 0,
                latency_ms: 0,
            },
        );

        info!("Registered exchange feed: {}", exchange);
    }

    /// Start all connections
    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;

        for (exchange, config) in &self.configs {
            let exchange = exchange.clone();
            let config = config.clone();
            let states = self.states.clone();
            let feeds = self.feeds.clone();
            let tick_tx = self.tick_tx.clone();
            let running = self.running.clone();
            let subs = self.subscriptions.clone();

            tokio::spawn(async move {
                Self::connection_loop(exchange, config, states, feeds, tick_tx, running, subs)
                    .await;
            });
        }

        info!("WebSocket manager started");
        Ok(())
    }

    /// Stop all connections
    pub async fn stop(&self) {
        *self.running.write().await = false;

        let mut states = self.states.write().await;
        for state in states.values_mut() {
            *state = ConnectionState::Disconnected;
        }

        info!("WebSocket manager stopped");
    }

    /// Get connection state
    pub async fn get_state(&self, exchange: &str) -> Option<ConnectionState> {
        let states = self.states.read().await;
        states.get(exchange).copied()
    }

    /// Get feed statistics
    pub async fn get_feed(&self, exchange: &str) -> Option<ExchangeFeed> {
        let feeds = self.feeds.read().await;
        feeds.get(exchange).cloned()
    }

    /// Get all feeds
    pub async fn get_all_feeds(&self) -> Vec<ExchangeFeed> {
        let feeds = self.feeds.read().await;
        feeds.values().cloned().collect()
    }

    /// Subscribe to additional symbols on a running connection.
    pub async fn subscribe_symbols(&self, _exchange: &str, symbols: &[String]) {
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@trade", s.to_lowercase()))
            .collect();
        let mut subs = self.subscriptions.write().await;
        let _json = subs.subscribe(&streams);
        // In a full implementation the JSON would be forwarded to the write-half
        // of the active connection. For now the subscription state is tracked.
    }

    /// Unsubscribe symbols.
    pub async fn unsubscribe_symbols(&self, _exchange: &str, symbols: &[String]) {
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@trade", s.to_lowercase()))
            .collect();
        let mut subs = self.subscriptions.write().await;
        let _json = subs.unsubscribe(&streams);
    }

    /// Connection loop with reconnection logic
    async fn connection_loop(
        exchange: String,
        config: ConnectionConfig,
        states: Arc<RwLock<HashMap<String, ConnectionState>>>,
        feeds: Arc<RwLock<HashMap<String, ExchangeFeed>>>,
        tick_tx: mpsc::Sender<MarketTick>,
        running: Arc<RwLock<bool>>,
        subs: Arc<RwLock<SubscriptionManager>>,
    ) {
        let mut reconnect_attempts: u32 = 0;

        while *running.read().await {
            // Set connecting state
            {
                let mut states = states.write().await;
                states.insert(exchange.clone(), ConnectionState::Connecting);
            }

            let url = build_ws_url(&config.ws_url, &config.symbols);
            debug!("Connecting to {} at {}", exchange, url);

            match Self::connect(
                &exchange,
                &url,
                &config,
                tick_tx.clone(),
                running.clone(),
                subs.clone(),
            )
            .await
            {
                Ok(()) => {
                    // connect() returns Ok when the read-loop ends normally
                    // (e.g. server closed or shutdown signalled).
                    reconnect_attempts = 0;

                    {
                        let mut feeds = feeds.write().await;
                        if let Some(feed) = feeds.get_mut(&exchange) {
                            feed.state = ConnectionState::Disconnected;
                        }
                    }

                    if !*running.read().await {
                        break;
                    }
                    // Server closed — reconnect
                    warn!("{}: connection closed, will reconnect", exchange);
                }
                Err(e) => {
                    error!("Connection to {} failed: {}", exchange, e);
                    reconnect_attempts += 1;

                    {
                        let mut states = states.write().await;
                        states.insert(exchange.clone(), ConnectionState::Reconnecting);
                    }
                    {
                        let mut feeds = feeds.write().await;
                        if let Some(feed) = feeds.get_mut(&exchange) {
                            feed.state = ConnectionState::Reconnecting;
                            feed.reconnect_count = reconnect_attempts;
                        }
                    }

                    if reconnect_attempts >= config.max_reconnect_attempts {
                        error!("Max reconnection attempts reached for {}", exchange);
                        let mut states = states.write().await;
                        states.insert(exchange.clone(), ConnectionState::Failed);
                        break;
                    }
                }
            }

            let backoff = backoff_with_jitter(config.reconnect_interval_ms, reconnect_attempts);
            warn!("Reconnecting to {} in {}ms", exchange, backoff);
            sleep(Duration::from_millis(backoff)).await;
        }

        // Cleanup
        let mut states = states.write().await;
        states.insert(exchange.clone(), ConnectionState::Disconnected);
    }

    /// Open a WebSocket connection and run the read loop.
    async fn connect(
        exchange: &str,
        url: &str,
        config: &ConnectionConfig,
        tick_tx: mpsc::Sender<MarketTick>,
        running: Arc<RwLock<bool>>,
        subs: Arc<RwLock<SubscriptionManager>>,
    ) -> Result<()> {
        let timeout = Duration::from_millis(config.timeout_ms);

        let ws_stream = tokio::time::timeout(timeout, tokio_tungstenite::connect_async(url))
            .await
            .map_err(|_| StreamError::Connection(format!("timeout connecting to {}", url)))?
            .map_err(|e| StreamError::Connection(format!("ws connect error: {}", e)))?
            .0;

        info!("{}: WebSocket connected", exchange);

        let (mut ws_write, mut ws_read) = ws_stream.split();

        // Re-subscribe on reconnect
        {
            let mut sub_mgr = subs.write().await;
            let resub_json = sub_mgr.resubscribe_all();
            if !resub_json.is_empty() {
                let _ = ws_write.send(TungsteniteMsg::Text(resub_json.into())).await;
            }
        }

        loop {
            // Check running flag
            if !*running.read().await {
                let _ = ws_write.send(TungsteniteMsg::Close(None)).await;
                break;
            }

            let msg = tokio::time::timeout(
                Duration::from_secs(config.heartbeat_interval_sec + 10),
                ws_read.next(),
            )
            .await;

            match msg {
                Ok(Some(Ok(TungsteniteMsg::Text(text)))) => {
                    if let Some(tick) = parse_binance_message(exchange, &text) {
                        let _ = tick_tx.send(tick).await;
                    }
                }
                Ok(Some(Ok(TungsteniteMsg::Ping(payload)))) => {
                    let _ = ws_write.send(TungsteniteMsg::Pong(payload)).await;
                }
                Ok(Some(Ok(TungsteniteMsg::Close(_)))) => {
                    debug!("{}: received close frame", exchange);
                    break;
                }
                Ok(Some(Err(e))) => {
                    warn!("{}: ws error: {}", exchange, e);
                    break;
                }
                Ok(None) => {
                    debug!("{}: stream ended", exchange);
                    break;
                }
                Err(_) => {
                    // Read timeout — send ping as heartbeat
                    let ping_result = ws_write.send(TungsteniteMsg::Ping(Vec::new().into())).await;
                    if ping_result.is_err() {
                        break;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Process incoming message — delegates to `parse_binance_message`.
    pub fn process_message(exchange: &str, data: &str) -> Option<MarketTick> {
        parse_binance_message(exchange, data)
    }

    /// Send heartbeat/ping
    pub async fn send_heartbeat(&self, exchange: &str) -> Result<()> {
        debug!("Sending heartbeat to {}", exchange);
        Ok(())
    }
}

impl std::fmt::Debug for WebSocketManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketManager")
            .field("exchanges", &self.configs.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let (tx, _rx) = mpsc::channel(100);
        let manager = WebSocketManager::new(tx);

        assert!(manager.get_all_feeds().await.is_empty());
    }

    #[tokio::test]
    async fn test_register_exchange() {
        let (tx, _rx) = mpsc::channel(100);
        let mut manager = WebSocketManager::new(tx);

        let config = ConnectionConfig {
            exchange: "test".to_string(),
            ..Default::default()
        };

        manager.register_exchange(config).await;

        let feed = manager.get_feed("test").await;
        assert!(feed.is_some());
        assert_eq!(feed.unwrap().state, ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_start_stop() {
        let (tx, _rx) = mpsc::channel(100);
        let mut manager = WebSocketManager::new(tx);

        let config = ConnectionConfig {
            exchange: "test".to_string(),
            ..Default::default()
        };

        manager.register_exchange(config).await;

        // start() will attempt a real connection that will fail,
        // but the manager itself should not panic.
        manager.start().await.unwrap();
        sleep(Duration::from_millis(200)).await;

        let state = manager.get_state("test").await;
        assert!(state.is_some());

        manager.stop().await;
    }

    #[test]
    fn test_parse_binance_trade() {
        let json =
            r#"{"e":"trade","s":"BTCUSDT","p":"50000.50","q":"0.5","m":true,"T":1709500000000}"#;
        let tick = parse_binance_message("binance", json).unwrap();

        assert_eq!(tick.symbol, "BTCUSDT");
        assert_eq!(tick.exchange, "binance");
        assert_eq!(tick.price, Decimal::from_str("50000.50").unwrap());
        assert_eq!(tick.quantity, Decimal::from_str("0.5").unwrap());
        assert_eq!(tick.side, Some(Side::Ask)); // is_buyer_maker=true → Ask
        assert_eq!(tick.tick_type, TickType::Trade);
    }

    #[test]
    fn test_parse_binance_book_ticker() {
        let json = r#"{"s":"BTCUSDT","b":"50000","B":"1.5","a":"50001","A":"2.0"}"#;
        let tick = parse_binance_message("binance", json).unwrap();

        assert_eq!(tick.symbol, "BTCUSDT");
        assert_eq!(tick.side, None);
        assert_eq!(tick.tick_type, TickType::Bid);
    }

    #[test]
    fn test_parse_combined_stream() {
        let json = r#"{"stream":"btcusdt@trade","data":{"e":"trade","s":"BTCUSDT","p":"50000","q":"0.5","m":false,"T":1709500000000}}"#;
        let tick = parse_binance_message("binance", json).unwrap();

        assert_eq!(tick.symbol, "BTCUSDT");
        assert_eq!(tick.side, Some(Side::Bid)); // is_buyer_maker=false → Bid
    }

    #[test]
    fn test_parse_invalid_message() {
        assert!(parse_binance_message("binance", "not json").is_none());
        assert!(parse_binance_message("binance", r#"{"foo":"bar"}"#).is_none());
    }

    #[test]
    fn test_process_message_backward_compat() {
        let json =
            r#"{"e":"trade","s":"BTCUSDT","p":"50000","q":"0.1","m":false,"T":1709500000000}"#;
        let tick = WebSocketManager::process_message("binance", json);
        assert!(tick.is_some());
        assert_eq!(tick.unwrap().tick_type, TickType::Trade);
    }

    #[test]
    fn test_subscription_manager() {
        let mut mgr = SubscriptionManager::new();

        let json = mgr.subscribe(&["btcusdt@trade".to_string()]);
        assert!(json.contains("SUBSCRIBE"));
        assert!(json.contains("btcusdt@trade"));

        // Duplicate subscribe returns empty
        let dup = mgr.subscribe(&["btcusdt@trade".to_string()]);
        assert!(dup.is_empty());

        let unsub = mgr.unsubscribe(&["btcusdt@trade".to_string()]);
        assert!(unsub.contains("UNSUBSCRIBE"));

        // Unsubscribe again returns empty
        let dup_unsub = mgr.unsubscribe(&["btcusdt@trade".to_string()]);
        assert!(dup_unsub.is_empty());
    }

    #[test]
    fn test_build_ws_url_single() {
        let url = build_ws_url("wss://stream.binance.com:9443/ws", &["BTCUSDT".to_string()]);
        assert_eq!(url, "wss://stream.binance.com:9443/ws/btcusdt@trade");
    }

    #[test]
    fn test_build_ws_url_multi() {
        let url = build_ws_url(
            "wss://stream.binance.com:9443/ws",
            &["BTCUSDT".to_string(), "ETHUSDT".to_string()],
        );
        assert!(url.contains("/stream?streams="));
        assert!(url.contains("btcusdt@trade"));
        assert!(url.contains("ethusdt@trade"));
    }

    #[test]
    fn test_backoff_with_jitter() {
        let b0 = backoff_with_jitter(1000, 0);
        assert!(b0 >= 1000 && b0 <= 1250);

        let b5 = backoff_with_jitter(1000, 5);
        assert!(b5 >= 32000 && b5 <= 40000);

        // Cap at 60s
        let b10 = backoff_with_jitter(1000, 10);
        assert!(b10 <= 75_000);
    }

    #[test]
    fn test_epoch_ms_to_datetime() {
        let dt = epoch_ms_to_datetime(1709500000000).unwrap();
        assert_eq!(dt.timestamp(), 1709500000);
    }
}
