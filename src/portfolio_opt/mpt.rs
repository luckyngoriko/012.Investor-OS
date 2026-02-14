//! Modern Portfolio Theory (Markowitz Optimization)
//!
//! Mean-variance optimization for portfolio allocation

use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::info;

use super::{OptimizationConstraints, OptimizationError, OptimizationObjective, OptimizedPortfolio, Result};

/// Asset for optimization
#[derive(Debug, Clone)]
pub struct Asset {
    pub symbol: String,
    pub expected_return: Decimal,
    pub risk: Decimal, // Standard deviation
    pub correlations: HashMap<String, f32>, // Correlation with other assets
}

impl Asset {
    /// Create new asset
    pub fn new(symbol: &str, expected_return: Decimal, risk: Decimal) -> Self {
        Self {
            symbol: symbol.to_string(),
            expected_return,
            risk,
            correlations: HashMap::new(),
        }
    }
    
    /// Set correlation with another asset
    pub fn set_correlation(&mut self, other_symbol: &str, correlation: f32) {
        self.correlations.insert(other_symbol.to_string(), correlation.clamp(-1.0, 1.0));
    }
    
    /// Get correlation (defaults to 0 if not set)
    pub fn correlation(&self, other_symbol: &str) -> f32 {
        if other_symbol == self.symbol {
            1.0
        } else {
            self.correlations.get(other_symbol).copied().unwrap_or(0.0)
        }
    }
    
    /// Calculate Sharpe ratio
    pub fn sharpe_ratio(&self, risk_free_rate: Decimal) -> f32 {
        let excess_return = self.expected_return - risk_free_rate;
        if self.risk > Decimal::ZERO {
            let ratio: f32 = (excess_return / self.risk).try_into().unwrap_or(0.0f32);
            ratio
        } else {
            0.0
        }
    }
}

/// Markowitz optimizer
#[derive(Debug)]
pub struct MarkowitzOptimizer {
    risk_free_rate: Decimal,
    max_iterations: usize,
    convergence_threshold: f32,
}

/// Portfolio statistics
#[derive(Debug, Clone)]
pub struct PortfolioStats {
    pub expected_return: Decimal,
    pub expected_risk: Decimal,
    pub variance: Decimal,
    pub sharpe_ratio: f32,
}

impl MarkowitzOptimizer {
    /// Create new optimizer
    pub fn new() -> Self {
        Self {
            risk_free_rate: Decimal::try_from(0.02).unwrap(),
            max_iterations: 1000,
            convergence_threshold: 0.0001,
        }
    }
    
    /// Set risk-free rate
    pub fn set_risk_free_rate(&mut self, rate: Decimal) {
        self.risk_free_rate = rate;
    }
    
    /// Optimize portfolio
    pub fn optimize(
        &self,
        assets: &[Asset],
        objective: OptimizationObjective,
        constraints: OptimizationConstraints,
    ) -> Result<OptimizedPortfolio> {
        if assets.is_empty() {
            return Err(OptimizationError::InsufficientData(
                "No assets provided".to_string()
            ));
        }
        
        if assets.len() < constraints.min_assets {
            return Err(OptimizationError::InvalidConstraints(
                format!("Need at least {} assets", constraints.min_assets)
            ));
        }
        
        // Build covariance matrix
        let cov_matrix = self.build_covariance_matrix(assets);
        
        // Initial equal weights
        let n = assets.len();
        let equal_weight = Decimal::try_from(1.0 / n as f64).unwrap();
        let mut weights: Vec<Decimal> = vec![equal_weight; n];
        
        // Optimize based on objective
        let portfolio = match objective {
            OptimizationObjective::MaximizeReturn => {
                self.optimize_max_return(assets, &mut weights, &constraints)?
            }
            OptimizationObjective::MinimizeRisk => {
                self.optimize_min_risk(assets, &cov_matrix, &mut weights, &constraints)?
            }
            OptimizationObjective::MaximizeSharpe => {
                self.optimize_max_sharpe(assets, &cov_matrix, &mut weights, &constraints)?
            }
            _ => {
                return Err(OptimizationError::InvalidConstraints(
                    "Objective not supported by MPT".to_string()
                ));
            }
        };
        
        info!(
            "MPT optimization complete: {} assets, objective={:?}",
            assets.len(), objective
        );
        
        Ok(portfolio)
    }
    
    /// Build covariance matrix from correlations and risks
    fn build_covariance_matrix(&self, assets: &[Asset]) -> Vec<Vec<f32>> {
        let n = assets.len();
        let mut matrix = vec![vec![0.0f32; n]; n];
        
        for (i, asset_i) in assets.iter().enumerate() {
            for (j, asset_j) in assets.iter().enumerate() {
                if i == j {
                    // Variance = risk^2
                    let risk: f32 = asset_i.risk.try_into().unwrap_or(0.0f32);
                    matrix[i][j] = risk * risk;
                } else {
                    // Covariance = correlation * sigma_i * sigma_j
                    let corr = asset_i.correlation(&asset_j.symbol);
                    let risk_i: f32 = asset_i.risk.try_into().unwrap_or(0.0f32);
                    let risk_j: f32 = asset_j.risk.try_into().unwrap_or(0.0f32);
                    matrix[i][j] = corr * risk_i * risk_j;
                }
            }
        }
        
        matrix
    }
    
    /// Calculate portfolio stats
    fn calculate_portfolio_stats(
        &self,
        assets: &[Asset],
        weights: &[Decimal],
        cov_matrix: &[Vec<f32>],
    ) -> PortfolioStats {
        // Expected return = sum(w_i * r_i)
        let expected_return: Decimal = assets.iter()
            .zip(weights.iter())
            .map(|(a, w)| a.expected_return * w)
            .sum();
        
        // Portfolio variance = w^T * Cov * w
        let mut variance = Decimal::ZERO;
        for (i, wi) in weights.iter().enumerate() {
            for (j, wj) in weights.iter().enumerate() {
                let cov = Decimal::try_from(cov_matrix[i][j]).unwrap_or(Decimal::ZERO);
                variance += wi * wj * cov;
            }
        }
        
        // Risk = sqrt(variance)
        let variance_f32: f32 = variance.try_into().unwrap_or(0.0f32);
        let risk_f32 = variance_f32.sqrt().max(0.0);
        let expected_risk = Decimal::try_from(risk_f32).unwrap_or(Decimal::ZERO);
        
        // Sharpe ratio
        let excess_return = expected_return - self.risk_free_rate;
        let sharpe = if expected_risk > Decimal::ZERO {
            let ratio: f32 = (excess_return / expected_risk).try_into().unwrap_or(0.0f32);
            ratio
        } else {
            0.0
        };
        
        PortfolioStats {
            expected_return,
            expected_risk,
            variance,
            sharpe_ratio: sharpe,
        }
    }
    
    /// Optimize for maximum return (within risk constraints)
    fn optimize_max_return(
        &self,
        assets: &[Asset],
        weights: &mut [Decimal],
        constraints: &OptimizationConstraints,
    ) -> Result<OptimizedPortfolio> {
        // Simple approach: allocate to highest returning assets
        let mut sorted: Vec<_> = assets.iter().enumerate()
            .map(|(i, a)| (i, a.expected_return))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Allocate to top performers
        let max_assets = constraints.max_assets.min(assets.len());
        let weight_per_asset = Decimal::try_from(1.0 / max_assets as f64).unwrap();
        
        for weight in weights.iter_mut() {
            *weight = Decimal::ZERO;
        }
        
        for (idx, _) in sorted.iter().take(max_assets) {
            weights[*idx] = weight_per_asset;
        }
        
        self.build_portfolio(assets, weights, OptimizationObjective::MaximizeReturn)
    }
    
    /// Optimize for minimum risk
    fn optimize_min_risk(
        &self,
        assets: &[Asset],
        _cov_matrix: &[Vec<f32>],
        weights: &mut [Decimal],
        constraints: &OptimizationConstraints,
    ) -> Result<OptimizedPortfolio> {
        // Simplified minimum variance: equal weight low-risk assets
        let avg_risk: Decimal = assets.iter()
            .map(|a| a.risk)
            .sum::<Decimal>() / Decimal::from(assets.len() as i64);
        
        let low_risk_assets: Vec<_> = assets.iter()
            .enumerate()
            .filter(|(_, a)| a.risk <= avg_risk)
            .map(|(i, _)| i)
            .collect();
        
        let n = low_risk_assets.len().max(1);
        let weight = Decimal::try_from(1.0 / n as f64).unwrap();
        
        for w in weights.iter_mut() {
            *w = Decimal::ZERO;
        }
        
        for idx in low_risk_assets.iter().take(constraints.max_assets) {
            weights[*idx] = weight;
        }
        
        self.build_portfolio(assets, weights, OptimizationObjective::MinimizeRisk)
    }
    
    /// Optimize for maximum Sharpe ratio
    fn optimize_max_sharpe(
        &self,
        assets: &[Asset],
        _cov_matrix: &[Vec<f32>],
        weights: &mut [Decimal],
        constraints: &OptimizationConstraints,
    ) -> Result<OptimizedPortfolio> {
        // Rank by individual Sharpe ratios
        let mut sorted: Vec<_> = assets.iter().enumerate()
            .map(|(i, a)| (i, a.sharpe_ratio(self.risk_free_rate)))
            .collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Allocate to top Sharpe assets
        let max_assets = constraints.max_assets.min(assets.len());
        let weight_per_asset = Decimal::try_from(1.0 / max_assets as f64).unwrap();
        
        for w in weights.iter_mut() {
            *w = Decimal::ZERO;
        }
        
        for (idx, _) in sorted.iter().take(max_assets) {
            weights[*idx] = weight_per_asset;
        }
        
        self.build_portfolio(assets, weights, OptimizationObjective::MaximizeSharpe)
    }
    
    /// Build portfolio from weights
    fn build_portfolio(
        &self,
        assets: &[Asset],
        weights: &[Decimal],
        objective: OptimizationObjective,
    ) -> Result<OptimizedPortfolio> {
        let cov_matrix = self.build_covariance_matrix(assets);
        let stats = self.calculate_portfolio_stats(assets, weights, &cov_matrix);
        
        let mut portfolio = OptimizedPortfolio::new(
            &format!("MPT {:?}", objective),
            objective,
        );
        
        portfolio.expected_return = stats.expected_return;
        portfolio.expected_risk = stats.expected_risk;
        portfolio.sharpe_ratio = stats.sharpe_ratio;
        portfolio.concentration_risk = self.calculate_concentration(weights);
        portfolio.diversification_ratio = self.calculate_diversification_ratio(
            assets, weights, &cov_matrix
        );
        
        for (i, asset) in assets.iter().enumerate() {
            if weights[i] > Decimal::ZERO {
                portfolio.weights.insert(asset.symbol.clone(), weights[i]);
            }
        }
        
        Ok(portfolio)
    }
    
    /// Calculate concentration (HHI)
    fn calculate_concentration(&self, weights: &[Decimal]) -> f32 {
        weights.iter()
            .map(|w| {
                let f: f32 = (*w).try_into().unwrap_or(0.0f32);
                f * f
            })
            .sum()
    }
    
    /// Calculate diversification ratio
    fn calculate_diversification_ratio(
        &self,
        assets: &[Asset],
        weights: &[Decimal],
        cov_matrix: &[Vec<f32>],
    ) -> f32 {
        // DR = sum(w_i * sigma_i) / portfolio_sigma
        let weighted_sum: Decimal = assets.iter()
            .zip(weights.iter())
            .map(|(a, w)| a.risk * w)
            .sum();
        
        let portfolio_variance = self.calculate_portfolio_variance(weights, cov_matrix);
        let portfolio_risk = portfolio_variance.sqrt();
        
        if portfolio_risk > 0.0 {
            let weighted_f32: f32 = weighted_sum.try_into().unwrap_or(0.0f32);
            weighted_f32 / portfolio_risk
        } else {
            1.0
        }
    }
    
    /// Calculate portfolio variance
    fn calculate_portfolio_variance(
        &self,
        weights: &[Decimal],
        cov_matrix: &[Vec<f32>],
    ) -> f32 {
        let mut variance = 0.0f32;
        
        for (i, wi) in weights.iter().enumerate() {
            let wi_f32: f32 = (*wi).try_into().unwrap_or(0.0f32);
            for (j, wj) in weights.iter().enumerate() {
                let wj_f32: f32 = (*wj).try_into().unwrap_or(0.0f32);
                variance += wi_f32 * wj_f32 * cov_matrix[i][j];
            }
        }
        
        variance.max(0.0)
    }
}

impl Default for MarkowitzOptimizer {
    fn default() -> Self {
        Self::new()
    }
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
    fn test_asset_creation() {
        let asset = create_test_asset("AAPL", 0.12, 0.20);
        assert_eq!(asset.symbol, "AAPL");
        assert_eq!(asset.expected_return, Decimal::try_from(0.12).unwrap());
    }

    #[test]
    fn test_correlation() {
        let mut aapl = create_test_asset("AAPL", 0.12, 0.20);
        aapl.set_correlation("MSFT", 0.7);
        
        assert_eq!(aapl.correlation("MSFT"), 0.7);
        assert_eq!(aapl.correlation("AAPL"), 1.0); // Self
        assert_eq!(aapl.correlation("GOOGL"), 0.0); // Default
    }

    #[test]
    fn test_sharpe_ratio() {
        let asset = create_test_asset("AAPL", 0.12, 0.20);
        let risk_free = Decimal::try_from(0.02).unwrap();
        
        // Sharpe = (0.12 - 0.02) / 0.20 = 0.5
        let sharpe = asset.sharpe_ratio(risk_free);
        assert!(sharpe > 0.49 && sharpe < 0.51);
    }

    #[test]
    fn test_optimizer_creation() {
        let opt = MarkowitzOptimizer::new();
        assert_eq!(opt.max_iterations, 1000);
    }

    #[test]
    fn test_covariance_matrix() {
        let mut aapl = create_test_asset("AAPL", 0.12, 0.20);
        let msft = create_test_asset("MSFT", 0.10, 0.18);
        
        aapl.set_correlation("MSFT", 0.5);
        
        let opt = MarkowitzOptimizer::new();
        let matrix = opt.build_covariance_matrix(&[aapl, msft]);
        
        // Variance on diagonal (use approximate equality for floating point)
        assert!((matrix[0][0] - 0.04).abs() < 0.0001); // 0.20^2
        assert!((matrix[1][1] - 0.0324).abs() < 0.0001); // 0.18^2
        
        // Covariance off-diagonal
        // cov = corr * sigma_i * sigma_j = 0.5 * 0.20 * 0.18 = 0.018
        assert!(matrix[0][1] > 0.017 && matrix[0][1] < 0.019);
    }

    #[test]
    fn test_optimize_max_return() {
        let opt = MarkowitzOptimizer::new();
        
        let assets = vec![
            create_test_asset("AAPL", 0.15, 0.25),
            create_test_asset("MSFT", 0.10, 0.20),
            create_test_asset("GOOGL", 0.12, 0.22),
        ];
        
        let constraints = OptimizationConstraints::default();
        let result = opt.optimize(&assets, OptimizationObjective::MaximizeReturn, constraints);
        
        assert!(result.is_ok());
        let portfolio = result.unwrap();
        assert!(portfolio.expected_return > Decimal::ZERO);
    }

    #[test]
    fn test_optimize_min_risk() {
        let opt = MarkowitzOptimizer::new();
        
        let assets = vec![
            create_test_asset("AAPL", 0.15, 0.15),
            create_test_asset("MSFT", 0.10, 0.20),
            create_test_asset("GOOGL", 0.12, 0.25),
        ];
        
        let constraints = OptimizationConstraints::default();
        let result = opt.optimize(&assets, OptimizationObjective::MinimizeRisk, constraints);
        
        assert!(result.is_ok());
        let portfolio = result.unwrap();
        assert!(portfolio.expected_risk > Decimal::ZERO);
    }

    #[test]
    fn test_portfolio_stats() {
        let opt = MarkowitzOptimizer::new();
        
        let aapl = create_test_asset("AAPL", 0.12, 0.20);
        let weights = vec![Decimal::from(1)];
        let cov_matrix = opt.build_covariance_matrix(&[aapl.clone()]);
        
        let stats = opt.calculate_portfolio_stats(&[aapl], &weights, &cov_matrix);
        
        assert_eq!(stats.expected_return, Decimal::try_from(0.12).unwrap());
        assert!(stats.sharpe_ratio > 0.0);
    }

    #[test]
    fn test_insufficient_data() {
        let opt = MarkowitzOptimizer::new();
        let assets: Vec<Asset> = vec![];
        
        let result = opt.optimize(&assets, OptimizationObjective::MaximizeReturn, OptimizationConstraints::default());
        
        assert!(result.is_err());
    }
}
