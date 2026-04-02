//! Smart Order Router
//!
//! Compares prices across brokers and routes to best execution.
//! Routes based on symbol pattern:
//! - Crypto (ends with USDT/BTC) → Binance
//! - Forex (contains _ like EUR_USD) → OANDA
//! - Stocks (3-4 letter symbols like AAPL) → IBKR
//! - Cross-listed → compare prices, pick lowest spread

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::OrderSide;

/// A price quote from a specific broker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerQuote {
    /// Broker identifier (binance / ibkr / oanda).
    pub broker: String,
    /// Mid-market price.
    pub price: f64,
    /// Bid-ask spread (absolute).
    pub spread: f64,
    /// Whether the broker can currently accept orders for this symbol.
    pub available: bool,
}

/// The result of routing an order through the smart router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Unique identifier for the routing decision (audit trail).
    pub id: Uuid,
    /// The symbol that was routed.
    pub symbol: String,
    /// The order side.
    pub side: OrderSide,
    /// Requested quantity.
    pub quantity: f64,
    /// The broker selected for execution.
    pub chosen_broker: String,
    /// The indicative price at the chosen broker.
    pub price: f64,
    /// Human-readable reason for the routing choice.
    pub reason: String,
    /// All quotes considered during routing.
    pub alternatives: Vec<BrokerQuote>,
    /// Percentage savings vs. the worst available quote.
    pub savings_pct: f64,
    /// Timestamp of the routing decision.
    pub routed_at: DateTime<Utc>,
}

/// Asset class inferred from symbol pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetClass {
    Crypto,
    Forex,
    Stock,
}

/// Smart Order Router.
///
/// Holds a set of known broker names and routes orders to the best
/// execution venue based on the symbol's asset class.  In this
/// implementation the router uses deterministic symbol-pattern rules
/// and simulated quotes (no live price fetching).
#[derive(Debug, Clone)]
pub struct SmartRouter {
    /// Available broker identifiers.
    brokers: Vec<String>,
}

impl SmartRouter {
    /// Create a new router with the given broker identifiers.
    pub fn new(brokers: Vec<String>) -> Self {
        Self { brokers }
    }

    /// Create a router pre-configured with the three default brokers.
    pub fn default_brokers() -> Self {
        Self {
            brokers: vec![
                "binance".to_string(),
                "ibkr".to_string(),
                "oanda".to_string(),
            ],
        }
    }

    // ----------------------------------------------------------------
    // Classification
    // ----------------------------------------------------------------

    /// Classify a symbol into an asset class.
    pub fn classify_symbol(symbol: &str) -> AssetClass {
        let upper = symbol.to_uppercase();

        // Crypto: ends with USDT or BTC (e.g. BTCUSDT, ETHBTC)
        if upper.ends_with("USDT") || upper.ends_with("BTC") {
            return AssetClass::Crypto;
        }

        // Forex: contains underscore (e.g. EUR_USD, GBP_JPY)
        if symbol.contains('_') {
            return AssetClass::Forex;
        }

        // Default: stock
        AssetClass::Stock
    }

    // ----------------------------------------------------------------
    // Quote generation (simulated)
    // ----------------------------------------------------------------

    /// Generate simulated broker quotes for the symbol.
    ///
    /// In production this would query each broker's market-data API.
    /// Here we return deterministic values that depend only on the
    /// asset class so that routing logic can be exercised.
    fn generate_quotes(&self, symbol: &str) -> Vec<BrokerQuote> {
        let asset_class = Self::classify_symbol(symbol);

        self.brokers
            .iter()
            .map(|broker| {
                let (price, spread, available) = match (asset_class, broker.as_str()) {
                    // Crypto — Binance has tightest spread; IBKR wider spread + same price
                    (AssetClass::Crypto, "binance") => (100.0, 0.02, true),
                    (AssetClass::Crypto, "ibkr") => (100.0, 0.20, true),
                    (AssetClass::Crypto, "oanda") => (0.0, 0.0, false),

                    // Forex
                    (AssetClass::Forex, "oanda") => (1.1000, 0.0002, true),
                    (AssetClass::Forex, "ibkr") => (1.1001, 0.0005, true),
                    (AssetClass::Forex, "binance") => (0.0, 0.0, false),

                    // Stock
                    (AssetClass::Stock, "ibkr") => (150.0, 0.02, true),
                    (AssetClass::Stock, "binance") => (0.0, 0.0, false),
                    (AssetClass::Stock, "oanda") => (0.0, 0.0, false),

                    // Unknown broker
                    _ => (0.0, 0.0, false),
                };

                BrokerQuote {
                    broker: broker.clone(),
                    price,
                    spread,
                    available,
                }
            })
            .collect()
    }

    // ----------------------------------------------------------------
    // Routing
    // ----------------------------------------------------------------

    /// Route an order to the best execution venue.
    ///
    /// Returns a `RoutingDecision` describing which broker was chosen,
    /// why, and what alternatives were considered.
    pub fn route_order(&self, symbol: &str, side: OrderSide, quantity: f64) -> RoutingDecision {
        let quotes = self.generate_quotes(symbol);
        let asset_class = Self::classify_symbol(symbol);

        // Filter to available quotes
        let available: Vec<&BrokerQuote> = quotes.iter().filter(|q| q.available).collect();

        // Pick the best quote: lowest effective cost = price + spread/2 for buy,
        // highest effective price = price - spread/2 for sell.
        let best = available.iter().min_by(|a, b| {
            let cost_a = Self::effective_cost(a, side);
            let cost_b = Self::effective_cost(b, side);
            cost_a
                .partial_cmp(&cost_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let worst = available.iter().max_by(|a, b| {
            let cost_a = Self::effective_cost(a, side);
            let cost_b = Self::effective_cost(b, side);
            cost_a
                .partial_cmp(&cost_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let (chosen_broker, price, reason, savings_pct) = match (best, worst) {
            (Some(b), Some(w)) => {
                let best_cost = Self::effective_cost(b, side);
                let worst_cost = Self::effective_cost(w, side);

                let savings = if worst_cost.abs() > f64::EPSILON {
                    ((worst_cost - best_cost) / worst_cost * 100.0).abs()
                } else {
                    0.0
                };

                let reason = format!(
                    "{} routed to {} — asset_class={:?}, spread={:.6}, best effective cost={:.6}",
                    symbol, b.broker, asset_class, b.spread, best_cost,
                );

                (b.broker.clone(), b.price, reason, savings)
            }
            _ => {
                let reason = format!(
                    "No available broker for {} (asset_class={:?})",
                    symbol, asset_class,
                );
                ("none".to_string(), 0.0, reason, 0.0)
            }
        };

        tracing::info!(
            symbol = %symbol,
            chosen_broker = %chosen_broker,
            price = %price,
            savings_pct = %savings_pct,
            "Smart router decision"
        );

        RoutingDecision {
            id: Uuid::new_v4(),
            symbol: symbol.to_string(),
            side,
            quantity,
            chosen_broker,
            price,
            reason,
            alternatives: quotes,
            savings_pct,
            routed_at: Utc::now(),
        }
    }

    /// Effective cost for a quote (lower is better for the buyer).
    ///
    /// Buy  → price + spread / 2  (you pay the ask)
    /// Sell → -(price - spread / 2) negated so "lower is better" still holds
    fn effective_cost(quote: &BrokerQuote, side: OrderSide) -> f64 {
        match side {
            OrderSide::Buy => quote.price + quote.spread / 2.0,
            OrderSide::Sell => -(quote.price - quote.spread / 2.0),
        }
    }
}

// ====================================================================
// Tests
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn router() -> SmartRouter {
        SmartRouter::default_brokers()
    }

    #[test]
    fn test_crypto_routes_to_binance() {
        let r = router();
        let decision = r.route_order("BTCUSDT", OrderSide::Buy, 0.5);
        assert_eq!(decision.chosen_broker, "binance");
        assert_eq!(decision.symbol, "BTCUSDT");
        assert!(decision.price > 0.0);
        assert!(decision.savings_pct >= 0.0);
    }

    #[test]
    fn test_forex_routes_to_oanda() {
        let r = router();
        let decision = r.route_order("EUR_USD", OrderSide::Buy, 10_000.0);
        assert_eq!(decision.chosen_broker, "oanda");
        assert_eq!(decision.symbol, "EUR_USD");
        assert!(decision.price > 0.0);
    }

    #[test]
    fn test_stock_routes_to_ibkr() {
        let r = router();
        let decision = r.route_order("AAPL", OrderSide::Sell, 100.0);
        assert_eq!(decision.chosen_broker, "ibkr");
        assert_eq!(decision.symbol, "AAPL");
        assert!(decision.price > 0.0);
    }

    #[test]
    fn test_routing_decision_serialization() {
        let r = router();
        let decision = r.route_order("ETHBTC", OrderSide::Buy, 2.0);
        let json = serde_json::to_string(&decision).expect("serialize");
        let restored: RoutingDecision = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.chosen_broker, decision.chosen_broker);
        assert_eq!(restored.symbol, decision.symbol);
        assert!((restored.quantity - decision.quantity).abs() < f64::EPSILON);
        assert!((restored.savings_pct - decision.savings_pct).abs() < 1e-10);
        assert_eq!(restored.alternatives.len(), decision.alternatives.len());
    }

    #[test]
    fn test_classify_crypto() {
        assert_eq!(SmartRouter::classify_symbol("BTCUSDT"), AssetClass::Crypto);
        assert_eq!(SmartRouter::classify_symbol("ETHBTC"), AssetClass::Crypto);
    }

    #[test]
    fn test_classify_forex() {
        assert_eq!(SmartRouter::classify_symbol("EUR_USD"), AssetClass::Forex);
        assert_eq!(SmartRouter::classify_symbol("GBP_JPY"), AssetClass::Forex);
    }

    #[test]
    fn test_classify_stock() {
        assert_eq!(SmartRouter::classify_symbol("AAPL"), AssetClass::Stock);
        assert_eq!(SmartRouter::classify_symbol("MSFT"), AssetClass::Stock);
    }

    #[test]
    fn test_sell_side_routing() {
        let r = router();
        let decision = r.route_order("BTCUSDT", OrderSide::Sell, 1.0);
        // Binance still best for crypto sells (tightest spread)
        assert_eq!(decision.chosen_broker, "binance");
    }

    #[test]
    fn test_no_broker_available() {
        // Router with only OANDA — cannot trade stocks
        let r = SmartRouter::new(vec!["oanda".to_string()]);
        let decision = r.route_order("AAPL", OrderSide::Buy, 50.0);
        assert_eq!(decision.chosen_broker, "none");
        assert_eq!(decision.price, 0.0);
    }
}
