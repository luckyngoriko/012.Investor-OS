//! Global Order Router

use super::{ExchangeId, GlobalExchangeRegistry, GlobalExchangeError};
use rust_decimal::Decimal;

/// Global order router
#[derive(Debug)]
pub struct GlobalOrderRouter {
    registry: GlobalExchangeRegistry,
}

/// Routing decision
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    pub exchange: ExchangeId,
    pub reason: String,
    pub score: LiquidityScore,
}

/// Liquidity score for exchange selection
#[derive(Debug, Clone)]
pub struct LiquidityScore {
    pub exchange: ExchangeId,
    pub score: f64,
    pub depth: Decimal,
    pub spread: Decimal,
    pub latency_ms: u64,
}

impl GlobalOrderRouter {
    pub fn new(registry: GlobalExchangeRegistry) -> Self {
        Self { registry }
    }
    
    /// Route order to best exchange
    pub fn route_order(&self, symbol: &str) -> Result<RoutingDecision, GlobalExchangeError> {
        let exchanges = self.registry.get_exchanges_for_symbol(symbol);
        
        if exchanges.is_empty() {
            return Err(GlobalExchangeError::ExchangeNotFound(
                format!("No exchange supports {}", symbol)
            ));
        }
        
        // Simplified: route to first available
        let exchange = exchanges[0];
        
        Ok(RoutingDecision {
            exchange: exchange.id.clone(),
            reason: "Best liquidity".to_string(),
            score: LiquidityScore {
                exchange: exchange.id.clone(),
                score: 1.0,
                depth: Decimal::ZERO,
                spread: Decimal::ZERO,
                latency_ms: 0,
            },
        })
    }
    
    /// Get best price across exchanges
    pub fn get_best_price(&self, _symbol: &str) -> Option<Decimal> {
        // Would query all exchanges and return best price
        None
    }
}
