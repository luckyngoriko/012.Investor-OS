//! Multi-Asset Portfolio Management - Sprint 11

use super::{binance, oanda};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Unified asset class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetClass {
    Equity,
    Crypto,
    Forex,
    ETF,
    Commodity,
}

impl std::fmt::Display for AssetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equity => write!(f, "Equity"),
            Self::Crypto => write!(f, "Crypto"),
            Self::Forex => write!(f, "Forex"),
            Self::ETF => write!(f, "ETF"),
            Self::Commodity => write!(f, "Commodity"),
        }
    }
}

/// Unified position across all asset classes
#[derive(Debug, Clone)]
pub struct MultiAssetPosition {
    pub symbol: String,
    pub asset_class: AssetClass,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub current_price: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub currency: String,
}

/// Multi-asset portfolio
#[derive(Debug, Clone, Default)]
pub struct MultiAssetPortfolio {
    pub positions: Vec<MultiAssetPosition>,
    pub cash_balances: HashMap<String, Decimal>, // currency -> amount
    pub total_value_usd: Decimal,
}

impl MultiAssetPortfolio {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add position from Binance (Crypto)
    pub fn add_crypto_positions(&mut self, balances: &[binance::Balance], prices: &HashMap<String, Decimal>) {
        for balance in balances {
            if balance.free > Decimal::ZERO || balance.locked > Decimal::ZERO {
                let total = balance.free + balance.locked;
                let symbol = format!("{}USDT", balance.asset);
                let price = prices.get(&symbol).copied().unwrap_or(Decimal::ZERO);
                let market_value = total * price;
                
                if market_value > Decimal::ZERO {
                    self.positions.push(MultiAssetPosition {
                        symbol: balance.asset.clone(),
                        asset_class: AssetClass::Crypto,
                        quantity: total,
                        avg_cost: Decimal::ZERO, // Would need history
                        current_price: price,
                        market_value,
                        unrealized_pnl: Decimal::ZERO,
                        currency: "USDT".to_string(),
                    });
                }
            }
        }
    }
    
    /// Add forex positions from OANDA
    pub fn add_forex_balance(&mut self, account: &oanda::AccountSummary) {
        self.cash_balances.insert(
            account.currency.clone(),
            account.balance
        );
    }
    
    /// Calculate total portfolio value in USD
    pub fn calculate_total_value(&mut self) {
        self.total_value_usd = self.positions.iter()
            .map(|p| p.market_value)
            .sum();
        
        // Add cash (simplified - assumes USD or converts)
        for (currency, amount) in &self.cash_balances {
            if currency == "USD" {
                self.total_value_usd += *amount;
            }
            // TODO: Convert other currencies
        }
    }
    
    /// Get allocation by asset class
    pub fn get_allocation(&self) -> HashMap<AssetClass, Decimal> {
        let mut allocation = HashMap::new();
        
        for position in &self.positions {
            let entry = allocation.entry(position.asset_class).or_insert(Decimal::ZERO);
            *entry += position.market_value;
        }
        
        // Convert to percentages
        if self.total_value_usd > Decimal::ZERO {
            for value in allocation.values_mut() {
                *value = *value / self.total_value_usd * Decimal::from(100);
            }
        }
        
        allocation
    }
    
    /// Get risk concentration (largest positions)
    pub fn get_top_positions(&self, n: usize) -> Vec<&MultiAssetPosition> {
        let mut sorted = self.positions.iter()
            .collect::<Vec<_>>();
        
        sorted.sort_by(|a, b| b.market_value.cmp(&a.market_value));
        sorted.truncate(n);
        sorted
    }
}

/// Unified order for all asset classes
#[derive(Debug, Clone)]
pub struct UnifiedOrder {
    pub symbol: String,
    pub asset_class: AssetClass,
    pub side: OrderSide,
    pub quantity: Decimal,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
}

/// Multi-asset order router
pub struct OrderRouter {
    binance: Option<binance::BinanceClient>,
    oanda: Option<oanda::OandaClient>,
}

impl OrderRouter {
    pub fn new(
        binance: Option<binance::BinanceClient>,
        oanda: Option<oanda::OandaClient>,
    ) -> Self {
        Self { binance, oanda }
    }
    
    /// Route order to appropriate exchange
    pub async fn submit_order(&self, order: UnifiedOrder) -> Result<OrderResult, OrderError> {
        match order.asset_class {
            AssetClass::Crypto => {
                if let Some(client) = &self.binance {
                    // Convert to Binance order
                    let binance_order = binance::BinanceOrder {
                        symbol: format!("{}USDT", order.symbol),
                        side: match order.side {
                            OrderSide::Buy => binance::OrderSide::BUY,
                            OrderSide::Sell => binance::OrderSide::SELL,
                        },
                        order_type: match order.order_type {
                            OrderType::Market => binance::OrderType::MARKET,
                            OrderType::Limit => binance::OrderType::LIMIT,
                            _ => binance::OrderType::MARKET,
                        },
                        quantity: Some(order.quantity),
                        price: order.price,
                    };
                    
                    let response = client.place_order(binance_order).await
                        .map_err(|e| OrderError::Execution(e.to_string()))?;
                    
                    Ok(OrderResult {
                        order_id: response.orderId.to_string(),
                        status: response.status,
                        executed_qty: response.executedQty,
                        avg_price: response.price,
                    })
                } else {
                    Err(OrderError::NoClient("Binance not configured".to_string()))
                }
            }
            AssetClass::Forex => {
                // OANDA order logic here
                Err(OrderError::NotImplemented("Forex orders".to_string()))
            }
            _ => Err(OrderError::NotSupported(format!("{:?}", order.asset_class))),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("No client: {0}")]
    NoClient(String),
    #[error("Execution: {0}")]
    Execution(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Not supported: {0}")]
    NotSupported(String),
}

#[derive(Debug, Clone)]
pub struct OrderResult {
    pub order_id: String,
    pub status: String,
    pub executed_qty: Decimal,
    pub avg_price: Decimal,
}

/// Portfolio rebalancing
pub struct Rebalancer {
    target_allocations: HashMap<AssetClass, Decimal>,
}

impl Rebalancer {
    pub fn new(target_allocations: HashMap<AssetClass, Decimal>) -> Self {
        Self { target_allocations }
    }
    
    /// Calculate rebalancing trades needed
    pub fn calculate_trades(&self, portfolio: &MultiAssetPortfolio) -> Vec<UnifiedOrder> {
        let current_alloc = portfolio.get_allocation();
        let trades = vec![];
        
        for (asset_class, target_pct) in &self.target_allocations {
            let current_pct = current_alloc.get(asset_class).copied().unwrap_or(Decimal::ZERO);
            let diff = target_pct - current_pct;
            
            if diff.abs() > Decimal::from(5) { // 5% threshold
                // Would calculate actual trade here
                tracing::info!(
                    "Rebalance needed: {:?} current {}% target {}%",
                    asset_class, current_pct, target_pct
                );
            }
        }
        
        trades
    }
}
