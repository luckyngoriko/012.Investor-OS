//! Real-Time Streaming Engine - Sprint 12

use rust_decimal::Decimal;

// Re-export submodules
pub mod orderbook;
pub mod trade_analyzer;
pub mod websocket;

// Re-export types from submodules for backward compatibility
pub use orderbook::{OrderBook, PriceLevel, BookUpdate, UpdateType, Side};

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
    pub side: orderbook::Side,
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
