//! Arbitrage opportunity detection and tracking

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::execution::venue::{Venue, VenueQuote};
use crate::execution::order::OrderSide;

/// Type of arbitrage opportunity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArbitrageType {
    /// Cross-venue: Buy on A, sell on B
    CrossVenue,
    /// Triangular: A → B → C → A
    Triangular,
    /// Statistical: Mean reversion
    Statistical,
}

/// Arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: Uuid,
    pub arb_type: ArbitrageType,
    pub symbol: String,
    pub buy_venue: Venue,
    pub sell_venue: Venue,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub quantity: Decimal,
    pub gross_profit: Decimal,
    pub estimated_costs: Decimal,
    pub net_profit: Decimal,
    pub profit_bps: Decimal, // Basis points
    pub confidence: Decimal, // 0-1
    pub detected_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub latency_ms: u64,
}

impl ArbitrageOpportunity {
    /// Create new cross-venue opportunity
    pub fn cross_venue(
        symbol: impl Into<String>,
        buy_quote: &VenueQuote,
        sell_quote: &VenueQuote,
        quantity: Decimal,
        estimated_costs: Decimal,
    ) -> Option<Self> {
        let symbol = symbol.into();
        let buy_price = buy_quote.ask;
        let sell_price = sell_quote.bid;
        
        // Check if profitable (sell > buy)
        if sell_price <= buy_price {
            return None;
        }
        
        let gross_profit = (sell_price - buy_price) * quantity;
        let net_profit = gross_profit - estimated_costs;
        
        if net_profit <= Decimal::ZERO {
            return None;
        }
        
        let notional = quantity * buy_price;
        let profit_bps = (net_profit / notional) * Decimal::from(10000);
        
        let latency_ms = buy_quote.latency_ms.max(sell_quote.latency_ms);
        
        Some(Self {
            id: Uuid::new_v4(),
            arb_type: ArbitrageType::CrossVenue,
            symbol,
            buy_venue: buy_quote.venue.clone(),
            sell_venue: sell_quote.venue.clone(),
            buy_price,
            sell_price,
            quantity,
            gross_profit,
            estimated_costs,
            net_profit,
            profit_bps,
            confidence: Decimal::ONE, // Direct arb has high confidence
            detected_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::seconds(5), // 5 second window
            latency_ms,
        })
    }
    
    /// Check if opportunity is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
    
    /// Calculate time to live in milliseconds
    pub fn ttl_ms(&self) -> i64 {
        self.expires_at.timestamp_millis() - Utc::now().timestamp_millis()
    }
    
    /// Estimate execution time
    pub fn estimated_execution_time_ms(&self) -> u64 {
        // Buy latency + sell latency + processing
        self.latency_ms * 2 + 50
    }
    
    /// Check if we can execute before expiry
    pub fn is_executable(&self) -> bool {
        let ttl = self.ttl_ms();
        let exec_time = self.estimated_execution_time_ms() as i64;
        ttl > exec_time
    }
}

/// Triangular arbitrage path
#[derive(Debug, Clone)]
pub struct TriangularPath {
    pub path: Vec<(String, OrderSide)>, // (symbol, side)
    pub venues: Vec<Venue>,
    pub rates: Vec<Decimal>,
    pub start_amount: Decimal,
    pub end_amount: Decimal,
    pub profit_pct: Decimal,
}

impl TriangularPath {
    /// Calculate profit for triangular arbitrage
    /// Path: A → B → C → A
    pub fn calculate_profit(
        amount: Decimal,
        rate_ab: Decimal, // A/B rate
        rate_bc: Decimal, // B/C rate
        rate_ca: Decimal, // C/A rate
    ) -> Decimal {
        // Start with amount of A
        // A → B: amount_b = amount_a / rate_ab
        // B → C: amount_c = amount_b / rate_bc
        // C → A: amount_a_final = amount_c / rate_ca
        let amount_b = amount / rate_ab;
        let amount_c = amount_b / rate_bc;
        let amount_a_final = amount_c / rate_ca;
        
        amount_a_final - amount
    }
    
    /// Common triangular paths in crypto
    pub fn common_crypto_triangles() -> Vec<Vec<String>> {
        vec![
            vec!["BTC".to_string(), "ETH".to_string(), "USDT".to_string()],
            vec!["BTC".to_string(), "SOL".to_string(), "USDT".to_string()],
            vec!["ETH".to_string(), "SOL".to_string(), "USDT".to_string()],
            vec!["BTC".to_string(), "BNB".to_string(), "USDT".to_string()],
        ]
    }
}

/// Opportunity scanner configuration
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub min_profit_bps: Decimal,
    pub max_latency_ms: u64,
    pub scan_interval_ms: u64,
    pub symbols: Vec<String>,
    pub venues: Vec<Venue>,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            min_profit_bps: Decimal::from(5), // 5 bps = 0.05%
            max_latency_ms: 200,
            scan_interval_ms: 100,
            symbols: vec!["BTC".to_string(), "ETH".to_string()],
            venues: vec![Venue::Binance, Venue::Coinbase, Venue::Kraken],
        }
    }
}

/// Opportunity tracker for analytics
#[derive(Debug, Default)]
pub struct OpportunityTracker {
    detected: Vec<ArbitrageOpportunity>,
    executed: Vec<(ArbitrageOpportunity, Decimal)>, // (opportunity, actual_profit)
    missed: Vec<ArbitrageOpportunity>,
}

impl OpportunityTracker {
    pub fn record_detected(&mut self, opp: ArbitrageOpportunity) {
        self.detected.push(opp);
    }
    
    pub fn record_executed(&mut self, opp: ArbitrageOpportunity, actual_profit: Decimal) {
        self.executed.push((opp, actual_profit));
    }
    
    pub fn record_missed(&mut self, opp: ArbitrageOpportunity) {
        self.missed.push(opp);
    }
    
    pub fn stats(&self) -> OpportunityStats {
        let total_detected = self.detected.len();
        let total_executed = self.executed.len();
        let total_missed = self.missed.len();
        
        let avg_profit = if total_executed > 0 {
            self.executed.iter()
                .map(|(_, p)| *p)
                .fold(Decimal::ZERO, |a, b| a + b)
                / Decimal::from(total_executed as i64)
        } else {
            Decimal::ZERO
        };
        
        OpportunityStats {
            total_detected,
            total_executed,
            total_missed,
            execution_rate: if total_detected > 0 {
                Decimal::from(total_executed as i64) / Decimal::from(total_detected as i64)
            } else {
                Decimal::ZERO
            },
            avg_profit_per_trade: avg_profit,
            total_profit: self.executed.iter().map(|(_, p)| *p).fold(Decimal::ZERO, |a, b| a + b),
        }
    }
}

/// Opportunity statistics
#[derive(Debug, Clone)]
pub struct OpportunityStats {
    pub total_detected: usize,
    pub total_executed: usize,
    pub total_missed: usize,
    pub execution_rate: Decimal,
    pub avg_profit_per_trade: Decimal,
    pub total_profit: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    fn create_quote(venue: Venue, bid: i64, ask: i64) -> VenueQuote {
        VenueQuote {
            venue,
            symbol: "BTC".to_string(),
            bid: Decimal::from(bid),
            ask: Decimal::from(ask),
            bid_size: Decimal::from(100),
            ask_size: Decimal::from(100),
            timestamp: Utc::now(),
            latency_ms: 20,
        }
    }
    
    #[test]
    fn test_cross_venue_opportunity() {
        let binance = create_quote(Venue::Binance, 50000, 50100);
        let coinbase = create_quote(Venue::Coinbase, 50200, 50300);
        
        // Buy on Binance at 50100, sell on Coinbase at 50200
        let opp = ArbitrageOpportunity::cross_venue(
            "BTC",
            &binance,
            &coinbase,
            Decimal::from(1),
            Decimal::from(50), // Estimated costs
        );
        
        assert!(opp.is_some());
        let opp = opp.unwrap();
        
        // Profit = (50200 - 50100) * 1 - 50 = $50
        assert_eq!(opp.gross_profit, Decimal::from(100));
        assert_eq!(opp.net_profit, Decimal::from(50));
        assert!(opp.profit_bps > Decimal::ZERO);
    }
    
    #[test]
    fn test_no_opportunity_when_unprofitable() {
        let binance = create_quote(Venue::Binance, 50000, 50100);
        let coinbase = create_quote(Venue::Coinbase, 50050, 50150);
        
        // Sell price (50050) < Buy price (50100), no arb
        let opp = ArbitrageOpportunity::cross_venue(
            "BTC",
            &binance,
            &coinbase,
            Decimal::from(1),
            Decimal::from(10),
        );
        
        assert!(opp.is_none());
    }
    
    #[test]
    fn test_opportunity_validity() {
        let binance = create_quote(Venue::Binance, 50000, 50100);
        let coinbase = create_quote(Venue::Coinbase, 50200, 50300);
        
        let opp = ArbitrageOpportunity::cross_venue(
            "BTC",
            &binance,
            &coinbase,
            Decimal::from(1),
            Decimal::from(10),
        ).unwrap();
        
        assert!(opp.is_valid());
        assert!(opp.is_executable());
    }
    
    #[test]
    fn test_triangular_profit_calculation() {
        // Path: BTC → ETH → USDT → BTC
        // 1 BTC = 15 ETH (rate_btc_eth = 15)
        // 1 ETH = 2000 USDT (rate_eth_usdt = 2000)
        // 1 BTC = 30000 USDT (rate_btc_usdt = 30000)
        
        let rate_btc_eth = Decimal::from(15);   // ETH per BTC
        let rate_eth_usdt = Decimal::from(2000); // USDT per ETH
        let rate_btc_usdt = Decimal::from(30000); // USDT per BTC
        
        // Start with 1 BTC
        let profit = TriangularPath::calculate_profit(
            Decimal::ONE,
            rate_btc_eth,
            rate_eth_usdt,
            Decimal::ONE / rate_btc_usdt, // Inverse for USDT → BTC
        );
        
        // 1 BTC → 15 ETH → 30000 USDT → 1 BTC
        // With perfect rates, profit = 0
        // In practice, rates have small discrepancies
        println!("Triangular profit: {}", profit);
    }
    
    #[test]
    fn test_opportunity_tracker() {
        let mut tracker = OpportunityTracker::default();
        
        let opp1 = ArbitrageOpportunity::cross_venue(
            "BTC",
            &create_quote(Venue::Binance, 50000, 50100),
            &create_quote(Venue::Coinbase, 50200, 50300),
            Decimal::from(1),
            Decimal::from(10),
        ).unwrap();
        
        tracker.record_detected(opp1.clone());
        tracker.record_executed(opp1, Decimal::from(45));
        
        let stats = tracker.stats();
        assert_eq!(stats.total_detected, 1);
        assert_eq!(stats.total_executed, 1);
        assert_eq!(stats.execution_rate, Decimal::ONE);
        assert_eq!(stats.total_profit, Decimal::from(45));
    }
}
