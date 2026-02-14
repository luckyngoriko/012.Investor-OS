//! Margin calculations and risk metrics

use rust_decimal::Decimal;
use std::collections::HashMap;

use super::account::MarginAccount;
use super::position::Position;

/// Margin calculator for risk analysis
#[derive(Debug, Clone)]
pub struct MarginCalculator;

/// Risk metrics for a margin account
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    /// Margin ratio (equity / locked_margin)
    pub margin_ratio: Decimal,
    /// Maintenance margin ratio required
    pub maintenance_ratio: Decimal,
    /// Distance to margin call (percentage)
    pub distance_to_call: Decimal,
    /// Liquidation price for each position
    pub liquidation_prices: HashMap<String, Decimal>,
    /// Portfolio Value at Risk (95% confidence)
    pub var_95: Decimal,
    /// Maximum drawdown estimate
    pub max_drawdown_estimate: Decimal,
    /// Leverage used (notional / equity)
    pub leverage_used: Decimal,
}

impl MarginCalculator {
    /// Create new calculator
    pub fn new() -> Self {
        Self
    }
    
    /// Calculate comprehensive risk metrics
    pub fn calculate_risk(&self, account: &MarginAccount) -> RiskMetrics {
        let margin_ratio = account.margin_ratio();
        let maintenance_ratio = Decimal::ONE + account.maintenance_margin_rate;
        
        // Distance to margin call
        let distance_to_call = if margin_ratio == Decimal::MAX {
            Decimal::MAX // No positions
        } else if margin_ratio > maintenance_ratio {
            ((margin_ratio - maintenance_ratio) / maintenance_ratio) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        
        // Liquidation prices
        let liquidation_prices = self.calculate_liquidation_prices(account);
        
        // VaR (simplified - assumes 2% daily vol)
        let var_95 = self.calculate_var(account, Decimal::try_from(0.02).unwrap());
        
        // Leverage used
        let leverage_used = if account.equity.is_zero() {
            Decimal::ZERO
        } else {
            account.total_exposure() / account.equity
        };
        
        RiskMetrics {
            margin_ratio,
            maintenance_ratio,
            distance_to_call,
            liquidation_prices,
            var_95,
            max_drawdown_estimate: var_95 * Decimal::from(2), // Rough estimate
            leverage_used,
        }
    }
    
    /// Calculate liquidation price for each position
    pub fn calculate_liquidation_prices(&self, account: &MarginAccount) -> HashMap<String, Decimal> {
        let mut prices = HashMap::new();
        
        for (symbol, position) in &account.positions {
            let liq_price = self.position_liquidation_price(
                position,
                account.equity,
                account.locked_margin,
                account.maintenance_margin_rate,
            );
            prices.insert(symbol.clone(), liq_price);
        }
        
        prices
    }
    
    /// Calculate liquidation price for single position
    /// Formula: For long: Entry * (1 - (Equity / Notional) + Maintenance)
    fn position_liquidation_price(
        &self,
        position: &Position,
        equity: Decimal,
        total_locked: Decimal,
        maintenance_rate: Decimal,
    ) -> Decimal {
        if position.quantity.is_zero() {
            return Decimal::ZERO;
        }
        
        let notional = position.notional_value();
        let position_margin = position.margin_used();
        
        // Share of total margin (proportional allocation)
        let margin_share = if total_locked.is_zero() {
            Decimal::ONE
        } else {
            position_margin / total_locked
        };
        
        // Equity allocated to this position
        let position_equity = equity * margin_share;
        
        // Maintenance requirement
        let maintenance = notional * maintenance_rate;
        
        match position.side {
            super::position::PositionSide::Long => {
                // Liquidation when: position_equity + pnl <= maintenance
                // pnl = (liq_price - entry) * qty
                // liq_price = entry - (position_equity - maintenance) / qty
                let buffer = position_equity - maintenance;
                let price_drop = buffer / position.quantity;
                position.entry_price - price_drop
            }
            super::position::PositionSide::Short => {
                // For short: pnl = (entry - liq_price) * qty
                // liq_price = entry + (position_equity - maintenance) / qty
                let buffer = position_equity - maintenance;
                let price_rise = buffer / position.quantity;
                position.entry_price + price_rise
            }
        }
    }
    
    /// Calculate Value at Risk (simplified parametric VaR)
    pub fn calculate_var(&self, account: &MarginAccount, daily_volatility: Decimal) -> Decimal {
        // VaR = Portfolio Value * Z * volatility
        // Z = 1.645 for 95% confidence
        let z_score = Decimal::try_from(1.645).unwrap();
        let portfolio_value = account.total_exposure();
        
        portfolio_value * z_score * daily_volatility
    }
    
    /// Calculate buying power
    pub fn buying_power(&self, account: &MarginAccount) -> Decimal {
        // Available margin * max leverage
        account.available_margin * account.max_leverage
    }
    
    /// Calculate maximum position size for given capital
    pub fn max_position_size(
        &self,
        available_margin: Decimal,
        price: Decimal,
        leverage: Decimal,
    ) -> Decimal {
        // Max qty = (available * leverage) / price
        if price.is_zero() {
            return Decimal::ZERO;
        }
        
        (available_margin * leverage) / price
    }
    
    /// Check if portfolio is well-balanced
    pub fn concentration_risk(&self, account: &MarginAccount) -> Vec<(String, Decimal)> {
        let total_exposure = account.total_exposure();
        
        if total_exposure.is_zero() {
            return Vec::new();
        }
        
        account.positions
            .values()
            .map(|p| {
                let concentration = (p.notional_value() / total_exposure) * Decimal::from(100);
                (p.symbol.clone(), concentration)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::margin::position::{Position, PositionSide};
    
    #[test]
    fn test_buying_power_calculation() {
        let calc = MarginCalculator::new();
        let account = MarginAccount::new(
            "test".to_string(),
            Decimal::from(100000),
        );
        
        // With $100k and 20x leverage = $2M buying power
        let bp = calc.buying_power(&account);
        assert_eq!(bp, Decimal::from(2000000));
    }
    
    #[test]
    fn test_max_position_size() {
        let calc = MarginCalculator::new();
        
        // $10k available, BTC at $50k, 5x leverage
        let max_size = calc.max_position_size(
            Decimal::from(10000),
            Decimal::from(50000),
            Decimal::from(5),
        );
        
        // Max notional = $50k, at $50k/BTC = 1 BTC
        assert_eq!(max_size, Decimal::from(1));
    }
    
    #[test]
    fn test_liquidation_price_long() {
        let calc = MarginCalculator::new();
        let mut account = MarginAccount::new(
            "test".to_string(),
            Decimal::from(10000),
        );
        
        let position = Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(10), // 10x leverage
        );
        
        account.open_position(position).unwrap();
        
        let liq_prices = calc.calculate_liquidation_prices(&account);
        let liq_price = liq_prices.get("BTC").unwrap();
        
        // At 10x with 5% maintenance, liq should be around $45,000
        // (roughly 10% below entry for 10x with buffer)
        assert!(*liq_price < Decimal::from(50000));
        assert!(*liq_price > Decimal::from(40000));
    }
    
    #[test]
    fn test_concentration_risk() {
        let calc = MarginCalculator::new();
        let mut account = MarginAccount::new(
            "test".to_string(),
            Decimal::from(100000),
        );
        
        // Open two positions
        account.open_position(Position::new(
            "BTC".to_string(),
            PositionSide::Long,
            Decimal::from(1),
            Decimal::from(50000),
            Decimal::from(5),
        )).unwrap();
        
        account.open_position(Position::new(
            "ETH".to_string(),
            PositionSide::Long,
            Decimal::from(10),
            Decimal::from(3000),
            Decimal::from(5),
        )).unwrap();
        
        let concentration = calc.concentration_risk(&account);
        
        // BTC: $50k, ETH: $30k, Total: $80k
        // BTC concentration = 50/80 = 62.5%
        let btc_conc = concentration.iter()
            .find(|(s, _)| s == "BTC")
            .map(|(_, c)| *c)
            .unwrap();
        
        assert!(btc_conc > Decimal::from(60) && btc_conc < Decimal::from(65));
    }
    
    #[test]
    fn test_risk_metrics() {
        let calc = MarginCalculator::new();
        let account = MarginAccount::new(
            "test".to_string(),
            Decimal::from(100000),
        );
        
        let metrics = calc.calculate_risk(&account);
        
        // Empty account should have MAX margin ratio
        assert_eq!(metrics.margin_ratio, Decimal::MAX);
        assert!(metrics.leverage_used.is_zero());
    }
}
