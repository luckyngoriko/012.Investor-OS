//! Venue analysis and comparison

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Trading venue (exchange, dark pool, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Venue {
    InteractiveBrokers,
    Binance,
    Coinbase,
    Kraken,
    DarkPool(String),
    Internal,
}

impl Venue {
    pub fn name(&self) -> String {
        match self {
            Venue::InteractiveBrokers => "IBKR".to_string(),
            Venue::Binance => "Binance".to_string(),
            Venue::Coinbase => "Coinbase".to_string(),
            Venue::Kraken => "Kraken".to_string(),
            Venue::DarkPool(name) => format!("DarkPool:{}", name),
            Venue::Internal => "Internal".to_string(),
        }
    }
    
    pub fn venue_type(&self) -> VenueType {
        match self {
            Venue::InteractiveBrokers => VenueType::Exchange,
            Venue::Binance | Venue::Coinbase | Venue::Kraken => VenueType::CryptoExchange,
            Venue::DarkPool(_) => VenueType::DarkPool,
            Venue::Internal => VenueType::Internal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VenueType {
    Exchange,
    CryptoExchange,
    DarkPool,
    Internal,
}

/// Market data snapshot for a venue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueQuote {
    pub venue: Venue,
    pub symbol: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
    pub timestamp: DateTime<Utc>,
    pub latency_ms: u64,
}

impl VenueQuote {
    /// Calculate mid price
    pub fn mid(&self) -> Decimal {
        (self.bid + self.ask) / Decimal::from(2)
    }
    
    /// Calculate spread
    pub fn spread(&self) -> Decimal {
        self.ask - self.bid
    }
    
    /// Calculate spread as percentage
    pub fn spread_pct(&self) -> Decimal {
        if self.mid().is_zero() {
            return Decimal::ZERO;
        }
        (self.spread() / self.mid()) * Decimal::from(100)
    }
    
    /// Get effective price for buying (ask + market impact)
    pub fn effective_buy_price(&self, size: Decimal) -> Option<Decimal> {
        if size > self.ask_size {
            return None; // Insufficient liquidity
        }
        Some(self.ask)
    }
    
    /// Get effective price for selling (bid - market impact)
    pub fn effective_sell_price(&self, size: Decimal) -> Option<Decimal> {
        if size > self.bid_size {
            return None; // Insufficient liquidity
        }
        Some(self.bid)
    }
}

/// Fee structure for a venue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeStructure {
    pub maker_fee: Decimal,      // Rebate usually negative
    pub taker_fee: Decimal,      // Positive fee
    pub min_fee: Decimal,        // Minimum fee per trade
    pub max_fee: Option<Decimal>, // Maximum fee per trade
    pub volume_discounts: Vec<(Decimal, Decimal)>, // (volume_threshold, discount_rate)
}

impl Default for FeeStructure {
    fn default() -> Self {
        Self {
            maker_fee: Decimal::try_from(-0.0002).unwrap(), // -0.02% rebate
            taker_fee: Decimal::try_from(0.001).unwrap(),   // 0.1% fee
            min_fee: Decimal::ZERO,
            max_fee: None,
            volume_discounts: Vec::new(),
        }
    }
}

impl FeeStructure {
    /// Calculate fee for a trade
    pub fn calculate_fee(&self, notional: Decimal, is_maker: bool, _monthly_volume: Decimal) -> Decimal {
        let base_rate = if is_maker { self.maker_fee } else { self.taker_fee };
        let fee = notional * base_rate;
        
        // Apply min/max constraints
        // For rebates (negative fees), don't apply min_fee
        let fee = if fee < Decimal::ZERO {
            fee // Keep rebate as is
        } else {
            fee.max(self.min_fee)
        };
        
        if let Some(max) = self.max_fee {
            fee.min(max)
        } else {
            fee
        }
    }
}

/// Venue score for ranking
#[derive(Debug, Clone)]
pub struct VenueScore {
    pub venue: Venue,
    pub total_cost: Decimal,
    pub price_score: Decimal,
    pub liquidity_score: Decimal,
    pub latency_score: Decimal,
    pub reliability_score: Decimal,
    pub composite_score: Decimal,
}

/// Venue analyzer for comparing trading venues
#[derive(Debug)]
pub struct VenueAnalyzer {
    quotes: HashMap<(Venue, String), VenueQuote>,
    fees: HashMap<Venue, FeeStructure>,
    reliability_scores: HashMap<Venue, Decimal>,
}

impl VenueAnalyzer {
    pub fn new() -> Self {
        let mut fees = HashMap::new();
        fees.insert(Venue::InteractiveBrokers, FeeStructure {
            maker_fee: Decimal::ZERO,
            taker_fee: Decimal::try_from(0.005).unwrap(), // $0.005/share
            min_fee: Decimal::ONE,
            max_fee: Some(Decimal::from(1)),
            volume_discounts: vec![],
        });
        
        fees.insert(Venue::Binance, FeeStructure {
            maker_fee: Decimal::try_from(-0.0002).unwrap(),
            taker_fee: Decimal::try_from(0.001).unwrap(),
            min_fee: Decimal::ZERO,
            max_fee: None,
            volume_discounts: vec![],
        });
        
        Self {
            quotes: HashMap::new(),
            fees,
            reliability_scores: HashMap::new(),
        }
    }
    
    /// Update quote for a venue
    pub fn update_quote(&mut self, quote: VenueQuote) {
        let key = (quote.venue.clone(), quote.symbol.clone());
        self.quotes.insert(key, quote);
    }
    
    /// Get best quote for symbol across all venues
    pub fn get_best_quote(&self, symbol: &str, side: crate::execution::order::OrderSide) -> Option<&VenueQuote> {
        let quotes: Vec<_> = self.quotes.iter()
            .filter(|((_, sym), _)| sym == symbol)
            .map(|(_, quote)| quote)
            .collect();
        
        match side {
            crate::execution::order::OrderSide::Buy => {
                quotes.iter().min_by(|a, b| a.ask.cmp(&b.ask)).copied()
            }
            crate::execution::order::OrderSide::Sell => {
                quotes.iter().max_by(|a, b| a.bid.cmp(&b.bid)).copied()
            }
        }
    }
    
    /// Score venues for a given order
    pub fn score_venues(
        &self,
        symbol: &str,
        size: Decimal,
        side: crate::execution::order::OrderSide,
    ) -> Vec<VenueScore> {
        let mut scores = Vec::new();
        
        for ((venue, sym), quote) in &self.quotes {
            if sym != symbol {
                continue;
            }
            
            // Check liquidity
            let available = match side {
                crate::execution::order::OrderSide::Buy => quote.ask_size,
                crate::execution::order::OrderSide::Sell => quote.bid_size,
            };
            
            if available < size {
                continue; // Skip venues with insufficient liquidity
            }
            
            // Calculate scores (0-100, higher is better)
            let price_score = self.calculate_price_score(quote, side);
            let liquidity_score = (available / size).min(Decimal::from(10)) * Decimal::from(10);
            let latency_score = Decimal::from(100u64.saturating_sub(quote.latency_ms));
            let reliability = *self.reliability_scores.get(venue).unwrap_or(&Decimal::from(90));
            
            // Calculate total cost
            let price = match side {
                crate::execution::order::OrderSide::Buy => quote.ask,
                crate::execution::order::OrderSide::Sell => quote.bid,
            };
            let notional = size * price;
            let fee = self.fees.get(venue)
                .map(|f| f.calculate_fee(notional, false, Decimal::ZERO))
                .unwrap_or(Decimal::ZERO);
            let total_cost = fee.abs(); // Cost is always positive
            
            // Composite score (lower cost + higher quality = better)
            let composite = price_score * Decimal::from(40)
                + liquidity_score * Decimal::from(30)
                + latency_score * Decimal::from(15)
                + reliability * Decimal::from(15);
            
            scores.push(VenueScore {
                venue: venue.clone(),
                total_cost,
                price_score,
                liquidity_score,
                latency_score,
                reliability_score: reliability,
                composite_score: composite / Decimal::from(100),
            });
        }
        
        // Sort by composite score descending
        scores.sort_by(|a, b| b.composite_score.cmp(&a.composite_score));
        scores
    }
    
    fn calculate_price_score(
        &self,
        quote: &VenueQuote,
        side: crate::execution::order::OrderSide,
    ) -> Decimal {
        // Calculate how good the price is vs best available
        let all_quotes: Vec<_> = self.quotes.values()
            .filter(|q| q.symbol == quote.symbol)
            .collect();
        
        if all_quotes.is_empty() {
            return Decimal::from(50); // Neutral
        }
        
        match side {
            crate::execution::order::OrderSide::Buy => {
                let best_ask = all_quotes.iter().map(|q| q.ask).min().unwrap_or(quote.ask);
                if best_ask.is_zero() {
                    return Decimal::from(50);
                }
                // Lower ask is better
                let ratio = best_ask / quote.ask;
                (ratio * Decimal::from(100)).min(Decimal::from(100))
            }
            crate::execution::order::OrderSide::Sell => {
                let best_bid = all_quotes.iter().map(|q| q.bid).max().unwrap_or(quote.bid);
                if quote.bid.is_zero() {
                    return Decimal::from(50);
                }
                // Higher bid is better
                let ratio = quote.bid / best_bid;
                (ratio * Decimal::from(100)).min(Decimal::from(100))
            }
        }
    }
}

impl Default for VenueAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::order::OrderSide;
    use chrono::Utc;
    
    fn create_test_quote(venue: Venue, bid: i64, ask: i64) -> VenueQuote {
        VenueQuote {
            venue,
            symbol: "BTC".to_string(),
            bid: Decimal::from(bid),
            ask: Decimal::from(ask),
            bid_size: Decimal::from(10),
            ask_size: Decimal::from(10),
            timestamp: Utc::now(),
            latency_ms: 50,
        }
    }
    
    #[test]
    fn test_venue_quote_mid_and_spread() {
        let quote = VenueQuote {
            venue: Venue::Binance,
            symbol: "BTC".to_string(),
            bid: Decimal::from(50000),
            ask: Decimal::from(50100),
            bid_size: Decimal::from(5),
            ask_size: Decimal::from(3),
            timestamp: Utc::now(),
            latency_ms: 20,
        };
        
        assert_eq!(quote.mid(), Decimal::from(50050));
        assert_eq!(quote.spread(), Decimal::from(100));
        // Spread % = 100 / 50050 * 100 = 0.1998%
        assert!(quote.spread_pct() < Decimal::ONE);
    }
    
    #[test]
    fn test_get_best_quote_buy() {
        let mut analyzer = VenueAnalyzer::new();
        
        analyzer.update_quote(create_test_quote(Venue::Binance, 50000, 50100));
        analyzer.update_quote(create_test_quote(Venue::Coinbase, 50010, 50090));
        analyzer.update_quote(create_test_quote(Venue::Kraken, 50020, 50120));
        
        let best = analyzer.get_best_quote("BTC", OrderSide::Buy);
        assert!(best.is_some());
        // Coinbase has lowest ask at 50090
        assert_eq!(best.unwrap().venue, Venue::Coinbase);
        assert_eq!(best.unwrap().ask, Decimal::from(50090));
    }
    
    #[test]
    fn test_get_best_quote_sell() {
        let mut analyzer = VenueAnalyzer::new();
        
        analyzer.update_quote(create_test_quote(Venue::Binance, 50100, 50200));
        analyzer.update_quote(create_test_quote(Venue::Coinbase, 50150, 50250));
        analyzer.update_quote(create_test_quote(Venue::Kraken, 50080, 50180));
        
        let best = analyzer.get_best_quote("BTC", OrderSide::Sell);
        assert!(best.is_some());
        // Coinbase has highest bid at 50150
        assert_eq!(best.unwrap().venue, Venue::Coinbase);
        assert_eq!(best.unwrap().bid, Decimal::from(50150));
    }
    
    #[test]
    fn test_insufficient_liquidity() {
        let mut quote = create_test_quote(Venue::Binance, 50000, 50100);
        quote.ask_size = Decimal::from(1); // Only 1 BTC available
        
        let effective = quote.effective_buy_price(Decimal::from(5));
        assert!(effective.is_none()); // Need 5, only 1 available
        
        let effective = quote.effective_buy_price(Decimal::from(1));
        assert!(effective.is_some()); // Exactly 1 available
    }
    
    #[test]
    fn test_fee_calculation() {
        let fees = FeeStructure::default();
        
        // Taker fee on $10k = $10
        let taker_fee = fees.calculate_fee(Decimal::from(10000), false, Decimal::ZERO);
        assert_eq!(taker_fee, Decimal::from(10)); // 0.1%
        
        // Maker rebate on $10k = -$2
        let maker_fee = fees.calculate_fee(Decimal::from(10000), true, Decimal::ZERO);
        assert_eq!(maker_fee, Decimal::from(-2)); // -0.02%
    }
    
    #[test]
    fn test_venue_scoring() {
        let mut analyzer = VenueAnalyzer::new();
        
        analyzer.update_quote(create_test_quote(Venue::Binance, 50000, 50100));
        analyzer.update_quote(create_test_quote(Venue::Coinbase, 50050, 50150));
        
        let scores = analyzer.score_venues("BTC", Decimal::from(5), OrderSide::Buy);
        
        assert_eq!(scores.len(), 2);
        // Binance should have better score (lower ask)
        assert!(scores[0].venue == Venue::Binance);
        assert!(scores[0].composite_score >= scores[1].composite_score);
    }
}
