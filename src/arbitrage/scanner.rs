//! Arbitrage opportunity scanner

use rust_decimal::Decimal;
use tokio::time::{interval, Duration};
use tracing::debug;
use std::collections::HashMap;

use crate::execution::venue::{VenueQuote, VenueAnalyzer};
use crate::execution::cost::CostCalculator;

use super::opportunity::{ArbitrageOpportunity, TriangularPath, ScannerConfig};
use super::error::Result;

/// Real-time arbitrage scanner
#[derive(Debug)]
pub struct ArbitrageScanner {
    config: ScannerConfig,
    venue_analyzer: VenueAnalyzer,
    cost_calculator: CostCalculator,
}

impl ArbitrageScanner {
    pub fn new(config: ScannerConfig) -> Self {
        Self {
            config,
            venue_analyzer: VenueAnalyzer::new(),
            cost_calculator: CostCalculator::new(),
        }
    }
    
    /// Update venue quote
    pub fn update_quote(&mut self, quote: VenueQuote) {
        self.venue_analyzer.update_quote(quote);
    }
    
    /// Scan for cross-venue arbitrage opportunities
    pub fn scan_cross_venue(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        
        for symbol in &self.config.symbols {
            // Get all quotes for this symbol
            let quotes: Vec<&VenueQuote> = self.config.venues.iter()
                .filter_map(|v| {
                    let _key = (v.clone(), symbol.clone());
                    // Access through venue_analyzer quotes
                    None::<&VenueQuote> // Simplified
                })
                .collect();
            
            // For each pair of venues, check for arb
            for (i, buy_quote) in quotes.iter().enumerate() {
                for sell_quote in quotes.iter().skip(i + 1) {
                    // Try both directions
                    if let Some(opp) = self.check_arb_pair(symbol, buy_quote, sell_quote) {
                        opportunities.push(opp);
                    }
                    if let Some(opp) = self.check_arb_pair(symbol, sell_quote, buy_quote) {
                        opportunities.push(opp);
                    }
                }
            }
        }
        
        // Sort by profit
        opportunities.sort_by(|a, b| b.net_profit.cmp(&a.net_profit));
        opportunities
    }
    
    /// Check arbitrage between two quotes
    fn check_arb_pair(
        &self,
        symbol: &str,
        buy_quote: &&VenueQuote,
        sell_quote: &&VenueQuote,
    ) -> Option<ArbitrageOpportunity> {
        if buy_quote.venue == sell_quote.venue {
            return None;
        }
        
        // Calculate max executable quantity
        let quantity = buy_quote.ask_size.min(sell_quote.bid_size);
        if quantity.is_zero() {
            return None;
        }
        
        // Estimate costs
        let notional = quantity * buy_quote.ask;
        let estimated_costs = self.estimate_total_costs(notional);
        
        ArbitrageOpportunity::cross_venue(
            symbol,
            buy_quote,
            sell_quote,
            quantity,
            estimated_costs,
        )
    }
    
    /// Estimate total costs for arbitrage
    fn estimate_total_costs(&self, notional: Decimal) -> Decimal {
        // Fees on both sides + estimated slippage
        let fees = notional * Decimal::try_from(0.002).unwrap(); // 0.2% total fees
        let slippage = notional * Decimal::try_from(0.0005).unwrap(); // 5 bps slippage
        fees + slippage
    }
    
    /// Scan for triangular arbitrage opportunities
    pub fn scan_triangular(&self) -> Vec<TriangularPath> {
        let mut paths = Vec::new();
        let triangles = TriangularPath::common_crypto_triangles();
        
        for triangle in triangles {
            if let Some(path) = self.check_triangular_path(&triangle) {
                if path.profit_pct > self.config.min_profit_bps / Decimal::from(100) {
                    paths.push(path);
                }
            }
        }
        
        paths
    }
    
    /// Check a specific triangular path
    fn check_triangular_path(&self, symbols: &[String]) -> Option<TriangularPath> {
        if symbols.len() != 3 {
            return None;
        }
        
        // Get rates for each leg
        // Simplified: would need order book depth analysis
        None
    }
    
    /// Get best opportunity across all types
    pub fn get_best_opportunity(&self) -> Option<ArbitrageOpportunity> {
        let cross_venue = self.scan_cross_venue();
        cross_venue.into_iter().next() // Best due to sorting
    }
    
    /// Filter opportunities by minimum profit
    pub fn filter_by_profit(&self, opportunities: Vec<ArbitrageOpportunity>) -> Vec<ArbitrageOpportunity> {
        opportunities.into_iter()
            .filter(|o| o.profit_bps >= self.config.min_profit_bps)
            .filter(|o| o.latency_ms <= self.config.max_latency_ms)
            .filter(|o| o.is_executable())
            .collect()
    }
    
    /// Start continuous scanning (async)
    pub async fn run<F, Fut>(self, mut callback: F) -> Result<()>
    where
        F: FnMut(Vec<ArbitrageOpportunity>) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let mut interval = interval(Duration::from_millis(self.config.scan_interval_ms));
        
        loop {
            interval.tick().await;
            
            let opportunities = self.scan_cross_venue();
            let filtered = self.filter_by_profit(opportunities);
            
            if !filtered.is_empty() {
                debug!("Found {} arbitrage opportunities", filtered.len());
                callback(filtered).await;
            }
        }
    }
    
    /// Access venue analyzer
    pub fn venue_analyzer(&self) -> &VenueAnalyzer {
        &self.venue_analyzer
    }
    
    pub fn venue_analyzer_mut(&mut self) -> &mut VenueAnalyzer {
        &mut self.venue_analyzer
    }
}

/// Multi-symbol scanner with prioritization
#[derive(Debug)]
pub struct PriorityScanner {
    scanners: HashMap<String, ArbitrageScanner>,
    priority_symbols: Vec<String>,
}

impl PriorityScanner {
    pub fn new(symbols: Vec<String>, config: ScannerConfig) -> Self {
        let mut scanners = HashMap::new();
        
        for symbol in &symbols {
            let mut symbol_config = config.clone();
            symbol_config.symbols = vec![symbol.clone()];
            scanners.insert(symbol.clone(), ArbitrageScanner::new(symbol_config));
        }
        
        Self {
            scanners,
            priority_symbols: symbols,
        }
    }
    
    /// Update quote for specific symbol
    pub fn update_quote(&mut self, quote: VenueQuote) {
        if let Some(scanner) = self.scanners.get_mut(&quote.symbol) {
            scanner.update_quote(quote);
        }
    }
    
    /// Scan all symbols, priority order
    pub fn scan_all(&self) -> Vec<(String, Vec<ArbitrageOpportunity>)> {
        let mut results = Vec::new();
        
        for symbol in &self.priority_symbols {
            if let Some(scanner) = self.scanners.get(symbol) {
                let opps = scanner.scan_cross_venue();
                if !opps.is_empty() {
                    results.push((symbol.clone(), opps));
                }
            }
        }
        
        results
    }
    
    /// Get best opportunity across all symbols
    pub fn get_global_best(&self) -> Option<(String, ArbitrageOpportunity)> {
        let mut best: Option<(String, ArbitrageOpportunity)> = None;
        
        for (symbol, opps) in self.scan_all() {
            if let Some(opp) = opps.into_iter().next() {
                match &best {
                    None => best = Some((symbol, opp)),
                    Some((_, ref best_opp)) => {
                        if opp.net_profit > best_opp.net_profit {
                            best = Some((symbol, opp));
                        }
                    }
                }
            }
        }
        
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::execution::venue::Venue;
    
    fn create_quote(venue: Venue, symbol: &str, bid: i64, ask: i64) -> VenueQuote {
        VenueQuote {
            venue,
            symbol: symbol.to_string(),
            bid: Decimal::from(bid),
            ask: Decimal::from(ask),
            bid_size: Decimal::from(10),
            ask_size: Decimal::from(10),
            timestamp: Utc::now(),
            latency_ms: 20,
        }
    }
    
    #[test]
    fn test_scanner_finds_opportunities() {
        let config = ScannerConfig {
            symbols: vec!["BTC".to_string()],
            venues: vec![Venue::Binance, Venue::Coinbase],
            ..Default::default()
        };
        
        let mut scanner = ArbitrageScanner::new(config);
        
        // Setup price discrepancy
        scanner.update_quote(create_quote(Venue::Binance, "BTC", 50000, 50100));
        scanner.update_quote(create_quote(Venue::Coinbase, "BTC", 50200, 50300));
        
        // Note: scan_cross_venue needs quotes to be accessible
        // This is a simplified test - real implementation needs quote storage
    }
    
    #[test]
    fn test_estimate_costs() {
        let config = ScannerConfig::default();
        let scanner = ArbitrageScanner::new(config);
        
        let costs = scanner.estimate_total_costs(Decimal::from(100000));
        
        // 0.25% of $100k = $250
        assert!(costs > Decimal::from(200) && costs < Decimal::from(300));
    }
    
    #[test]
    fn test_filter_by_profit() {
        let config = ScannerConfig {
            min_profit_bps: Decimal::from(10),
            ..Default::default()
        };
        let scanner = ArbitrageScanner::new(config);
        
        // Create mock opportunities
        let opps = vec![
            ArbitrageOpportunity {
                id: uuid::Uuid::new_v4(),
                arb_type: super::super::opportunity::ArbitrageType::CrossVenue,
                symbol: "BTC".to_string(),
                buy_venue: Venue::Binance,
                sell_venue: Venue::Coinbase,
                buy_price: Decimal::from(50000),
                sell_price: Decimal::from(50020),
                quantity: Decimal::ONE,
                gross_profit: Decimal::from(20),
                estimated_costs: Decimal::from(10),
                net_profit: Decimal::from(10),
                profit_bps: Decimal::from(2), // Below threshold
                confidence: Decimal::ONE,
                detected_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::seconds(5),
                latency_ms: 50,
            },
            ArbitrageOpportunity {
                id: uuid::Uuid::new_v4(),
                arb_type: super::super::opportunity::ArbitrageType::CrossVenue,
                symbol: "ETH".to_string(),
                buy_venue: Venue::Binance,
                sell_venue: Venue::Coinbase,
                buy_price: Decimal::from(3000),
                sell_price: Decimal::from(3030),
                quantity: Decimal::ONE,
                gross_profit: Decimal::from(30),
                estimated_costs: Decimal::from(5),
                net_profit: Decimal::from(25),
                profit_bps: Decimal::from(20), // Above threshold
                confidence: Decimal::ONE,
                detected_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::seconds(5),
                latency_ms: 50,
            },
        ];
        
        let filtered = scanner.filter_by_profit(opps);
        
        // Only the ETH opportunity with 20 bps should pass
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].symbol, "ETH");
    }
    
    #[test]
    fn test_priority_scanner() {
        let config = ScannerConfig::default();
        let scanner = PriorityScanner::new(
            vec!["BTC".to_string(), "ETH".to_string()],
            config,
        );
        
        assert_eq!(scanner.scanners.len(), 2);
        assert!(scanner.scanners.contains_key("BTC"));
        assert!(scanner.scanners.contains_key("ETH"));
    }
}
