//! WebSocket Connection Manager

use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::{MarketTick, Result, TickType, Side};

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
    pub last_message: Option<chrono::DateTime<chrono::Utc>>,
    pub messages_received: u64,
    pub reconnect_count: u32,
    pub latency_ms: u64,
}

/// WebSocket message
#[derive(Debug, Clone)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping,
    Pong,
    Close,
}

/// WebSocket Manager
pub struct WebSocketManager {
    /// Connection configurations
    configs: HashMap<String, ConnectionConfig>,
    /// Connection states
    states: Arc<RwLock<HashMap<String, ConnectionState>>>,
    /// Feed statistics
    feeds: Arc<RwLock<HashMap<String, ExchangeFeed>>>,
    /// Tick output channel
    tick_tx: mpsc::Sender<MarketTick>,
    /// Running flag
    running: Arc<RwLock<bool>>,
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
        }
    }

    /// Register an exchange feed
    pub async fn register_exchange(&mut self, config: ConnectionConfig) {
        let exchange = config.exchange.clone();
        
        self.configs.insert(exchange.clone(), config);
        
        let mut states = self.states.write().await;
        states.insert(exchange.clone(), ConnectionState::Disconnected);
        
        let mut feeds = self.feeds.write().await;
        feeds.insert(exchange.clone(), ExchangeFeed {
            exchange: exchange.clone(),
            state: ConnectionState::Disconnected,
            last_message: None,
            messages_received: 0,
            reconnect_count: 0,
            latency_ms: 0,
        });
        
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
            
            tokio::spawn(async move {
                Self::connection_loop(exchange, config, states, feeds, tick_tx, running).await;
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

    /// Connection loop with reconnection logic
    async fn connection_loop(
        exchange: String,
        config: ConnectionConfig,
        states: Arc<RwLock<HashMap<String, ConnectionState>>>,
        feeds: Arc<RwLock<HashMap<String, ExchangeFeed>>>,
        tick_tx: mpsc::Sender<MarketTick>,
        running: Arc<RwLock<bool>>,
    ) {
        let mut reconnect_attempts = 0;
        
        while *running.read().await {
            // Set connecting state
            {
                let mut states = states.write().await;
                states.insert(exchange.clone(), ConnectionState::Connecting);
            }
            
            // Attempt connection (simulated for testing)
            match Self::connect(&exchange, &config, tick_tx.clone()).await {
                Ok(()) => {
                    reconnect_attempts = 0;
                    
                    // Update state to connected
                    {
                        let mut states = states.write().await;
                        states.insert(exchange.clone(), ConnectionState::Connected);
                    }
                    
                    {
                        let mut feeds = feeds.write().await;
                        if let Some(feed) = feeds.get_mut(&exchange) {
                            feed.state = ConnectionState::Connected;
                            feed.reconnect_count = 0;
                        }
                    }
                    
                    info!("Connected to {}", exchange);
                    
                    // Keep connection alive
                    while *running.read().await {
                        sleep(Duration::from_secs(1)).await;
                        
                        // Update heartbeat
                        let mut feeds = feeds.write().await;
                        if let Some(feed) = feeds.get_mut(&exchange) {
                            feed.last_message = Some(Utc::now());
                        }
                    }
                }
                Err(e) => {
                    error!("Connection to {} failed: {}", exchange, e);
                    reconnect_attempts += 1;
                    
                    // Update state
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
                    
                    // Exponential backoff
                    let backoff = config.reconnect_interval_ms * 2u64.pow(reconnect_attempts.min(5));
                    warn!("Reconnecting to {} in {}ms", exchange, backoff);
                    sleep(Duration::from_millis(backoff)).await;
                }
            }
        }
        
        // Cleanup
        let mut states = states.write().await;
        states.insert(exchange.clone(), ConnectionState::Disconnected);
    }

    /// Connect to exchange (simulated)
    async fn connect(
        _exchange: &str,
        _config: &ConnectionConfig,
        _tick_tx: mpsc::Sender<MarketTick>,
    ) -> Result<()> {
        // In production, this would establish actual WebSocket connection
        // For testing, we simulate success
        sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    /// Process incoming message (simulated)
    fn process_message(
        exchange: &str,
        data: &str,
    ) -> Option<MarketTick> {
        // Parse JSON message (simplified)
        // In production, parse actual exchange message format
        
        if data.contains("trade") {
            Some(MarketTick {
                symbol: "BTCUSDT".to_string(),
                exchange: exchange.to_string(),
                price: Decimal::try_from(50000.0).ok()?,
                quantity: Decimal::try_from(0.1).ok()?,
                side: Side::Bid,
                timestamp: Utc::now(),
                tick_type: TickType::Trade,
            })
        } else {
            None
        }
    }

    /// Send heartbeat/ping
    async fn send_heartbeat(&self, exchange: &str) -> Result<()> {
        debug!("Sending heartbeat to {}", exchange);
        // In production, send actual ping frame
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
        
        manager.start().await.unwrap();
        sleep(Duration::from_millis(200)).await;
        
        // Should be connected or connecting
        let state = manager.get_state("test").await;
        assert!(state.is_some());
        
        manager.stop().await;
    }

    #[tokio::test]
    async fn test_process_message() {
        let tick = WebSocketManager::process_message("binance", r#"{"type":"trade","price":"50000"}"#);
        
        assert!(tick.is_some());
        let tick = tick.unwrap();
        assert_eq!(tick.exchange, "binance");
        assert_eq!(tick.tick_type, TickType::Trade);
    }
}
