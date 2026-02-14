//! Prime Broker Selector
//!
//! Routes orders to optimal prime broker based on multiple factors

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;

use super::{PrimeBrokerError, Result};

/// Prime broker trait - institutional broker interface
#[async_trait]
pub trait PrimeBroker: Send + Sync + std::fmt::Debug {
    /// Get broker identifier
    fn id(&self) -> PrimeBrokerId;
    
    /// Get broker name
    fn name(&self) -> &str;
    
    /// Connect to broker
    async fn connect(&mut self) -> Result<()>;
    
    /// Check if connected
    fn is_connected(&self) -> bool;
    
    /// Get financing rate for symbol
    fn get_financing_rate(&self, symbol: &str, quantity: Decimal, long_term: bool) -> Decimal;
    
    /// Get commission rate
    fn get_commission_rate(&self, symbol: &str, quantity: Decimal) -> Decimal;
    
    /// Get available margin
    fn available_margin(&self) -> Decimal;
    
    /// Get total equity
    fn total_equity(&self) -> Decimal;
    
    /// Check if can execute order
    fn can_execute(&self, symbol: &str, quantity: Decimal) -> bool;
    
    /// Get execution score (higher is better)
    fn execution_score(&self, symbol: &str, quantity: Decimal) -> Decimal;
    
    /// Get market depth score
    fn market_depth_score(&self, symbol: &str) -> Decimal;
}

/// Prime broker identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimeBrokerId {
    GoldmanSachs,
    MorganStanley,
    JPMorgan,
    CreditSuisse,
    UBS,
    BankOfAmerica,
    Citigroup,
    Barclays,
    DeutscheBank,
    Nomura,
    InteractiveBrokers,
    BinanceInstitutional,
}

impl PrimeBrokerId {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrimeBrokerId::GoldmanSachs => "GS",
            PrimeBrokerId::MorganStanley => "MS",
            PrimeBrokerId::JPMorgan => "JPM",
            PrimeBrokerId::CreditSuisse => "CS",
            PrimeBrokerId::UBS => "UBS",
            PrimeBrokerId::BankOfAmerica => "BAC",
            PrimeBrokerId::Citigroup => "C",
            PrimeBrokerId::Barclays => "BARC",
            PrimeBrokerId::DeutscheBank => "DB",
            PrimeBrokerId::Nomura => "NMR",
            PrimeBrokerId::InteractiveBrokers => "IBKR",
            PrimeBrokerId::BinanceInstitutional => "BINANCE",
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            PrimeBrokerId::GoldmanSachs => "Goldman Sachs",
            PrimeBrokerId::MorganStanley => "Morgan Stanley",
            PrimeBrokerId::JPMorgan => "JP Morgan",
            PrimeBrokerId::CreditSuisse => "Credit Suisse",
            PrimeBrokerId::UBS => "UBS",
            PrimeBrokerId::BankOfAmerica => "Bank of America",
            PrimeBrokerId::Citigroup => "Citigroup",
            PrimeBrokerId::Barclays => "Barclays",
            PrimeBrokerId::DeutscheBank => "Deutsche Bank",
            PrimeBrokerId::Nomura => "Nomura",
            PrimeBrokerId::InteractiveBrokers => "Interactive Brokers",
            PrimeBrokerId::BinanceInstitutional => "Binance Institutional",
        }
    }
}

/// Routing decision
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub broker_id: PrimeBrokerId,
    pub score: Decimal,
    pub expected_cost: Decimal,
    pub expected_fill_time_ms: u32,
    pub confidence: f64,
}

/// Broker selector with scoring algorithm
#[derive(Debug)]
pub struct BrokerSelector {
    broker_ids: Vec<PrimeBrokerId>,
    weights: RoutingWeights,
    history: Vec<RoutingHistoryEntry>,
}

/// Scoring weights
#[derive(Debug, Clone)]
pub struct RoutingWeights {
    pub cost_weight: Decimal,          // Commission + financing
    pub execution_quality_weight: Decimal, // Fill rate, slippage
    pub liquidity_weight: Decimal,     // Market depth
    pub relationship_weight: Decimal,  // Existing positions, soft dollars
}

impl Default for RoutingWeights {
    fn default() -> Self {
        Self {
            cost_weight: Decimal::from(4),
            execution_quality_weight: Decimal::from(3),
            liquidity_weight: Decimal::from(2),
            relationship_weight: Decimal::from(1),
        }
    }
}

/// Routing history entry
#[derive(Debug, Clone)]
struct RoutingHistoryEntry {
    timestamp: DateTime<Utc>,
    symbol: String,
    broker_id: PrimeBrokerId,
    quantity: Decimal,
    filled: bool,
    slippage_bps: Decimal,
}

impl BrokerSelector {
    /// Create new selector
    pub fn new() -> Self {
        Self {
            broker_ids: Vec::new(),
            weights: RoutingWeights::default(),
            history: Vec::new(),
        }
    }

    /// Add broker to selection pool
    pub fn add_broker(&mut self, id: PrimeBrokerId) {
        if !self.broker_ids.contains(&id) {
            self.broker_ids.push(id);
        }
    }

    /// Select best broker for order
    pub fn select_broker(
        &self,
        symbol: &str,
        quantity: Decimal,
        brokers: &HashMap<PrimeBrokerId, Box<dyn PrimeBroker>>,
    ) -> Option<RoutingDecision> {
        let mut best_decision: Option<RoutingDecision> = None;
        let mut best_score: Option<Decimal> = None;

        for (broker_id, broker) in brokers {
            // Skip if can't execute
            if !broker.can_execute(symbol, quantity) {
                continue;
            }

            // Calculate score
            let score = self.calculate_score(broker.as_ref(), symbol, quantity);
            
            if best_score.map_or(true, |b| score > b) {
                best_score = Some(score);
                
                // Calculate expected costs
                let commission = broker.get_commission_rate(symbol, quantity);
                let financing = broker.get_financing_rate(symbol, quantity, false);
                let expected_cost = commission + financing;

                best_decision = Some(RoutingDecision {
                    broker_id: *broker_id,
                    score,
                    expected_cost,
                    expected_fill_time_ms: self.estimate_fill_time(broker.as_ref(), quantity),
                    confidence: 0.85, // Would be based on historical data
                });
            }
        }

        best_decision
    }

    /// Calculate routing score for broker
    fn calculate_score(
        &self,
        broker: &dyn PrimeBroker,
        symbol: &str,
        quantity: Decimal,
    ) -> Decimal {
        // Cost score (lower cost = higher score)
        let commission = broker.get_commission_rate(symbol, quantity);
        let financing = broker.get_financing_rate(symbol, quantity, false);
        let cost_score = Decimal::from(100) - (commission + financing) * Decimal::from(100);

        // Execution quality score
        let execution_score = broker.execution_score(symbol, quantity);

        // Liquidity score
        let liquidity_score = broker.market_depth_score(symbol);

        // Relationship score (based on existing positions)
        let relationship_score = if broker.total_equity() > Decimal::ZERO {
            broker.available_margin() / broker.total_equity() * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Weighted total
        let total = cost_score * self.weights.cost_weight
            + execution_score * self.weights.execution_quality_weight
            + liquidity_score * self.weights.liquidity_weight
            + relationship_score * self.weights.relationship_weight;

        total / (self.weights.cost_weight 
            + self.weights.execution_quality_weight 
            + self.weights.liquidity_weight 
            + self.weights.relationship_weight)
    }

    /// Estimate fill time in milliseconds
    fn estimate_fill_time(&self, broker: &dyn PrimeBroker, quantity: Decimal) -> u32 {
        // Simplified estimation based on market depth
        let depth_score = broker.market_depth_score(broker.name());
        
        if depth_score > Decimal::from(80) {
            50 // Fast fill
        } else if depth_score > Decimal::from(50) {
            200 // Medium
        } else {
            1000 // Slow
        }
    }

    /// Update weights
    pub fn set_weights(&mut self, weights: RoutingWeights) {
        self.weights = weights;
    }

    /// Log routing result
    pub fn log_routing(&mut self, symbol: &str, broker_id: PrimeBrokerId, quantity: Decimal, filled: bool, slippage_bps: Decimal) {
        self.history.push(RoutingHistoryEntry {
            timestamp: Utc::now(),
            symbol: symbol.to_string(),
            broker_id,
            quantity,
            filled,
            slippage_bps,
        });

        // Keep only last 1000 entries
        if self.history.len() > 1000 {
            self.history.remove(0);
        }
    }

    /// Get broker performance stats
    pub fn get_broker_stats(&self, broker_id: PrimeBrokerId) -> BrokerPerformanceStats {
        let broker_history: Vec<_> = self.history.iter()
            .filter(|h| h.broker_id == broker_id)
            .collect();

        let total = broker_history.len();
        let filled = broker_history.iter().filter(|h| h.filled).count();
        let avg_slippage = if total > 0 {
            broker_history.iter().map(|h| h.slippage_bps).sum::<Decimal>() / Decimal::from(total)
        } else {
            Decimal::ZERO
        };

        BrokerPerformanceStats {
            broker_id,
            total_orders: total,
            fill_rate: if total > 0 { Decimal::from(filled) / Decimal::from(total) * Decimal::from(100) } else { Decimal::ZERO },
            avg_slippage_bps: avg_slippage,
        }
    }
}

impl Default for BrokerSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Broker performance statistics
#[derive(Debug, Clone)]
pub struct BrokerPerformanceStats {
    pub broker_id: PrimeBrokerId,
    pub total_orders: usize,
    pub fill_rate: Decimal,
    pub avg_slippage_bps: Decimal,
}

/// Generic prime broker implementation
#[derive(Debug)]
pub struct GenericPrimeBroker {
    id: PrimeBrokerId,
    config: BrokerConfig,
    connected: bool,
    equity: Decimal,
    margin_used: Decimal,
}

#[derive(Debug, Clone)]
struct BrokerConfig {
    base_commission_rate: Decimal,
    base_financing_rate: Decimal,
    execution_quality_score: Decimal,
    market_depth_score: Decimal,
}

impl GenericPrimeBroker {
    pub fn new(id: PrimeBrokerId) -> Self {
        let config = BrokerConfig {
            base_commission_rate: Decimal::try_from(0.0005).unwrap(), // 5 bps
            base_financing_rate: Decimal::try_from(0.015).unwrap(),   // 1.5% annual
            execution_quality_score: Decimal::from(85),
            market_depth_score: Decimal::from(75),
        };

        Self {
            id,
            config,
            connected: false,
            equity: Decimal::from(10_000_000), // $10M default
            margin_used: Decimal::ZERO,
        }
    }

    pub fn with_equity(mut self, equity: Decimal) -> Self {
        self.equity = equity;
        self
    }
}

#[async_trait]
impl PrimeBroker for GenericPrimeBroker {
    fn id(&self) -> PrimeBrokerId {
        self.id
    }

    fn name(&self) -> &str {
        self.id.name()
    }

    async fn connect(&mut self) -> Result<()> {
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        self.connected = true;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn get_financing_rate(&self, _symbol: &str, _quantity: Decimal, long_term: bool) -> Decimal {
        if long_term {
            self.config.base_financing_rate * Decimal::try_from(0.9).unwrap() // 10% discount
        } else {
            self.config.base_financing_rate
        }
    }

    fn get_commission_rate(&self, _symbol: &str, quantity: Decimal) -> Decimal {
        // Tiered commission
        if quantity > Decimal::from(100_000) {
            self.config.base_commission_rate * Decimal::try_from(0.5).unwrap()
        } else if quantity > Decimal::from(10_000) {
            self.config.base_commission_rate * Decimal::try_from(0.8).unwrap()
        } else {
            self.config.base_commission_rate
        }
    }

    fn available_margin(&self) -> Decimal {
        self.equity - self.margin_used
    }

    fn total_equity(&self) -> Decimal {
        self.equity
    }

    fn can_execute(&self, _symbol: &str, quantity: Decimal) -> bool {
        let required_margin = quantity * Decimal::try_from(0.25).unwrap(); // 25% margin
        self.available_margin() >= required_margin
    }

    fn execution_score(&self, _symbol: &str, _quantity: Decimal) -> Decimal {
        self.config.execution_quality_score
    }

    fn market_depth_score(&self, _symbol: &str) -> Decimal {
        self.config.market_depth_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broker_id() {
        assert_eq!(PrimeBrokerId::GoldmanSachs.as_str(), "GS");
        assert_eq!(PrimeBrokerId::GoldmanSachs.name(), "Goldman Sachs");
    }

    #[test]
    fn test_selector_creation() {
        let selector = BrokerSelector::new();
        assert!(selector.broker_ids.is_empty());
    }

    #[test]
    fn test_add_broker() {
        let mut selector = BrokerSelector::new();
        selector.add_broker(PrimeBrokerId::GoldmanSachs);
        selector.add_broker(PrimeBrokerId::GoldmanSachs); // Duplicate
        
        assert_eq!(selector.broker_ids.len(), 1);
    }

    #[tokio::test]
    async fn test_generic_broker() {
        let mut broker = GenericPrimeBroker::new(PrimeBrokerId::GoldmanSachs);
        
        assert!(!broker.is_connected());
        broker.connect().await.unwrap();
        assert!(broker.is_connected());
        
        assert_eq!(broker.total_equity(), Decimal::from(10_000_000));
        assert!(broker.can_execute("AAPL", Decimal::from(100)));
    }

    #[test]
    fn test_tiered_commission() {
        let broker = GenericPrimeBroker::new(PrimeBrokerId::GoldmanSachs);
        
        let small = broker.get_commission_rate("AAPL", Decimal::from(1000));
        let large = broker.get_commission_rate("AAPL", Decimal::from(200_000));
        
        assert!(large < small); // Large orders get discount
    }
}
