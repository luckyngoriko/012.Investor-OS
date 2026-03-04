//! Paper Trading Portfolio
//!
//! Tracks virtual positions, P&L, and account state

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

use crate::broker::{Execution, OrderSide, Position};

/// Paper trading portfolio state
#[derive(Debug, Clone)]
pub struct PaperPortfolio {
    cash: Decimal,
    positions: HashMap<String, PaperPosition>,
    executions: Vec<Execution>,
    created_at: DateTime<Utc>,
}

/// Virtual position in a symbol
#[derive(Debug, Clone)]
pub struct PaperPosition {
    pub symbol: String,
    pub quantity: Decimal,
    pub avg_cost: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub total_commissions: Decimal, // Track commissions paid
    pub portfolio_id: Uuid,
    pub opened_at: DateTime<Utc>,
}

impl PaperPortfolio {
    /// Create new portfolio with initial cash
    pub fn new(initial_cash: Decimal) -> Self {
        Self {
            cash: initial_cash,
            positions: HashMap::new(),
            executions: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Apply an execution to the portfolio
    pub fn apply_execution(&mut self, execution: &Execution) {
        let notional = execution.quantity * execution.price;
        let total_cost = notional + execution.commission;

        match execution.side {
            OrderSide::Buy => {
                // Deduct cash
                self.cash -= total_cost;

                // Update or create position
                let position = self
                    .positions
                    .entry(execution.ticker.clone())
                    .or_insert_with(|| PaperPosition {
                        symbol: execution.ticker.clone(),
                        quantity: Decimal::ZERO,
                        avg_cost: Decimal::ZERO,
                        unrealized_pnl: Decimal::ZERO,
                        realized_pnl: Decimal::ZERO,
                        total_commissions: Decimal::ZERO,
                        portfolio_id: Uuid::new_v4(),
                        opened_at: Utc::now(),
                    });

                // Track buy commission
                position.total_commissions += execution.commission;

                // Update average cost (including commission in cost basis)
                let current_cost = position.quantity * position.avg_cost;
                let new_cost = execution.quantity * execution.price + execution.commission;
                position.quantity += execution.quantity;

                if !position.quantity.is_zero() {
                    position.avg_cost = (current_cost + new_cost) / position.quantity;
                }
            }
            OrderSide::Sell => {
                // Add cash
                self.cash += notional - execution.commission;

                if let Some(position) = self.positions.get_mut(&execution.ticker) {
                    // Calculate realized P&L
                    // avg_cost already includes the buy commission per share
                    let cost_basis = execution.quantity * position.avg_cost;
                    let proceeds = execution.quantity * execution.price;
                    let realized = proceeds - cost_basis - execution.commission;
                    position.realized_pnl += realized;
                    position.total_commissions += execution.commission;

                    // Update position
                    position.quantity -= execution.quantity;

                    // Keep avg_cost for remaining shares
                    if position.quantity.is_zero() {
                        position.avg_cost = Decimal::ZERO;
                        position.total_commissions = Decimal::ZERO;
                    }
                }
            }
        }

        self.executions.push(execution.clone());
    }

    /// Update unrealized P&L with current market prices
    pub fn update_market_prices(&mut self, prices: &HashMap<String, Decimal>) {
        for (symbol, position) in &mut self.positions {
            if let Some(current_price) = prices.get(symbol) {
                let market_value = position.quantity * *current_price;
                let cost_basis = position.quantity * position.avg_cost;
                position.unrealized_pnl = market_value - cost_basis;
            }
        }
    }

    /// Get cash balance
    pub fn cash_balance(&self) -> Decimal {
        self.cash
    }

    /// Get position for symbol
    pub fn get_position(&self, symbol: &str) -> Option<&PaperPosition> {
        self.positions.get(symbol)
    }

    /// Get all positions
    pub fn positions(&self) -> &HashMap<String, PaperPosition> {
        &self.positions
    }

    /// Get total equity (cash + position values)
    pub fn total_equity(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        let position_value: Decimal = self
            .positions
            .iter()
            .map(|(sym, pos)| {
                prices
                    .get(sym)
                    .map(|p| pos.quantity * *p)
                    .unwrap_or(Decimal::ZERO)
            })
            .sum();

        self.cash + position_value
    }

    /// Get total unrealized P&L
    pub fn total_unrealized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.unrealized_pnl).sum()
    }

    /// Get total realized P&L
    pub fn total_realized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.realized_pnl).sum()
    }

    /// Convert to broker Positions
    pub fn to_positions(&self) -> Vec<Position> {
        self.positions
            .values()
            .filter(|p| !p.quantity.is_zero())
            .map(|p| p.to_position())
            .collect()
    }

    /// Get execution history
    pub fn executions(&self) -> &[Execution] {
        &self.executions
    }

    /// Get portfolio summary
    pub fn summary(&self, prices: &HashMap<String, Decimal>) -> PortfolioSummary {
        let equity = self.total_equity(prices);
        let initial_equity = self
            .executions
            .first()
            .map(|_| {
                // Approximate initial equity
                self.cash + self.total_realized_pnl() + self.total_unrealized_pnl()
            })
            .unwrap_or(equity);

        let total_return = if !initial_equity.is_zero() {
            (equity - initial_equity) / initial_equity * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        PortfolioSummary {
            cash: self.cash,
            equity,
            total_return_pct: total_return,
            realized_pnl: self.total_realized_pnl(),
            unrealized_pnl: self.total_unrealized_pnl(),
            position_count: self.positions.len(),
            trade_count: self.executions.len(),
        }
    }
}

/// Portfolio summary statistics
#[derive(Debug, Clone)]
pub struct PortfolioSummary {
    pub cash: Decimal,
    pub equity: Decimal,
    pub total_return_pct: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub position_count: usize,
    pub trade_count: usize,
}

impl PaperPosition {
    /// Check if position is long
    pub fn is_long(&self) -> bool {
        self.quantity > Decimal::ZERO
    }

    /// Check if position is short
    pub fn is_short(&self) -> bool {
        self.quantity < Decimal::ZERO
    }

    /// Get position value at given price
    pub fn market_value(&self, price: Decimal) -> Decimal {
        self.quantity * price
    }

    /// Convert to broker Position
    pub fn to_position(&self) -> Position {
        Position {
            id: Uuid::new_v4(),
            ticker: self.symbol.clone(),
            quantity: self.quantity,
            avg_cost: self.avg_cost,
            market_price: None,
            market_value: None,
            unrealized_pnl: Some(self.unrealized_pnl),
            realized_pnl: self.realized_pnl,
            portfolio_id: self.portfolio_id,
            opened_at: self.opened_at,
            updated_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_creation() {
        let portfolio = PaperPortfolio::new(Decimal::from(100000));
        assert_eq!(portfolio.cash_balance(), Decimal::from(100000));
        assert!(portfolio.positions().is_empty());
    }

    #[test]
    fn test_apply_buy_execution() {
        let mut portfolio = PaperPortfolio::new(Decimal::from(100000));

        let execution = Execution {
            id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            broker_execution_id: "exec-1".to_string(),
            ticker: "BTC".to_string(),
            side: OrderSide::Buy,
            quantity: Decimal::from(1),
            price: Decimal::from(50000),
            commission: Decimal::from(50),
            timestamp: Utc::now(),
        };

        portfolio.apply_execution(&execution);

        assert_eq!(portfolio.cash_balance(), Decimal::from(49950)); // 100000 - 50000 - 50
        assert_eq!(portfolio.positions().len(), 1);

        let pos = portfolio.get_position("BTC").unwrap();
        assert_eq!(pos.quantity, Decimal::from(1));
        // avg_cost includes commission: (50000 + 50) / 1 = 50050
        assert_eq!(pos.avg_cost, Decimal::from(50050));
    }

    #[test]
    fn test_apply_sell_execution() {
        let mut portfolio = PaperPortfolio::new(Decimal::from(100000));

        // First buy
        let buy_exec = Execution {
            id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            broker_execution_id: "exec-1".to_string(),
            ticker: "BTC".to_string(),
            side: OrderSide::Buy,
            quantity: Decimal::from(1),
            price: Decimal::from(50000),
            commission: Decimal::from(50),
            timestamp: Utc::now(),
        };
        portfolio.apply_execution(&buy_exec);

        // Then sell at profit
        let sell_exec = Execution {
            id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            broker_execution_id: "exec-2".to_string(),
            ticker: "BTC".to_string(),
            side: OrderSide::Sell,
            quantity: Decimal::from(1),
            price: Decimal::from(55000),
            commission: Decimal::from(55),
            timestamp: Utc::now(),
        };
        portfolio.apply_execution(&sell_exec);

        // Cash should be: 49950 + 55000 - 55 = 104895
        assert_eq!(portfolio.cash_balance(), Decimal::from(104895));

        let pos = portfolio.get_position("BTC").unwrap();
        assert!(pos.quantity.is_zero());
        // Realized P&L: (55000 - 50000) - 50 - 55 = 4895
        assert_eq!(pos.realized_pnl, Decimal::from(4895));
    }

    #[test]
    fn test_unrealized_pnl() {
        let mut portfolio = PaperPortfolio::new(Decimal::from(100000));

        let execution = Execution {
            id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            broker_execution_id: "exec-1".to_string(),
            ticker: "BTC".to_string(),
            side: OrderSide::Buy,
            quantity: Decimal::from(1),
            price: Decimal::from(50000),
            commission: Decimal::from(50),
            timestamp: Utc::now(),
        };
        portfolio.apply_execution(&execution);

        let mut prices = HashMap::new();
        prices.insert("BTC".to_string(), Decimal::from(55000));

        portfolio.update_market_prices(&prices);

        let pos = portfolio.get_position("BTC").unwrap();
        // Unrealized P&L: 55000 - 50050 (avg_cost with commission) = 4950
        assert_eq!(pos.unrealized_pnl, Decimal::from(4950));
    }
}
