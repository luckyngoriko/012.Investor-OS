//! Portfolio Optimization Engine
//!
//! Sprint 32: Portfolio Optimization
//! - Modern Portfolio Theory (MPT)
//! - Black-Litterman model
//! - Risk parity allocation
//! - Efficient frontier calculation
//! - Maximum diversification portfolio

use rust_decimal::Decimal;
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

pub mod black_litterman;
pub mod efficient_frontier;
pub mod mpt;
pub mod risk_parity;

pub use black_litterman::{BlackLittermanModel, InvestorView, ViewConfidence};
pub use efficient_frontier::{EfficientFrontier, PortfolioPoint};
pub use mpt::{Asset, MarkowitzOptimizer, PortfolioStats};
pub use risk_parity::{RiskParityOptimizer, RiskContribution};

/// Portfolio optimization errors
#[derive(Error, Debug, Clone)]
pub enum OptimizationError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
    
    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),
    
    #[error("Invalid constraints: {0}")]
    InvalidConstraints(String),
    
    #[error("Singular covariance matrix")]
    SingularMatrix,
    
    #[error("No feasible solution")]
    NoFeasibleSolution,
}

pub type Result<T> = std::result::Result<T, OptimizationError>;

/// Optimization objective
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationObjective {
    MaximizeReturn,
    MinimizeRisk,
    MaximizeSharpe,
    RiskParity,
    MaxDiversification,
}

/// Optimization constraints
#[derive(Debug, Clone)]
pub struct OptimizationConstraints {
    pub min_weight: Decimal,
    pub max_weight: Decimal,
    pub min_assets: usize,
    pub max_assets: usize,
    pub target_return: Option<Decimal>,
    pub target_risk: Option<Decimal>,
    pub allow_short: bool,
    pub max_turnover: Option<Decimal>,
}

impl Default for OptimizationConstraints {
    fn default() -> Self {
        Self {
            min_weight: Decimal::ZERO,
            max_weight: Decimal::from(1),
            min_assets: 1,
            max_assets: 100,
            target_return: None,
            target_risk: None,
            allow_short: false,
            max_turnover: None,
        }
    }
}

/// Optimized portfolio
#[derive(Debug, Clone)]
pub struct OptimizedPortfolio {
    pub id: Uuid,
    pub name: String,
    pub objective: OptimizationObjective,
    pub weights: HashMap<String, Decimal>,
    pub expected_return: Decimal,
    pub expected_risk: Decimal,
    pub sharpe_ratio: f32,
    pub diversification_ratio: f32,
    pub concentration_risk: f32, // HHI index
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl OptimizedPortfolio {
    /// Create new optimized portfolio
    pub fn new(name: &str, objective: OptimizationObjective) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            objective,
            weights: HashMap::new(),
            expected_return: Decimal::ZERO,
            expected_risk: Decimal::ZERO,
            sharpe_ratio: 0.0,
            diversification_ratio: 0.0,
            concentration_risk: 0.0,
            created_at: chrono::Utc::now(),
        }
    }
    
    /// Get weight for asset
    pub fn weight(&self, symbol: &str) -> Decimal {
        self.weights.get(symbol).copied().unwrap_or(Decimal::ZERO)
    }
    
    /// Check if weights sum to 1.0
    pub fn is_fully_invested(&self) -> bool {
        let total: Decimal = self.weights.values().sum();
        (total - Decimal::from(1)).abs() < Decimal::try_from(0.0001f64).unwrap()
    }
    
    /// Calculate Herfindahl-Hirschman Index (concentration)
    pub fn calculate_concentration(&self) -> f32 {
        let sum_squares: f32 = self.weights.values()
            .map(|w| {
                let f: f32 = (*w).try_into().unwrap_or(0.0f32);
                f * f
            })
            .sum();
        sum_squares
    }
    
    /// Get top holdings
    pub fn top_holdings(&self, n: usize) -> Vec<(&String, &Decimal)> {
        let mut holdings: Vec<_> = self.weights.iter().collect();
        holdings.sort_by(|a, b| b.1.cmp(a.1));
        holdings.into_iter().take(n).collect()
    }
    
    /// Calculate number of holdings
    pub fn holding_count(&self) -> usize {
        self.weights.values().filter(|w| **w > Decimal::ZERO).count()
    }
}

/// Portfolio optimization engine
#[derive(Debug)]
pub struct PortfolioOptimizationEngine {
    mpt_optimizer: MarkowitzOptimizer,
    bl_model: BlackLittermanModel,
    rp_optimizer: RiskParityOptimizer,
    ef_calculator: EfficientFrontier,
    risk_free_rate: Decimal,
}

impl PortfolioOptimizationEngine {
    /// Create new optimization engine
    pub fn new() -> Self {
        Self {
            mpt_optimizer: MarkowitzOptimizer::new(),
            bl_model: BlackLittermanModel::new(),
            rp_optimizer: RiskParityOptimizer::new(),
            ef_calculator: EfficientFrontier::new(),
            risk_free_rate: Decimal::try_from(0.02).unwrap(), // 2%
        }
    }
    
    /// Optimize using Markowitz (MPT)
    pub fn optimize_mpt(
        &self,
        assets: &[Asset],
        objective: OptimizationObjective,
        constraints: OptimizationConstraints,
    ) -> Result<OptimizedPortfolio> {
        self.mpt_optimizer.optimize(assets, objective, constraints)
    }
    
    /// Optimize using Black-Litterman
    pub fn optimize_black_litterman(
        &self,
        assets: &[Asset],
        market_caps: &HashMap<String, Decimal>,
        views: &[InvestorView],
        constraints: OptimizationConstraints,
    ) -> Result<OptimizedPortfolio> {
        self.bl_model.optimize(assets, market_caps, views, constraints)
    }
    
    /// Optimize using Risk Parity
    pub fn optimize_risk_parity(
        &self,
        assets: &[Asset],
        constraints: OptimizationConstraints,
    ) -> Result<OptimizedPortfolio> {
        self.rp_optimizer.optimize(assets, constraints)
    }
    
    /// Calculate efficient frontier
    pub fn calculate_efficient_frontier(
        &self,
        assets: &[Asset],
        points: usize,
    ) -> Vec<PortfolioPoint> {
        self.ef_calculator.calculate(assets, points)
    }
    
    /// Get maximum Sharpe portfolio from frontier
    pub fn get_max_sharpe_portfolio(&self, frontier: &[PortfolioPoint]) -> Option<PortfolioPoint> {
        frontier.iter().max_by(|a, b| {
            a.sharpe_ratio.partial_cmp(&b.sharpe_ratio).unwrap()
        }).cloned()
    }
    
    /// Get minimum variance portfolio
    pub fn get_min_variance_portfolio(&self, frontier: &[PortfolioPoint]) -> Option<PortfolioPoint> {
        frontier.iter().min_by(|a, b| {
            a.risk.partial_cmp(&b.risk).unwrap()
        }).cloned()
    }
    
    /// Compare portfolios
    pub fn compare_portfolios(
        &self,
        portfolios: &[OptimizedPortfolio],
    ) -> PortfolioComparison {
        let mut comparison = PortfolioComparison::new();
        
        for (i, portfolio) in portfolios.iter().enumerate() {
            comparison.add_portfolio(format!("Portfolio {}", i + 1), portfolio);
        }
        
        comparison
    }
    
    /// Rebalance portfolio to target weights
    pub fn calculate_rebalance_trades(
        &self,
        current_weights: &HashMap<String, Decimal>,
        target_weights: &HashMap<String, Decimal>,
        portfolio_value: Decimal,
    ) -> Vec<RebalanceTrade> {
        let mut trades = Vec::new();
        
        // All unique symbols
        let all_symbols: std::collections::HashSet<_> = current_weights.keys()
            .chain(target_weights.keys())
            .collect();
        
        for symbol in all_symbols {
            let current = current_weights.get(symbol).copied().unwrap_or(Decimal::ZERO);
            let target = target_weights.get(symbol).copied().unwrap_or(Decimal::ZERO);
            
            let delta = target - current;
            
            if delta.abs() > Decimal::try_from(0.001).unwrap() {
                let value = delta * portfolio_value;
                trades.push(RebalanceTrade {
                    symbol: symbol.clone(),
                    current_weight: current,
                    target_weight: target,
                    delta_weight: delta,
                    trade_value: value,
                    action: if delta > Decimal::ZERO {
                        TradeAction::Buy
                    } else {
                        TradeAction::Sell
                    },
                });
            }
        }
        
        trades
    }
    
    /// Set risk-free rate
    pub fn set_risk_free_rate(&mut self, rate: Decimal) {
        self.risk_free_rate = rate;
        self.mpt_optimizer.set_risk_free_rate(rate);
        self.ef_calculator.set_risk_free_rate(rate);
    }
    
    /// Get risk-free rate
    pub fn risk_free_rate(&self) -> Decimal {
        self.risk_free_rate
    }
}

impl Default for PortfolioOptimizationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Portfolio comparison
#[derive(Debug)]
pub struct PortfolioComparison {
    portfolios: Vec<(String, OptimizedPortfolio)>,
}

impl PortfolioComparison {
    /// Create new comparison
    pub fn new() -> Self {
        Self {
            portfolios: Vec::new(),
        }
    }
    
    /// Add portfolio to comparison
    pub fn add_portfolio(&mut self, name: String, portfolio: &OptimizedPortfolio) {
        self.portfolios.push((name, portfolio.clone()));
    }
    
    /// Get best by Sharpe ratio
    pub fn best_by_sharpe(&self) -> Option<&str> {
        self.portfolios.iter()
            .max_by(|a, b| a.1.sharpe_ratio.partial_cmp(&b.1.sharpe_ratio).unwrap())
            .map(|(name, _)| name.as_str())
    }
    
    /// Get best by return
    pub fn best_by_return(&self) -> Option<&str> {
        self.portfolios.iter()
            .max_by(|a, b| a.1.expected_return.partial_cmp(&b.1.expected_return).unwrap())
            .map(|(name, _)| name.as_str())
    }
    
    /// Get lowest risk
    pub fn lowest_risk(&self) -> Option<&str> {
        self.portfolios.iter()
            .min_by(|a, b| a.1.expected_risk.partial_cmp(&b.1.expected_risk).unwrap())
            .map(|(name, _)| name.as_str())
    }
    
    /// Get most diversified
    pub fn most_diversified(&self) -> Option<&str> {
        self.portfolios.iter()
            .min_by(|a, b| a.1.concentration_risk.partial_cmp(&b.1.concentration_risk).unwrap())
            .map(|(name, _)| name.as_str())
    }
    
    /// Generate comparison table
    pub fn to_table(&self) -> String {
        let mut table = String::new();
        table.push_str("| Portfolio | Return | Risk | Sharpe | Diversification |\n");
        table.push_str("|-----------|--------|------|--------|------------------|\n");
        
        for (name, p) in &self.portfolios {
            table.push_str(&format!(
                "| {} | {:.2}% | {:.2}% | {:.2} | {:.2}% |\n",
                name,
                p.expected_return * Decimal::from(100),
                p.expected_risk * Decimal::from(100),
                p.sharpe_ratio,
                p.diversification_ratio * 100.0
            ));
        }
        
        table
    }
}

impl Default for PortfolioComparison {
    fn default() -> Self {
        Self::new()
    }
}

/// Rebalance trade
#[derive(Debug, Clone)]
pub struct RebalanceTrade {
    pub symbol: String,
    pub current_weight: Decimal,
    pub target_weight: Decimal,
    pub delta_weight: Decimal,
    pub trade_value: Decimal,
    pub action: TradeAction,
}

/// Trade action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_asset(symbol: &str, expected_return: f32, risk: f32) -> Asset {
        Asset {
            symbol: symbol.to_string(),
            expected_return: Decimal::try_from(expected_return).unwrap(),
            risk: Decimal::try_from(risk).unwrap(),
            correlations: HashMap::new(),
        }
    }

    #[test]
    fn test_engine_creation() {
        let engine = PortfolioOptimizationEngine::new();
        assert!(engine.risk_free_rate() > Decimal::ZERO);
    }

    #[test]
    fn test_optimized_portfolio_creation() {
        let portfolio = OptimizedPortfolio::new("Test", OptimizationObjective::MaximizeSharpe);
        assert_eq!(portfolio.name, "Test");
        assert!(portfolio.weights.is_empty());
    }

    #[test]
    fn test_portfolio_weights() {
        let mut portfolio = OptimizedPortfolio::new("Test", OptimizationObjective::MaximizeReturn);
        portfolio.weights.insert("AAPL".to_string(), Decimal::try_from(0.5).unwrap());
        portfolio.weights.insert("MSFT".to_string(), Decimal::try_from(0.5).unwrap());
        
        assert_eq!(portfolio.weight("AAPL"), Decimal::try_from(0.5).unwrap());
        assert_eq!(portfolio.weight("GOOGL"), Decimal::ZERO);
        assert!(portfolio.is_fully_invested());
    }

    #[test]
    fn test_concentration_calculation() {
        let mut portfolio = OptimizedPortfolio::new("Test", OptimizationObjective::MaximizeReturn);
        portfolio.weights.insert("AAPL".to_string(), Decimal::try_from(0.5).unwrap());
        portfolio.weights.insert("MSFT".to_string(), Decimal::try_from(0.3).unwrap());
        portfolio.weights.insert("GOOGL".to_string(), Decimal::try_from(0.2).unwrap());
        
        let hhi = portfolio.calculate_concentration();
        // HHI = 0.5^2 + 0.3^2 + 0.2^2 = 0.25 + 0.09 + 0.04 = 0.38
        assert!(hhi > 0.35 && hhi < 0.40);
    }

    #[test]
    fn test_rebalance_calculation() {
        let engine = PortfolioOptimizationEngine::new();
        
        let mut current = HashMap::new();
        current.insert("AAPL".to_string(), Decimal::try_from(0.4).unwrap());
        current.insert("MSFT".to_string(), Decimal::try_from(0.6).unwrap());
        
        let mut target = HashMap::new();
        target.insert("AAPL".to_string(), Decimal::try_from(0.5).unwrap());
        target.insert("MSFT".to_string(), Decimal::try_from(0.5).unwrap());
        
        let trades = engine.calculate_rebalance_trades(
            &current,
            &target,
            Decimal::from(100000),
        );
        
        assert_eq!(trades.len(), 2);
        
        let aapl_trade = trades.iter().find(|t| t.symbol == "AAPL").unwrap();
        assert_eq!(aapl_trade.action, TradeAction::Buy);
        assert_eq!(aapl_trade.trade_value, Decimal::from(10000));
    }

    #[test]
    fn test_portfolio_comparison() {
        let mut comp = PortfolioComparison::new();
        
        let mut p1 = OptimizedPortfolio::new("P1", OptimizationObjective::MaximizeReturn);
        p1.sharpe_ratio = 1.5;
        p1.expected_return = Decimal::try_from(0.15).unwrap();
        
        let mut p2 = OptimizedPortfolio::new("P2", OptimizationObjective::MinimizeRisk);
        p2.sharpe_ratio = 1.2;
        p2.expected_return = Decimal::try_from(0.10).unwrap();
        
        comp.add_portfolio("Aggressive".to_string(), &p1);
        comp.add_portfolio("Conservative".to_string(), &p2);
        
        assert_eq!(comp.best_by_sharpe(), Some("Aggressive"));
        assert_eq!(comp.best_by_return(), Some("Aggressive"));
    }

    #[test]
    fn test_optimization_constraints_default() {
        let constraints = OptimizationConstraints::default();
        assert_eq!(constraints.min_weight, Decimal::ZERO);
        assert_eq!(constraints.max_weight, Decimal::from(1));
        assert!(!constraints.allow_short);
    }
}
