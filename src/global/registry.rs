//! Exchange Registry
//!
//! Manages all connected exchanges and aggregates market data

use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::{
    exchanges::{Exchange, ExchangeId, ExchangeQuote, ExchangeStatus},
    GlobalError, Region, Result,
};

/// Registry of all exchanges
#[derive(Debug)]
pub struct ExchangeRegistry {
    exchanges: RwLock<HashMap<ExchangeId, Box<dyn Exchange>>>,
}

impl ExchangeRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            exchanges: RwLock::new(HashMap::new()),
        }
    }

    /// Register an exchange
    pub fn register(&mut self, exchange: Box<dyn Exchange>) {
        let id = exchange.id();
        info!("Registering exchange: {}", id);
        
        // Use blocking lock since we're in a non-async context
        if let Ok(mut exchanges) = self.exchanges.try_write() {
            exchanges.insert(id, exchange);
        }
    }

    /// Get exchange by ID
    pub fn get(&self, id: ExchangeId) -> Option<&dyn Exchange> {
        // This requires a more complex approach for async RwLock
        // For now, we'll use a simplified approach
        None // Placeholder - will be fixed with proper implementation
    }

    /// Get all exchanges
    pub fn all_exchanges(&self) -> Vec<&dyn Exchange> {
        // Simplified for now
        Vec::new()
    }

    /// Get active (connected) exchanges
    pub fn active_exchanges(&self) -> Vec<&dyn Exchange> {
        // Simplified for now
        Vec::new()
    }

    /// Get exchanges by region
    pub fn exchanges_by_region(&self, region: Region) -> Vec<&dyn Exchange> {
        // Simplified for now
        Vec::new()
    }

    /// Connect to all exchanges
    pub async fn connect_all(&mut self) -> Result<()> {
        let mut exchanges = self.exchanges.write().await;
        
        for (id, exchange) in exchanges.iter_mut() {
            info!("Connecting to exchange: {}", id);
            if let Err(e) = exchange.connect().await {
                error!("Failed to connect to {}: {:?}", id, e);
                return Err(GlobalError::Connection(format!("{}: {:?}", id, e)));
            }
        }

        info!("Connected to {} exchanges", exchanges.len());
        Ok(())
    }

    /// Disconnect from all exchanges
    pub async fn disconnect_all(&mut self) -> Result<()> {
        let mut exchanges = self.exchanges.write().await;
        
        for (id, exchange) in exchanges.iter_mut() {
            info!("Disconnecting from exchange: {}", id);
            if let Err(e) = exchange.disconnect().await {
                warn!("Error disconnecting from {}: {:?}", id, e);
            }
        }

        info!("Disconnected from all exchanges");
        Ok(())
    }

    /// Get best bid across all exchanges for symbol
    pub async fn get_best_bid(&self, symbol: &str) -> Option<(ExchangeId, ExchangeQuote)> {
        let exchanges = self.exchanges.read().await;
        let mut best_bid: Option<(ExchangeId, ExchangeQuote)> = None;

        for (id, exchange) in exchanges.iter() {
            if let Some(quote) = exchange.get_quote(symbol) {
                if best_bid.as_ref().map_or(true, |(_, b)| quote.bid > b.bid) {
                    best_bid = Some((*id, quote));
                }
            }
        }

        best_bid
    }

    /// Get best ask across all exchanges for symbol
    pub async fn get_best_ask(&self, symbol: &str) -> Option<(ExchangeId, ExchangeQuote)> {
        let exchanges = self.exchanges.read().await;
        let mut best_ask: Option<(ExchangeId, ExchangeQuote)> = None;

        for (id, exchange) in exchanges.iter() {
            if let Some(quote) = exchange.get_quote(symbol) {
                if best_ask.as_ref().map_or(true, |(_, a)| quote.ask < a.ask) {
                    best_ask = Some((*id, quote));
                }
            }
        }

        best_ask
    }

    /// Count of registered exchanges
    pub fn len(&self) -> usize {
        // Blocking read
        futures::executor::block_on(async {
            self.exchanges.read().await.len()
        })
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ExchangeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregated market data from multiple exchanges
#[derive(Debug, Clone)]
pub struct GlobalMarketData {
    pub symbol: String,
    pub bids: Vec<(ExchangeId, rust_decimal::Decimal, rust_decimal::Decimal)>, // (exchange, price, size)
    pub asks: Vec<(ExchangeId, rust_decimal::Decimal, rust_decimal::Decimal)>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl GlobalMarketData {
    /// Create empty market data
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            bids: Vec::new(),
            asks: Vec::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add a bid
    pub fn add_bid(&mut self, exchange: ExchangeId, price: rust_decimal::Decimal, size: rust_decimal::Decimal) {
        self.bids.push((exchange, price, size));
        // Sort by price descending (best bid first)
        self.bids.sort_by(|a, b| b.1.cmp(&a.1));
    }

    /// Add an ask
    pub fn add_ask(&mut self, exchange: ExchangeId, price: rust_decimal::Decimal, size: rust_decimal::Decimal) {
        self.asks.push((exchange, price, size));
        // Sort by price ascending (best ask first)
        self.asks.sort_by(|a, b| a.1.cmp(&b.1));
    }

    /// Get best bid
    pub fn best_bid(&self) -> Option<(ExchangeId, rust_decimal::Decimal, rust_decimal::Decimal)> {
        self.bids.first().copied()
    }

    /// Get best ask
    pub fn best_ask(&self) -> Option<(ExchangeId, rust_decimal::Decimal, rust_decimal::Decimal)> {
        self.asks.first().copied()
    }

    /// Get spread
    pub fn spread(&self) -> Option<rust_decimal::Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some((_, bid, _)), Some((_, ask, _))) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get spread in basis points
    pub fn spread_bps(&self) -> Option<rust_decimal::Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some((_, bid, _)), Some((_, ask, _))) if !bid.is_zero() => {
                Some(((ask - bid) / bid) * rust_decimal::Decimal::from(10000))
            }
            _ => None,
        }
    }

    /// Get VWAP (Volume Weighted Average Price) for bids up to target quantity
    pub fn vwap_bid(&self, target_qty: rust_decimal::Decimal) -> Option<rust_decimal::Decimal> {
        let mut remaining = target_qty;
        let mut total_value = rust_decimal::Decimal::ZERO;
        let mut total_qty = rust_decimal::Decimal::ZERO;

        for (_, price, size) in &self.bids {
            let take = remaining.min(*size);
            total_value += take * *price;
            total_qty += take;
            remaining -= take;

            if remaining.is_zero() {
                break;
            }
        }

        if total_qty.is_zero() {
            None
        } else {
            Some(total_value / total_qty)
        }
    }

    /// Get VWAP for asks up to target quantity
    pub fn vwap_ask(&self, target_qty: rust_decimal::Decimal) -> Option<rust_decimal::Decimal> {
        let mut remaining = target_qty;
        let mut total_value = rust_decimal::Decimal::ZERO;
        let mut total_qty = rust_decimal::Decimal::ZERO;

        for (_, price, size) in &self.asks {
            let take = remaining.min(*size);
            total_value += take * *price;
            total_qty += take;
            remaining -= take;

            if remaining.is_zero() {
                break;
            }
        }

        if total_qty.is_zero() {
            None
        } else {
            Some(total_value / total_qty)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_registry_creation() {
        let registry = ExchangeRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_global_market_data() {
        let mut data = GlobalMarketData::new("AAPL");
        
        data.add_bid(ExchangeId::NYSE, Decimal::from(150), Decimal::from(100));
        data.add_bid(ExchangeId::NASDAQ, Decimal::from(151), Decimal::from(50));
        data.add_ask(ExchangeId::NYSE, Decimal::from(152), Decimal::from(100));
        
        // Best bid should be NASDAQ at 151
        assert_eq!(data.best_bid().unwrap().1, Decimal::from(151));
        
        // Spread should be 1
        assert_eq!(data.spread().unwrap(), Decimal::from(1));
    }

    #[test]
    fn test_vwap_calculation() {
        let mut data = GlobalMarketData::new("BTC");
        
        // Add bids: 100 @ 50000, 50 @ 49900
        data.add_bid(ExchangeId::Binance, Decimal::from(50000), Decimal::from(100));
        data.add_bid(ExchangeId::Coinbase, Decimal::from(49900), Decimal::from(50));
        
        // VWAP for 120 quantity
        // Takes 100 @ 50000 = 5,000,000
        // Takes 20 @ 49900 = 998,000
        // Total = 5,998,000 / 120 = 49,983.33
        let vwap = data.vwap_bid(Decimal::from(120)).unwrap();
        assert!(vwap > Decimal::from(49900));
        assert!(vwap < Decimal::from(50000));
    }

    #[test]
    fn test_spread_bps() {
        let mut data = GlobalMarketData::new("AAPL");
        
        data.add_bid(ExchangeId::NYSE, Decimal::try_from(100.0).unwrap(), Decimal::from(100));
        data.add_ask(ExchangeId::NYSE, Decimal::try_from(100.10).unwrap(), Decimal::from(100));
        
        // Spread should be 10 bps (0.10%)
        let spread_bps = data.spread_bps().unwrap();
        assert_eq!(spread_bps, Decimal::from(10));
    }
}
