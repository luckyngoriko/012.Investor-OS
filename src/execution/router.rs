//! Smart Order Router (SOR) - Routes orders to optimal venues

use rust_decimal::Decimal;
use tracing::info;
use uuid::Uuid;

use super::order::Order;
use super::venue::{Venue, VenueAnalyzer};
use super::cost::CostCalculator;
use super::error::{ExecutionError, Result};

/// Routing decision
#[derive(Debug, Clone)]
pub struct RouteDecision {
    pub order_id: Uuid,
    pub primary_venue: Venue,
    pub backup_venues: Vec<Venue>,
    pub expected_cost: Decimal,
    pub reason: String,
}

/// Smart Order Router
#[derive(Debug)]
pub struct SmartRouter {
    venue_analyzer: VenueAnalyzer,
    cost_calculator: CostCalculator,
    default_backup_venues: Vec<Venue>,
}

impl SmartRouter {
    pub fn new() -> Self {
        Self {
            venue_analyzer: VenueAnalyzer::new(),
            cost_calculator: CostCalculator::new(),
            default_backup_venues: vec![
                Venue::InteractiveBrokers,
                Venue::Binance,
            ],
        }
    }
    
    /// Route order to best venue(s)
    pub fn route(&self, order: &Order) -> Result<RouteDecision> {
        // If venue is specified, use it
        if let Some(venue) = &order.venue {
            return Ok(RouteDecision {
                order_id: order.id,
                primary_venue: venue.clone(),
                backup_venues: self.default_backup_venues.clone(),
                expected_cost: Decimal::ZERO,
                reason: format!("User specified venue: {}", venue.name()),
            });
        }
        
        // Get venue scores
        let scores = self.venue_analyzer.score_venues(
            &order.symbol,
            order.quantity,
            order.side,
        );
        
        if scores.is_empty() {
            return Err(ExecutionError::RoutingFailed(
                format!("No venues available for {}", order.symbol)
            ));
        }
        
        let best = &scores[0];
        let backups: Vec<Venue> = scores.iter()
            .skip(1)
            .take(2)
            .map(|s| s.venue.clone())
            .collect();
        
        info!(
            "Routed order {} for {} to {} (score: {}, cost: ${})",
            order.id,
            order.symbol,
            best.venue.name(),
            best.composite_score,
            best.total_cost
        );
        
        Ok(RouteDecision {
            order_id: order.id,
            primary_venue: best.venue.clone(),
            backup_venues: backups,
            expected_cost: best.total_cost,
            reason: format!("Best composite score: {}", best.composite_score),
        })
    }
    
    /// Route with cost optimization
    pub fn route_with_cost_optimization(&self, order: &Order) -> Result<RouteDecision> {
        let basic_route = self.route(order)?;
        
        // Get quotes for cost comparison
        // (In real implementation, would fetch live quotes)
        
        Ok(basic_route)
    }
    
    /// Split large orders across multiple venues
    pub fn route_large_order(&self, order: &Order) -> Vec<RouteDecision> {
        let mut decisions = Vec::new();
        let scores = self.venue_analyzer.score_venues(
            &order.symbol,
            order.quantity,
            order.side,
        );
        
        if scores.len() < 2 {
            // Not enough venues, route all to best
            if let Ok(decision) = self.route(order) {
                decisions.push(decision);
            }
            return decisions;
        }
        
        // Split order between top 2 venues
        let total_score = scores[0].composite_score + scores[1].composite_score;
        if total_score.is_zero() {
            // Equal split if no score difference
            let _half = order.quantity / Decimal::from(2);
            
            decisions.push(RouteDecision {
                order_id: order.id,
                primary_venue: scores[0].venue.clone(),
                backup_venues: vec![],
                expected_cost: scores[0].total_cost,
                reason: "50% split for large order".to_string(),
            });
            
            decisions.push(RouteDecision {
                order_id: order.id,
                primary_venue: scores[1].venue.clone(),
                backup_venues: vec![],
                expected_cost: scores[1].total_cost,
                reason: "50% split for large order".to_string(),
            });
        } else {
            // Proportional split based on scores
            let ratio = scores[0].composite_score / total_score;
            let qty1 = order.quantity * ratio;
            let _qty2 = order.quantity - qty1;
            
            decisions.push(RouteDecision {
                order_id: order.id,
                primary_venue: scores[0].venue.clone(),
                backup_venues: vec![],
                expected_cost: scores[0].total_cost,
                reason: format!("{:.0}% split based on venue score", ratio * Decimal::from(100)),
            });
            
            decisions.push(RouteDecision {
                order_id: order.id,
                primary_venue: scores[1].venue.clone(),
                backup_venues: vec![],
                expected_cost: scores[1].total_cost,
                reason: format!("{:.0}% split based on venue score", (Decimal::ONE - ratio) * Decimal::from(100)),
            });
        }
        
        decisions
    }
    
    /// Get access to venue analyzer (for updating quotes)
    pub fn venue_analyzer_mut(&mut self) -> &mut VenueAnalyzer {
        &mut self.venue_analyzer
    }
    
    /// Get venue analyzer (read-only)
    pub fn venue_analyzer(&self) -> &VenueAnalyzer {
        &self.venue_analyzer
    }
}

impl Default for SmartRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::venue::{Venue, VenueQuote};
    use super::super::order::OrderSide;
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
            latency_ms: 10,
        }
    }
    
    #[test]
    fn test_route_with_specified_venue() {
        let router = SmartRouter::new();
        let order = Order::market("BTC", OrderSide::Buy, Decimal::from(1))
            .with_venue(Venue::Binance);
        
        let decision = router.route(&order).unwrap();
        
        assert_eq!(decision.primary_venue, Venue::Binance);
        assert!(decision.reason.contains("User specified"));
    }
    
    #[test]
    fn test_route_finds_best_venue() {
        let mut router = SmartRouter::new();
        
        // Add quotes - Binance has better price
        router.venue_analyzer_mut().update_quote(create_quote(Venue::Binance, 50000, 50100));
        router.venue_analyzer_mut().update_quote(create_quote(Venue::Coinbase, 50050, 50150));
        
        let order = Order::market("BTC", OrderSide::Buy, Decimal::from(5));
        let decision = router.route(&order).unwrap();
        
        // Should pick Binance (lower ask)
        assert_eq!(decision.primary_venue, Venue::Binance);
    }
    
    #[test]
    fn test_route_no_venues_available() {
        let router = SmartRouter::new();
        let order = Order::market("UNKNOWN", OrderSide::Buy, Decimal::from(1));
        
        let result = router.route(&order);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_route_large_order_split() {
        let mut router = SmartRouter::new();
        
        router.venue_analyzer_mut().update_quote(create_quote(Venue::Binance, 50000, 50100));
        router.venue_analyzer_mut().update_quote(create_quote(Venue::Coinbase, 50010, 50110));
        router.venue_analyzer_mut().update_quote(create_quote(Venue::Kraken, 50020, 50120));
        
        let order = Order::market("BTC", OrderSide::Buy, Decimal::from(100));
        let decisions = router.route_large_order(&order);
        
        // Should split between top 2 venues
        assert_eq!(decisions.len(), 2);
        assert_ne!(decisions[0].primary_venue, decisions[1].primary_venue);
    }
}
