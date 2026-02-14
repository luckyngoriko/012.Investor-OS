//! Paper Trading Engine
//!
//! Integrates paper trading with streaming data and strategy signals

use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::analytics::{Signal, SignalDirection};
use crate::broker::{
    paper::{PaperBroker, PaperPortfolio},
    Broker, Order, OrderSide, OrderType,
};
use crate::streaming::MarketTick;

/// Paper trading engine configuration
#[derive(Debug, Clone)]
pub struct PaperTradingConfig {
    /// Initial cash balance
    pub initial_balance: Decimal,
    /// Commission rate per trade (e.g., 0.001 = 0.1%)
    pub commission_rate: Decimal,
    /// Position size as % of portfolio (0.0-1.0)
    pub position_size_pct: Decimal,
    /// Enable auto-trading from signals
    pub auto_trade: bool,
    /// Max positions to hold
    pub max_positions: usize,
    /// Logging interval for portfolio summary
    pub log_interval_secs: u64,
}

impl Default for PaperTradingConfig {
    fn default() -> Self {
        Self {
            initial_balance: Decimal::from(100000),
            commission_rate: Decimal::from(1) / Decimal::from(1000), // 0.1%
            position_size_pct: Decimal::from(10) / Decimal::from(100), // 10%
            auto_trade: true,
            max_positions: 10,
            log_interval_secs: 60,
        }
    }
}

/// Paper trading engine
#[derive(Debug, Clone)]
pub struct PaperTradingEngine {
    config: PaperTradingConfig,
    broker: Arc<PaperBroker>,
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<PaperTradingStats>>,
}

/// Trading statistics
#[derive(Debug, Clone, Default)]
pub struct PaperTradingStats {
    pub signals_received: u64,
    pub trades_executed: u64,
    pub errors: u64,
    pub start_time: Option<chrono::DateTime<Utc>>,
}

impl PaperTradingEngine {
    /// Create new paper trading engine
    pub fn new(
        config: PaperTradingConfig,
        broker: Arc<PaperBroker>,
    ) -> Self {
        Self {
            config,
            broker,
            running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(PaperTradingStats::default())),
        }
    }

    /// Start paper trading
    pub async fn start(&self) -> Result<(), String> {
        info!("Starting paper trading engine...");

        *self.running.write().await = true;
        
        let mut stats = self.stats.write().await;
        stats.start_time = Some(Utc::now());
        drop(stats);

        // Start market data processing
        let _broker = self.broker.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            debug!("Market data processor started");
            
            while *running.read().await {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        // Start signal processing
        let broker = self.broker.clone();
        let config = self.config.clone();
        let running = self.running.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            debug!("Signal processor started");
            
            // Create signal channel
            let (tx, mut rx) = mpsc::channel::<Signal>(100);
            drop(tx); // Will be used by external signal source
            
            while *running.read().await {
                match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
                    Ok(Some(signal)) => {
                        stats.write().await.signals_received += 1;
                        
                        if config.auto_trade {
                            if let Err(e) = Self::process_signal(&broker, &config, signal).await {
                                warn!("Signal processing error: {}", e);
                                stats.write().await.errors += 1;
                            } else {
                                stats.write().await.trades_executed += 1;
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(_) => continue, // Timeout
                }
            }
        });

        // Start periodic logging
        let broker = self.broker.clone();
        let config = self.config.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(config.log_interval_secs));
            
            while *running.read().await {
                ticker.tick().await;
                
                let portfolio = broker.get_portfolio().await;
                let prices = HashMap::new(); // Would get from streaming
                let summary = portfolio.summary(&prices);
                
                info!(
                    "Paper Portfolio - Equity: ${}, Return: {:.2}%, Positions: {}, Trades: {}",
                    summary.equity, summary.total_return_pct, 
                    summary.position_count, summary.trade_count
                );
            }
        });

        info!("Paper trading engine started");
        Ok(())
    }

    /// Stop paper trading
    pub async fn stop(&self) -> Result<(), String> {
        info!("Stopping paper trading engine...");
        *self.running.write().await = false;
        
        // Print final stats
        let stats = self.stats.read().await.clone();
        info!("Paper Trading Stats: {:?}", stats);
        
        Ok(())
    }

    /// Process a trading signal
    async fn process_signal(
        broker: &PaperBroker,
        config: &PaperTradingConfig,
        signal: Signal,
    ) -> Result<(), String> {
        debug!("Processing signal: {:?}", signal);

        // Get current portfolio
        let portfolio = broker.get_portfolio().await;
        
        // Check max positions
        if portfolio.positions().len() >= config.max_positions
            && portfolio.get_position(&signal.ticker).is_none() {
                return Err("Max positions reached".to_string());
            }

        // Calculate position size
        let position_value = portfolio.cash_balance() * config.position_size_pct;
        
        // Get current price from order book
        let quantity = if let Some(book) = broker.get_order_book(&signal.ticker).await {
            let price = match signal.direction {
                SignalDirection::Long => book.best_ask().map(|l| l.price),
                SignalDirection::Short => book.best_bid().map(|l| l.price),
                _ => None,
            };
            
            price.map(|p| position_value / p).unwrap_or(Decimal::ZERO)
        } else {
            Decimal::ZERO
        };

        if quantity.is_zero() {
            return Err("Could not determine quantity".to_string());
        }

        // Determine order side
        let side = match signal.direction {
            SignalDirection::Long => OrderSide::Buy,
            SignalDirection::Short => OrderSide::Sell,
            _ => return Err("Neutral signal, no action".to_string()),
        };

        // Check if we need to close opposite position first
        if let Some(pos) = portfolio.get_position(&signal.ticker) {
            match (side, pos.is_long()) {
                (OrderSide::Buy, false) | (OrderSide::Sell, true) => {
                    // Close existing position
                    let portfolio_id = Uuid::new_v4();
                    let mut close_order = Order::new(
                        signal.ticker.clone(),
                        if pos.is_long() { OrderSide::Sell } else { OrderSide::Buy },
                        pos.quantity.abs(),
                        OrderType::Market,
                        portfolio_id,
                    ).with_notes("Close position");
                    
                    broker.place_order(&mut close_order).await
                        .map_err(|e| format!("Close order failed: {}", e))?;
                    
                    info!("Closed position for {}: {:?}", signal.ticker, pos);
                }
                _ => {} // Same direction, add to position
            }
        }

        // Place new order
        let portfolio_id = Uuid::new_v4();
        let mut order = Order::new(
            signal.ticker.clone(),
            side,
            quantity,
            OrderType::Market,
            portfolio_id,
        ).with_notes(format!("Signal confidence: {}", signal.confidence));

        broker.place_order(&mut order).await
            .map_err(|e| format!("Order failed: {}", e))?;
        
        info!(
            "Executed paper trade for {}: {:?} {} (confidence: {})",
            signal.ticker, side, quantity, signal.confidence
        );

        Ok(())
    }

    /// Update market data (called by streaming)
    pub async fn on_market_tick(&self, tick: &MarketTick) {
        self.broker.update_market_data(tick).await;
    }

    /// Get current stats
    pub async fn get_stats(&self) -> PaperTradingStats {
        self.stats.read().await.clone()
    }

    /// Get portfolio snapshot
    pub async fn get_portfolio(&self) -> PaperPortfolio {
        self.broker.get_portfolio().await
    }

    /// Check if running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::BrokerConfig;

    fn create_test_broker() -> Arc<PaperBroker> {
        let config = BrokerConfig {
            broker_type: crate::broker::BrokerType::InteractiveBrokers,
            account_id: "paper".to_string(),
            api_url: "".to_string(),
            auth_token: None,
            paper_trading: true,
            default_order_type: OrderType::Market,
            max_position_size: Decimal::from(100000),
            max_order_size: Decimal::from(50000),
            daily_loss_limit: Decimal::from(5000),
        };
        Arc::new(PaperBroker::new(config))
    }

    #[tokio::test]
    async fn test_engine_creation() {
        let broker = create_test_broker();

        let config = PaperTradingConfig::default();
        let engine = PaperTradingEngine::new(config, broker);

        assert!(!engine.is_running().await);
    }

    #[tokio::test]
    async fn test_engine_start_stop() {
        let broker = create_test_broker();

        let config = PaperTradingConfig::default();
        let engine = PaperTradingEngine::new(config, broker);

        engine.start().await.unwrap();
        assert!(engine.is_running().await);

        engine.stop().await.unwrap();
        assert!(!engine.is_running().await);
    }
}
