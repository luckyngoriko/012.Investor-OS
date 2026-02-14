//! Continuous Trading Engine

use super::MarketId;
use chrono::Duration;

/// Continuous trading engine
#[derive(Debug)]
pub struct ContinuousTradingEngine {
    opportunities: Vec<TradingOpportunity>,
}

impl ContinuousTradingEngine {
    pub fn new() -> Self {
        Self {
            opportunities: Vec::new(),
        }
    }
    
    /// Find opportunities across active markets
    pub fn find_opportunities(&self, markets: &[&super::Market]) -> Vec<TradingOpportunity> {
        markets.iter()
            .map(|m| TradingOpportunity {
                market: m.id.clone(),
                market_name: m.name.clone(),
                time_until_close: Duration::hours(6), // Simplified
                session_type: super::SessionType::Regular,
                priority: 1,
            })
            .collect()
    }
}

impl Default for ContinuousTradingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Trading opportunity
#[derive(Debug, Clone)]
pub struct TradingOpportunity {
    pub market: MarketId,
    pub market_name: String,
    pub time_until_close: Duration,
    pub session_type: super::SessionType,
    pub priority: u32,
}

/// Market status
#[derive(Debug, Clone)]
pub struct MarketStatus {
    pub market: MarketId,
    pub is_open: bool,
    pub session_type: super::SessionType,
    pub time_until_close: Option<Duration>,
    pub time_until_open: Option<Duration>,
}
