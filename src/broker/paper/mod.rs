//! Paper Trading Broker
//!
//! Sprint 25: Paper Trading & Backtesting Integration
//! - Simulates trades with real-time market data
//! - Tracks virtual positions, P&L, and performance
//! - Integrates with backtesting engine

use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use crate::broker::{
    AccountInfo, Broker, BrokerConfig, BrokerError, BrokerType, Execution, Order, OrderSide,
    OrderStatus, OrderType, Position, Result,
};
use crate::streaming::{MarketTick, OrderBook, Side as BookSide};

pub mod engine;
pub mod portfolio;

pub use engine::PaperTradingEngine;
pub use portfolio::{PaperPortfolio, PaperPosition};

/// Paper trading broker - simulates execution against market data
#[derive(Debug)]
pub struct PaperBroker {
    config: BrokerConfig,
    portfolio: RwLock<PaperPortfolio>,
    order_book: RwLock<HashMap<String, OrderBook>>,
    orders: RwLock<HashMap<Uuid, Order>>,
    executions: RwLock<Vec<Execution>>,
    connected: RwLock<bool>,
}

impl PaperBroker {
    /// Create new paper trading broker
    pub fn new(config: BrokerConfig) -> Self {
        Self {
            portfolio: RwLock::new(PaperPortfolio::new(
                Decimal::from(100000), // Default initial balance
            )),
            order_book: RwLock::new(HashMap::new()),
            orders: RwLock::new(HashMap::new()),
            executions: RwLock::new(Vec::new()),
            connected: RwLock::new(false),
            config,
        }
    }

    /// Update market data for a symbol
    pub async fn update_market_data(&self, tick: &MarketTick) {
        let mut books = self.order_book.write().await;
        let book = books.entry(tick.symbol.clone()).or_insert_with(|| {
            OrderBook::new(tick.symbol.clone(), tick.exchange.clone())
        });

        // Update book based on tick type
        use crate::streaming::TickType;
        use crate::streaming::orderbook::{BookUpdate, UpdateType};
        
        match tick.tick_type {
            TickType::Bid => {
                book.apply_update(BookUpdate {
                    side: BookSide::Bid,
                    price: tick.price,
                    quantity: tick.quantity,
                    update_type: UpdateType::Modify,
                    timestamp: tick.timestamp,
                });
            }
            TickType::Ask => {
                book.apply_update(BookUpdate {
                    side: BookSide::Ask,
                    price: tick.price,
                    quantity: tick.quantity,
                    update_type: UpdateType::Modify,
                    timestamp: tick.timestamp,
                });
            }
            _ => {}
        }

        // Process any pending orders against updated book
        drop(books);
        self.process_pending_orders(&tick.symbol).await;
    }

    /// Process pending limit orders against current book
    async fn process_pending_orders(&self, symbol: &str) {
        let mut orders = self.orders.write().await;
        let books = self.order_book.read().await;
        
        let Some(book) = books.get(symbol) else {
            return;
        };

        for order in orders.values_mut() {
            if order.ticker != *symbol || !order.is_active() {
                continue;
            }

            match order.order_type {
                OrderType::Market => {
                    // Market orders fill immediately at best price
                    let fill_price = match order.side {
                        OrderSide::Buy => book.best_ask().map(|l| l.price),
                        OrderSide::Sell => book.best_bid().map(|l| l.price),
                    };

                    if let Some(price) = fill_price {
                        self.fill_order(order, price).await;
                    }
                }
                OrderType::Limit => {
                    let Some(limit_price) = order.limit_price else { continue };
                    
                    // Check if limit order should fill
                    let should_fill = match order.side {
                        OrderSide::Buy => {
                            // Buy limit fills if best ask <= limit price
                            book.best_ask()
                                .map(|l| l.price <= limit_price)
                                .unwrap_or(false)
                        }
                        OrderSide::Sell => {
                            // Sell limit fills if best bid >= limit price
                            book.best_bid()
                                .map(|l| l.price >= limit_price)
                                .unwrap_or(false)
                        }
                    };

                    if should_fill {
                        let fill_price = match order.side {
                            OrderSide::Buy => limit_price.min(book.best_ask().unwrap().price),
                            OrderSide::Sell => limit_price.max(book.best_bid().unwrap().price),
                        };
                        self.fill_order(order, fill_price).await;
                    }
                }
                _ => {} // Other order types not yet implemented
            }
        }
    }

    /// Fill an order at given price
    async fn fill_order(&self, order: &mut Order, price: Decimal) {
        let commission_rate = Decimal::from(1) / Decimal::from(1000); // 0.1%
        let notional = order.remaining_quantity() * price;
        let commission = notional * commission_rate;

        let execution = Execution {
            id: Uuid::new_v4(),
            order_id: order.id,
            broker_execution_id: format!("paper-{}", Uuid::new_v4()),
            ticker: order.ticker.clone(),
            side: order.side,
            quantity: order.remaining_quantity(),
            price,
            commission,
            timestamp: Utc::now(),
        };

        // Update order
        order.filled_quantity = order.quantity;
        order.avg_fill_price = Some(price);
        order.commission = Some(commission);
        order.status = OrderStatus::Filled;
        order.updated_at = Utc::now();

        // Update portfolio
        {
            let mut portfolio = self.portfolio.write().await;
            portfolio.apply_execution(&execution);
        }

        // Record execution
        {
            let mut executions = self.executions.write().await;
            executions.push(execution);
        }

        info!(
            "Paper trade filled: {:?} {} {} @ {} (comm: {})",
            order.side, order.quantity, order.ticker, price, commission
        );
    }

    /// Get portfolio snapshot
    pub async fn get_portfolio(&self) -> PaperPortfolio {
        self.portfolio.read().await.clone()
    }

    /// Get order book for symbol
    pub async fn get_order_book(&self, symbol: &str) -> Option<OrderBook> {
        self.order_book.read().await.get(symbol).cloned()
    }
}

#[async_trait::async_trait]
impl Broker for PaperBroker {
    async fn connect(&mut self) -> Result<()> {
        *self.connected.write().await = true;
        info!("Paper trading broker connected (simulation mode)");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        *self.connected.write().await = false;
        info!("Paper trading broker disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Blocking read is ok for simple boolean
        futures::executor::block_on(async { *self.connected.read().await })
    }

    async fn get_account_info(&self) -> Result<AccountInfo> {
        let portfolio = self.portfolio.read().await;
        
        Ok(AccountInfo {
            account_id: "paper-account".to_string(),
            cash_balance: portfolio.cash_balance(),
            buying_power: portfolio.cash_balance(), // No margin in paper
            equity_with_loan: portfolio.cash_balance(),
            net_liquidation: portfolio.cash_balance(), // Simplified
            unrealized_pnl: portfolio.total_unrealized_pnl(),
            realized_pnl: portfolio.total_realized_pnl(),
            currency: "USD".to_string(),
            updated_at: Utc::now(),
        })
    }

    async fn get_positions(&self) -> Result<Vec<Position>> {
        let portfolio = self.portfolio.read().await;
        Ok(portfolio.to_positions())
    }

    async fn get_position(&self, ticker: &str) -> Result<Option<Position>> {
        let portfolio = self.portfolio.read().await;
        Ok(portfolio.get_position(ticker).map(|p| p.to_position()))
    }

    async fn place_order(&self, order: &mut Order) -> Result<()> {
        debug!("Paper broker placing order: {:?}", order);
        
        order.status = OrderStatus::Submitted;
        order.updated_at = Utc::now();

        // For market orders, try to fill immediately
        if matches!(order.order_type, OrderType::Market) {
            let books = self.order_book.read().await;
            if let Some(book) = books.get(&order.ticker) {
                let fill_price = match order.side {
                    OrderSide::Buy => book.best_ask().map(|l| l.price),
                    OrderSide::Sell => book.best_bid().map(|l| l.price),
                };
                drop(books);

                if let Some(price) = fill_price {
                    let mut order_clone = order.clone();
                    self.fill_order(&mut order_clone, price).await;
                    *order = order_clone;
                    
                    let mut orders = self.orders.write().await;
                    orders.insert(order.id, order.clone());
                    return Ok(());
                }
            }
        }

        // Store order for later processing
        let mut orders = self.orders.write().await;
        orders.insert(order.id, order.clone());

        Ok(())
    }

    async fn cancel_order(&self, order: &mut Order) -> Result<()> {
        let mut orders = self.orders.write().await;
        if let Some(existing) = orders.get_mut(&order.id) {
            if existing.is_active() {
                existing.status = OrderStatus::Cancelled;
                existing.updated_at = Utc::now();
                order.status = OrderStatus::Cancelled;
                info!("Paper order cancelled: {}", order.id);
                Ok(())
            } else {
                Err(BrokerError::InvalidOrder(format!(
                    "Order {} cannot be cancelled (status: {:?})",
                    order.id, existing.status
                )))
            }
        } else {
            Err(BrokerError::OrderRejected(format!(
                "Order {} not found",
                order.id
            )))
        }
    }

    async fn modify_order(&self, order: &mut Order, new_quantity: Option<Decimal>, new_price: Option<Decimal>) -> Result<()> {
        let mut orders = self.orders.write().await;
        if let Some(existing) = orders.get_mut(&order.id) {
            if !existing.is_active() {
                return Err(BrokerError::InvalidOrder(format!(
                    "Order {} cannot be modified (status: {:?})",
                    order.id, existing.status
                )));
            }
            
            if let Some(qty) = new_quantity {
                existing.quantity = qty;
            }
            if let Some(price) = new_price {
                existing.limit_price = Some(price);
            }
            existing.updated_at = Utc::now();
            
            // Update the passed order
            order.quantity = existing.quantity;
            order.limit_price = existing.limit_price;
            order.updated_at = existing.updated_at;
            
            Ok(())
        } else {
            Err(BrokerError::OrderRejected(format!(
                "Order {} not found",
                order.id
            )))
        }
    }

    async fn get_order_status(&self, order: &mut Order) -> Result<OrderStatus> {
        let orders = self.orders.read().await;
        if let Some(existing) = orders.get(&order.id) {
            order.status = existing.status;
            order.filled_quantity = existing.filled_quantity;
            order.avg_fill_price = existing.avg_fill_price;
            Ok(existing.status)
        } else {
            Err(BrokerError::OrderRejected(format!(
                "Order {} not found",
                order.id
            )))
        }
    }

    async fn get_executions(&self, order_id: Uuid) -> Result<Vec<Execution>> {
        let executions = self.executions.read().await;
        Ok(executions
            .iter()
            .filter(|e| e.order_id == order_id)
            .cloned()
            .collect())
    }

    async fn get_market_price(&self, ticker: &str) -> Result<Decimal> {
        let books = self.order_book.read().await;
        if let Some(book) = books.get(ticker) {
            // Return mid price
            if let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask()) {
                Ok((bid.price + ask.price) / Decimal::from(2))
            } else {
                Err(BrokerError::MarketData(format!(
                    "No price available for {}",
                    ticker
                )))
            }
        } else {
            Err(BrokerError::MarketData(format!(
                "No order book for {}",
                ticker
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> BrokerConfig {
        BrokerConfig {
            broker_type: BrokerType::InteractiveBrokers,
            account_id: "paper".to_string(),
            api_url: "".to_string(),
            auth_token: None,
            paper_trading: true,
            default_order_type: OrderType::Market,
            max_position_size: Decimal::from(100000),
            max_order_size: Decimal::from(50000),
            daily_loss_limit: Decimal::from(5000),
        }
    }

    #[tokio::test]
    async fn test_paper_broker_creation() {
        let config = create_test_config();
        let broker = PaperBroker::new(config);

        let portfolio = broker.get_portfolio().await;
        assert_eq!(portfolio.cash_balance(), Decimal::from(100000));
    }

    #[tokio::test]
    async fn test_connect_disconnect() {
        let config = create_test_config();
        let mut broker = PaperBroker::new(config);

        assert!(!broker.is_connected());
        
        broker.connect().await.unwrap();
        assert!(broker.is_connected());
        
        broker.disconnect().await.unwrap();
        assert!(!broker.is_connected());
    }

    #[tokio::test]
    async fn test_get_account_info() {
        let config = create_test_config();
        let broker = PaperBroker::new(config);

        let info = broker.get_account_info().await.unwrap();
        assert_eq!(info.cash_balance, Decimal::from(100000));
        assert_eq!(info.account_id, "paper-account");
    }
}
