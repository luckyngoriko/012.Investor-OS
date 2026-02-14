//! Black-Litterman Model
//!
//! Bayesian approach combining market equilibrium with investor views

use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::info;

use super::{Asset, OptimizationConstraints, OptimizationError, OptimizationObjective, OptimizedPortfolio, Result};

/// Black-Litterman model
#[derive(Debug)]
pub struct BlackLittermanModel {
    tau: f32, // Scaling factor (typically 0.025-0.05)
    risk_aversion: f32,
}

/// Investor view
#[derive(Debug, Clone)]
pub struct InvestorView {
    pub id: uuid::Uuid,
    pub description: String,
    pub assets: Vec<String>, // Affected assets
    pub relative_view: bool, // true = relative view (AAPL vs MSFT), false = absolute
    pub expected_return: Decimal,
    pub confidence: ViewConfidence,
}

/// View confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewConfidence {
    VeryLow,    // 20%
    Low,        // 40%
    Medium,     // 60%
    High,       // 80%
    VeryHigh,   // 95%
}

impl ViewConfidence {
    pub fn as_f32(&self) -> f32 {
        match self {
            ViewConfidence::VeryLow => 0.20,
            ViewConfidence::Low => 0.40,
            ViewConfidence::Medium => 0.60,
            ViewConfidence::High => 0.80,
            ViewConfidence::VeryHigh => 0.95,
        }
    }
    
    pub fn uncertainty(&self) -> f32 {
        1.0 - self.as_f32()
    }
}

impl BlackLittermanModel {
    /// Create new model
    pub fn new() -> Self {
        Self {
            tau: 0.025, // Standard value
            risk_aversion: 2.5,
        }
    }
    
    /// Set tau parameter
    pub fn set_tau(&mut self, tau: f32) {
        self.tau = tau.clamp(0.001, 0.5);
    }
    
    /// Set risk aversion
    pub fn set_risk_aversion(&mut self, lambda: f32) {
        self.risk_aversion = lambda.max(0.1);
    }
    
    /// Optimize using Black-Litterman
    pub fn optimize(
        &self,
        assets: &[Asset],
        market_caps: &HashMap<String, Decimal>,
        views: &[InvestorView],
        constraints: OptimizationConstraints,
    ) -> Result<OptimizedPortfolio> {
        if assets.is_empty() {
            return Err(OptimizationError::InsufficientData(
                "No assets provided".to_string()
            ));
        }
        
        // Step 1: Calculate market equilibrium returns (reverse optimization)
        let equilibrium_returns = self.calculate_equilibrium_returns(assets, market_caps)?;
        
        // Step 2: Apply investor views
        let blended_returns = if views.is_empty() {
            equilibrium_returns
        } else {
            self.apply_views(assets, &equilibrium_returns, views)?
        };
        
        // Step 3: Optimize portfolio using blended returns
        let weights = self.optimize_weights(assets, &blended_returns, &constraints)?;
        
        // Build portfolio
        let portfolio = self.build_portfolio(assets, &weights, &blended_returns)?;
        
        info!(
            "Black-Litterman optimization complete: {} assets, {} views",
            assets.len(),
            views.len()
        );
        
        Ok(portfolio)
    }
    
    /// Calculate equilibrium returns from market caps
    fn calculate_equilibrium_returns(
        &self,
        assets: &[Asset],
        market_caps: &HashMap<String, Decimal>,
    ) -> Result<Vec<Decimal>> {
        // Market cap weights
        let total_cap: Decimal = assets.iter()
            .map(|a| market_caps.get(&a.symbol).copied().unwrap_or(Decimal::ZERO))
            .sum();
        
        if total_cap <= Decimal::ZERO {
            // Fallback to equal weights
            return Ok(assets.iter()
                .map(|a| a.expected_return)
                .collect());
        }
        
        let weights: Vec<Decimal> = assets.iter()
            .map(|a| {
                let cap = market_caps.get(&a.symbol).copied().unwrap_or(Decimal::ZERO);
                cap / total_cap
            })
            .collect();
        
        // Build covariance matrix
        let cov_matrix = self.build_covariance_matrix(assets);
        
        // Calculate implied returns: PI = delta * Sigma * w
        // where delta is risk aversion, Sigma is covariance, w is market weights
        let implied_returns: Vec<Decimal> = (0..assets.len())
            .map(|i| {
                let variance_contrib: f32 = (0..assets.len())
                    .map(|j| {
                        let wj: f32 = weights[j].try_into().unwrap_or(0.0f32);
                        cov_matrix[i][j] * wj
                    })
                    .sum();
                
                let implied = self.risk_aversion * variance_contrib;
                Decimal::try_from(implied).unwrap_or(Decimal::ZERO)
            })
            .collect();
        
        Ok(implied_returns)
    }
    
    /// Apply investor views to equilibrium returns
    fn apply_views(
        &self,
        assets: &[Asset],
        equilibrium_returns: &[Decimal],
        views: &[InvestorView],
    ) -> Result<Vec<Decimal>> {
        // Simplified view application: blend equilibrium with views
        let mut blended = equilibrium_returns.to_vec();
        
        for view in views {
            let confidence = view.confidence.as_f32();
            let view_weight = confidence;
            let eq_weight = 1.0 - view_weight;
            
            if view.relative_view {
                // Relative view: apply to differential
                if view.assets.len() >= 2 {
                    let asset1_idx = assets.iter()
                        .position(|a| a.symbol == view.assets[0]);
                    let asset2_idx = assets.iter()
                        .position(|a| a.symbol == view.assets[1]);
                    
                    if let (Some(i), Some(j)) = (asset1_idx, asset2_idx) {
                        // Adjust returns to reflect view
                        let view_return: f32 = view.expected_return.try_into().unwrap_or(0.0f32);
                        let adjustment = Decimal::try_from(view_weight * view_return).unwrap();
                        
                        blended[i] = blended[i] * Decimal::try_from(eq_weight).unwrap() + adjustment;
                        blended[j] = blended[j] * Decimal::try_from(eq_weight).unwrap() - adjustment;
                    }
                }
            } else {
                // Absolute view
                for symbol in &view.assets {
                    if let Some(idx) = assets.iter().position(|a| a.symbol == *symbol) {
                        let view_return: f32 = view.expected_return.try_into().unwrap_or(0.0f32);
                        let view_decimal = Decimal::try_from(view_return).unwrap();
                        
                        blended[idx] = blended[idx] * Decimal::try_from(eq_weight).unwrap()
                            + view_decimal * Decimal::try_from(view_weight).unwrap();
                    }
                }
            }
        }
        
        Ok(blended)
    }
    
    /// Optimize portfolio weights
    fn optimize_weights(
        &self,
        assets: &[Asset],
        returns: &[Decimal],
        constraints: &OptimizationConstraints,
    ) -> Result<Vec<Decimal>> {
        // Simplified mean-variance optimization
        let n = assets.len();
        let _cov_matrix = self.build_covariance_matrix(assets);
        
        // Risk-adjusted returns
        let risk_adjusted: Vec<f32> = returns.iter()
            .zip(assets.iter())
            .map(|(r, a)| {
                let ret: f32 = (*r).try_into().unwrap_or(0.0f32);
                let risk: f32 = a.risk.try_into().unwrap_or(0.001);
                ret / (risk * risk)
            })
            .collect();
        
        // Normalize to get weights
        let total: f32 = risk_adjusted.iter().sum();
        
        let mut weights: Vec<Decimal> = if total > 0.0 {
            risk_adjusted.iter()
                .map(|w| Decimal::try_from(w / total).unwrap_or(Decimal::ZERO))
                .collect()
        } else {
            // Equal weight fallback
            let equal = 1.0 / n as f32;
            (0..n).map(|_| Decimal::try_from(equal).unwrap_or(Decimal::ZERO)).collect()
        };
        
        // Apply constraints
        self.apply_constraints(&mut weights, constraints);
        
        // Renormalize
        let total_weight: Decimal = weights.iter().sum();
        if total_weight > Decimal::ZERO {
            for w in weights.iter_mut() {
                *w /= total_weight;
            }
        }
        
        Ok(weights)
    }
    
    /// Build covariance matrix
    fn build_covariance_matrix(&self, assets: &[Asset]) -> Vec<Vec<f32>> {
        let n = assets.len();
        let mut matrix = vec![vec![0.0f32; n]; n];
        
        for (i, asset_i) in assets.iter().enumerate() {
            for (j, asset_j) in assets.iter().enumerate() {
                if i == j {
                    let risk: f32 = asset_i.risk.try_into().unwrap_or(0.0f32);
                    matrix[i][j] = risk * risk;
                } else {
                    let corr = asset_i.correlation(&asset_j.symbol);
                    let risk_i: f32 = asset_i.risk.try_into().unwrap_or(0.0f32);
                    let risk_j: f32 = asset_j.risk.try_into().unwrap_or(0.0f32);
                    matrix[i][j] = corr * risk_i * risk_j;
                }
            }
        }
        
        matrix
    }
    
    /// Apply constraints
    fn apply_constraints(&self, weights: &mut [Decimal], constraints: &OptimizationConstraints) {
        for w in weights.iter_mut() {
            // Min/max weight
            if *w < constraints.min_weight {
                *w = constraints.min_weight;
            }
            if *w > constraints.max_weight {
                *w = constraints.max_weight;
            }
            
            // No short selling
            if !constraints.allow_short && *w < Decimal::ZERO {
                *w = Decimal::ZERO;
            }
        }
    }
    
    /// Build portfolio
    fn build_portfolio(
        &self,
        assets: &[Asset],
        weights: &[Decimal],
        returns: &[Decimal],
    ) -> Result<OptimizedPortfolio> {
        let expected_return: Decimal = weights.iter()
            .zip(returns.iter())
            .map(|(w, r)| w * r)
            .sum();
        
        let cov_matrix = self.build_covariance_matrix(assets);
        let portfolio_variance: f32 = weights.iter().enumerate()
            .map(|(i, wi)| {
                let wi_f32: f32 = (*wi).try_into().unwrap_or(0.0f32);
                weights.iter().enumerate()
                    .map(|(j, wj)| {
                        let wj_f32: f32 = (*wj).try_into().unwrap_or(0.0f32);
                        wi_f32 * wj_f32 * cov_matrix[i][j]
                    })
                    .sum::<f32>()
            })
            .sum();
        
        let portfolio_risk = portfolio_variance.sqrt().max(0.0);
        let expected_risk = Decimal::try_from(portfolio_risk).unwrap_or(Decimal::ZERO);
        
        let mut portfolio = OptimizedPortfolio::new(
            "Black-Litterman",
            OptimizationObjective::MaximizeSharpe,
        );
        
        portfolio.expected_return = expected_return;
        portfolio.expected_risk = expected_risk;
        
        for (i, asset) in assets.iter().enumerate() {
            if weights[i] > Decimal::ZERO {
                portfolio.weights.insert(asset.symbol.clone(), weights[i]);
            }
        }
        
        Ok(portfolio)
    }
    
    /// Create absolute view
    pub fn create_absolute_view(
        asset: &str,
        expected_return: Decimal,
        confidence: ViewConfidence,
    ) -> InvestorView {
        InvestorView {
            id: uuid::Uuid::new_v4(),
            description: format!("{} will return {}%", asset, expected_return * Decimal::from(100)),
            assets: vec![asset.to_string()],
            relative_view: false,
            expected_return,
            confidence,
        }
    }
    
    /// Create relative view
    pub fn create_relative_view(
        outperforming: &str,
        underperforming: &str,
        spread: Decimal,
        confidence: ViewConfidence,
    ) -> InvestorView {
        InvestorView {
            id: uuid::Uuid::new_v4(),
            description: format!("{} will outperform {} by {}%", 
                outperforming, underperforming, spread * Decimal::from(100)),
            assets: vec![outperforming.to_string(), underperforming.to_string()],
            relative_view: true,
            expected_return: spread,
            confidence,
        }
    }
}

impl Default for BlackLittermanModel {
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
    fn test_model_creation() {
        let model = BlackLittermanModel::new();
        assert_eq!(model.tau, 0.025);
        assert_eq!(model.risk_aversion, 2.5);
    }

    #[test]
    fn test_view_confidence() {
        assert!((ViewConfidence::VeryLow.as_f32() - 0.20).abs() < 0.001);
        assert!((ViewConfidence::VeryHigh.as_f32() - 0.95).abs() < 0.001);
        assert!((ViewConfidence::Medium.uncertainty() - 0.40).abs() < 0.001);
    }

    #[test]
    fn test_create_absolute_view() {
        let view = BlackLittermanModel::create_absolute_view(
            "AAPL",
            Decimal::try_from(0.15).unwrap(),
            ViewConfidence::High,
        );
        
        assert_eq!(view.assets, vec!["AAPL"]);
        assert!(!view.relative_view);
        assert_eq!(view.confidence, ViewConfidence::High);
    }

    #[test]
    fn test_create_relative_view() {
        let view = BlackLittermanModel::create_relative_view(
            "AAPL",
            "MSFT",
            Decimal::try_from(0.05).unwrap(),
            ViewConfidence::Medium,
        );
        
        assert_eq!(view.assets, vec!["AAPL", "MSFT"]);
        assert!(view.relative_view);
    }

    #[test]
    fn test_equilibrium_returns() {
        let model = BlackLittermanModel::new();
        
        let a = create_test_asset("A", 0.10, 0.15);
        let b = create_test_asset("B", 0.08, 0.10);
        
        let assets = vec![a, b];
        
        let mut market_caps = HashMap::new();
        market_caps.insert("A".to_string(), Decimal::from(600));
        market_caps.insert("B".to_string(), Decimal::from(400));
        
        let returns = model.calculate_equilibrium_returns(&assets, &market_caps).unwrap();
        
        assert_eq!(returns.len(), 2);
        // Both should have positive implied returns
        assert!(returns[0] > Decimal::ZERO);
        assert!(returns[1] > Decimal::ZERO);
    }

    #[test]
    fn test_optimize_without_views() {
        let model = BlackLittermanModel::new();
        
        let a = create_test_asset("AAPL", 0.12, 0.20);
        let b = create_test_asset("MSFT", 0.10, 0.18);
        
        let assets = vec![a, b];
        
        let mut market_caps = HashMap::new();
        market_caps.insert("AAPL".to_string(), Decimal::from(1000));
        market_caps.insert("MSFT".to_string(), Decimal::from(1000));
        
        let result = model.optimize(&assets, &market_caps, &[], OptimizationConstraints::default());
        
        assert!(result.is_ok());
        let portfolio = result.unwrap();
        assert!(portfolio.is_fully_invested());
    }

    #[test]
    fn test_optimize_with_views() {
        let model = BlackLittermanModel::new();
        
        let a = create_test_asset("AAPL", 0.12, 0.20);
        let b = create_test_asset("MSFT", 0.10, 0.18);
        
        let assets = vec![a, b];
        
        let mut market_caps = HashMap::new();
        market_caps.insert("AAPL".to_string(), Decimal::from(1000));
        market_caps.insert("MSFT".to_string(), Decimal::from(1000));
        
        let view = BlackLittermanModel::create_absolute_view(
            "AAPL",
            Decimal::try_from(0.20).unwrap(),
            ViewConfidence::High,
        );
        
        let result = model.optimize(&assets, &market_caps, &[view], OptimizationConstraints::default());
        
        assert!(result.is_ok());
        let portfolio = result.unwrap();
        
        // With positive view on AAPL, should have higher weight
        let aapl_weight = portfolio.weight("AAPL");
        let msft_weight = portfolio.weight("MSFT");
        
        assert!(aapl_weight > msft_weight);
    }

    #[test]
    fn test_insufficient_data() {
        let model = BlackLittermanModel::new();
        let assets: Vec<Asset> = vec![];
        
        let result = model.optimize(&assets, &HashMap::new(), &[], OptimizationConstraints::default());
        
        assert!(result.is_err());
    }
}
