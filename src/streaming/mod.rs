//! Real-Time Streaming Engine - Sprint 12

use rust_decimal::Decimal;
use tokio::sync::broadcast;

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

/// Streaming engine coordinator
#[derive(Debug)]
pub struct StreamingEngine {
    signal_tx: broadcast::Sender<TradingSignal>,
}

impl StreamingEngine {
    pub fn new() -> Self {
        let (signal_tx, _) = broadcast::channel(1000);
        Self { signal_tx }
    }
    
    pub async fn start(&mut self) -> Result<(), StreamError> {
        tracing::info!("Streaming engine started");
        Ok(())
    }
    
    pub fn subscribe_signals(&self) -> broadcast::Receiver<TradingSignal> {
        self.signal_tx.subscribe()
    }
}

impl Default for StreamingEngine {
    fn default() -> Self { Self::new() }
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
}
