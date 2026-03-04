//! Real-Time Streaming Engine - Sprint 12 + Sprint 112

use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

// Re-export submodules
pub mod orderbook;
pub mod trade_analyzer;
pub mod websocket;

// Re-export types from submodules for backward compatibility
pub use orderbook::{BookUpdate, OrderBook, PriceLevel, Side, UpdateType};
pub use websocket::{ConnectionConfig, ConnectionState, ExchangeFeed, WebSocketManager, WsMessage};

/// Real-time market data stream
#[derive(Debug, Clone)]
pub struct MarketDataStream {
    pub symbol: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub last_price: Decimal,
    pub volume_24h: Decimal,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Market tick (for websocket compatibility)
#[derive(Debug, Clone)]
pub struct MarketTick {
    pub exchange: String,
    pub symbol: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: Option<orderbook::Side>,
    pub tick_type: TickType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Tick type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickType {
    Trade,
    Bid,
    Ask,
    BookDelta,
}

/// Trading signal from streaming engine
#[derive(Debug, Clone)]
pub struct TradingSignal {
    pub symbol: String,
    pub signal_type: SignalType,
    pub confidence: f64,
    pub cq_score: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum SignalType {
    Buy,
    Sell,
    Hold,
    StrongBuy,
    StrongSell,
}

/// Streaming errors
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Connection: {0}")]
    Connection(String),
    #[error("Processing: {0}")]
    Processing(String),
    #[error("Stream disconnected: {0}")]
    StreamDisconnected(String),
}

pub type Result<T> = std::result::Result<T, StreamError>;
pub type StreamingError = StreamError;

// ---------------------------------------------------------------------------
// StreamingConfig — configuration for the streaming engine
// ---------------------------------------------------------------------------

/// Configuration for the streaming engine (used by `new_with_config`).
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum acceptable tick-to-signal latency (ms).
    pub max_latency_ms: u64,
    /// Minimum ms between emitted signals for the same symbol.
    pub signal_cooldown_ms: u64,
    /// Whether to drop duplicate ticks with the same timestamp+price.
    pub enable_deduplication: bool,
    /// Rolling window for the per-symbol TradeAnalyzer (seconds).
    pub trade_window_sec: i64,
    /// Quantity threshold for "large trade" classification.
    pub large_trade_threshold: Decimal,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_latency_ms: 100,
            signal_cooldown_ms: 500,
            enable_deduplication: true,
            trade_window_sec: 60,
            large_trade_threshold: Decimal::from(10),
        }
    }
}

// ---------------------------------------------------------------------------
// StreamingEngine
// ---------------------------------------------------------------------------

/// Streaming engine wrapping WebSocket manager and signal broadcast.
pub struct StreamingEngine {
    signal_tx: tokio::sync::broadcast::Sender<TradingSignal>,
    ws_manager: Option<websocket::WebSocketManager>,
    tick_tx: Option<mpsc::Sender<MarketTick>>,
    running: bool,
    config: StreamingConfig,
    orderbooks: Arc<RwLock<HashMap<String, OrderBook>>>,
    analyzers: Arc<RwLock<HashMap<String, trade_analyzer::TradeAnalyzer>>>,
    /// Channel for external tick injection (returned by `new_with_config`)
    ext_tick_tx: Option<mpsc::Sender<MarketTick>>,
    /// Signal output channel (mpsc, used by `new_with_config`)
    signal_out_tx: Option<mpsc::Sender<TradingSignal>>,
    /// Handle of the background tick processing task
    tick_task: Option<tokio::task::JoinHandle<()>>,
}

impl StreamingEngine {
    /// Create a new streaming engine with a default broadcast capacity.
    pub fn new() -> Self {
        let (signal_tx, _) = tokio::sync::broadcast::channel(1024);
        Self {
            signal_tx,
            ws_manager: None,
            tick_tx: None,
            running: false,
            config: StreamingConfig::default(),
            orderbooks: Arc::new(RwLock::new(HashMap::new())),
            analyzers: Arc::new(RwLock::new(HashMap::new())),
            ext_tick_tx: None,
            signal_out_tx: None,
            tick_task: None,
        }
    }

    /// Create a streaming engine with explicit config and an mpsc signal output.
    ///
    /// Returns `(engine, tick_sender)` — the tick_sender can be used to inject
    /// ticks directly (useful for testing or when WS data arrives externally).
    pub fn new_with_config(
        config: StreamingConfig,
        signal_tx: mpsc::Sender<TradingSignal>,
    ) -> (Self, mpsc::Sender<MarketTick>) {
        let (broadcast_tx, _) = tokio::sync::broadcast::channel(1024);
        let (tick_tx, tick_rx) = mpsc::channel(4096);

        let orderbooks: Arc<RwLock<HashMap<String, OrderBook>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let analyzers: Arc<RwLock<HashMap<String, trade_analyzer::TradeAnalyzer>>> =
            Arc::new(RwLock::new(HashMap::new()));

        // Spawn background tick processing task
        let ob_clone = orderbooks.clone();
        let an_clone = analyzers.clone();
        let sig_tx = signal_tx.clone();
        let bcast_tx = broadcast_tx.clone();
        let cfg = config.clone();

        let tick_task = tokio::spawn(Self::tick_processing_loop(
            tick_rx, ob_clone, an_clone, sig_tx, bcast_tx, cfg,
        ));

        let engine = Self {
            signal_tx: broadcast_tx,
            ws_manager: None,
            tick_tx: Some(tick_tx.clone()),
            running: false,
            config,
            orderbooks,
            analyzers,
            ext_tick_tx: Some(tick_tx.clone()),
            signal_out_tx: Some(signal_tx),
            tick_task: Some(tick_task),
        };

        (engine, tick_tx)
    }

    /// Background loop: reads ticks, updates orderbooks + analyzers, emits signals.
    async fn tick_processing_loop(
        mut tick_rx: mpsc::Receiver<MarketTick>,
        orderbooks: Arc<RwLock<HashMap<String, OrderBook>>>,
        analyzers: Arc<RwLock<HashMap<String, trade_analyzer::TradeAnalyzer>>>,
        signal_tx: mpsc::Sender<TradingSignal>,
        broadcast_tx: tokio::sync::broadcast::Sender<TradingSignal>,
        _config: StreamingConfig,
    ) {
        while let Some(tick) = tick_rx.recv().await {
            let symbol = tick.symbol.clone();

            // Update orderbook with tick data
            {
                let mut books = orderbooks.write().await;
                if let Some(book) = books.get_mut(&symbol) {
                    if let Some(side) = tick.side {
                        book.apply_update(BookUpdate {
                            side,
                            price: tick.price,
                            quantity: tick.quantity,
                            update_type: UpdateType::Modify,
                            timestamp: tick.timestamp,
                        });
                    }
                }
            }

            // Run trade analyzer
            let signal_opt = {
                let books = orderbooks.read().await;
                let mut analyzers = analyzers.write().await;
                if let Some(analyzer) = analyzers.get_mut(&symbol) {
                    let best_bid = books
                        .get(&symbol)
                        .and_then(|b| b.best_bid())
                        .map(|l| l.price)
                        .unwrap_or(tick.price);
                    let best_ask = books
                        .get(&symbol)
                        .and_then(|b| b.best_ask())
                        .map(|l| l.price)
                        .unwrap_or(tick.price);

                    let analyzed = analyzer.analyze(tick, best_bid, best_ask);
                    let flow = analyzer.get_flow();

                    // Generate signal from trade flow
                    if flow.trade_count >= 2 {
                        let signal_type = if flow.buy_pressure > Decimal::try_from(0.7).unwrap() {
                            SignalType::StrongBuy
                        } else if flow.buy_pressure > Decimal::try_from(0.55).unwrap() {
                            SignalType::Buy
                        } else if flow.buy_pressure < Decimal::try_from(0.3).unwrap() {
                            SignalType::StrongSell
                        } else if flow.buy_pressure < Decimal::try_from(0.45).unwrap() {
                            SignalType::Sell
                        } else {
                            SignalType::Hold
                        };

                        Some(TradingSignal {
                            symbol: symbol.clone(),
                            signal_type,
                            confidence: flow.buy_pressure.try_into().unwrap_or(0.5),
                            cq_score: analyzed.notional.try_into().unwrap_or(0.0),
                            timestamp: chrono::Utc::now(),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(signal) = signal_opt {
                let _ = signal_tx.send(signal.clone()).await;
                let _ = broadcast_tx.send(signal);
            }
        }
    }

    /// Register a symbol for tracking (creates OrderBook + TradeAnalyzer).
    pub async fn register_symbol(&mut self, symbol: String, exchange: String) {
        let mut books = self.orderbooks.write().await;
        books
            .entry(symbol.clone())
            .or_insert_with(|| OrderBook::new(symbol.clone(), exchange.clone()));

        let mut analyzers = self.analyzers.write().await;
        analyzers.entry(symbol).or_insert_with(|| {
            trade_analyzer::TradeAnalyzer::new(
                self.config.trade_window_sec,
                self.config.large_trade_threshold,
            )
        });
    }

    /// Get a clone of the orderbook for a symbol.
    pub async fn get_orderbook(&self, symbol: &str) -> Option<OrderBook> {
        let books = self.orderbooks.read().await;
        books.get(symbol).cloned()
    }

    /// Start the streaming engine (initialises internal WebSocket manager).
    pub async fn start(&mut self) -> Result<()> {
        if self.tick_tx.is_none() {
            let (tick_tx, _tick_rx) = mpsc::channel(4096);
            let manager = websocket::WebSocketManager::new(tick_tx.clone());
            manager.start().await?;
            self.ws_manager = Some(manager);
            self.tick_tx = Some(tick_tx);
        }
        self.running = true;
        Ok(())
    }

    /// Stop the streaming engine.
    pub async fn stop(&mut self) {
        self.running = false;
        if let Some(ws) = &self.ws_manager {
            ws.stop().await;
        }
    }

    /// Subscribe to trading signals. Returns a broadcast receiver.
    pub fn subscribe_signals(&self) -> tokio::sync::broadcast::Receiver<TradingSignal> {
        self.signal_tx.subscribe()
    }

    /// Whether the engine is currently running.
    pub fn is_running(&self) -> bool {
        self.running
    }
}
