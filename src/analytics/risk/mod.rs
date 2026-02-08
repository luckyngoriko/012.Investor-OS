//! Risk Metrics
//!
//! S7-D2: Risk analytics (VaR, Sharpe, drawdown)

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use tracing::warn;

use crate::analytics::{AnalyticsError, Result};

/// Risk analyzer
pub struct RiskAnalyzer {
    returns: Vec<Decimal>,
    risk_free_rate: Decimal, // Annual rate
}

/// Risk metrics result
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub var_95: Decimal,          // Value at Risk (95% confidence)
    pub var_99: Decimal,          // Value at Risk (99% confidence)
    pub cvar_95: Decimal,         // Conditional VaR / Expected Shortfall
    pub sharpe_ratio: Decimal,    // Sharpe ratio
    pub sortino_ratio: Decimal,   // Sortino ratio
    pub max_drawdown: Decimal,    // Maximum drawdown
    pub calmar_ratio: Decimal,    // Calmar ratio
    pub volatility: Decimal,      // Annualized volatility
    pub downside_deviation: Decimal,
    pub beta: Option<Decimal>,    // Beta to market
    pub alpha: Option<Decimal>,   // Jensen's alpha
    pub information_ratio: Option<Decimal>,
}

impl RiskAnalyzer {
    /// Create a new risk analyzer
    pub fn new(returns: Vec<Decimal>, risk_free_rate: Decimal) -> Self {
        Self {
            returns,
            risk_free_rate,
        }
    }

    /// Calculate all risk metrics
    pub fn calculate_all(&self) -> Result<RiskMetrics> {
        if self.returns.len() < 30 {
            return Err(AnalyticsError::InsufficientData(
                "Need at least 30 returns for risk calculation".to_string()
            ));
        }

        Ok(RiskMetrics {
            var_95: self.var(Decimal::from(95) / Decimal::from(100)),
            var_99: self.var(Decimal::from(99) / Decimal::from(100)),
            cvar_95: self.cvar(Decimal::from(95) / Decimal::from(100)),
            sharpe_ratio: self.sharpe_ratio(),
            sortino_ratio: self.sortino_ratio(),
            max_drawdown: self.max_drawdown(),
            calmar_ratio: self.calmar_ratio(),
            volatility: self.volatility(),
            downside_deviation: self.downside_deviation(),
            beta: None, // Would need market returns
            alpha: None,
            information_ratio: None,
        })
    }

    /// Value at Risk (historical simulation)
    /// 
    /// VaR at confidence level (e.g., 0.95) represents the potential loss
    /// that will not be exceeded with the given confidence level.
    pub fn var(&self, confidence: Decimal) -> Decimal {
        if self.returns.is_empty() {
            return Decimal::ZERO;
        }

        let mut sorted_returns = self.returns.clone();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Index for the percentile
        let index = ((Decimal::ONE - confidence) * Decimal::from(sorted_returns.len() as i32))
            .try_into()
            .unwrap_or(0i64) as usize;
        
        let idx = index.min(sorted_returns.len() - 1);
        
        // VaR is typically expressed as a positive number (loss)
        -sorted_returns[idx]
    }

    /// Conditional VaR (Expected Shortfall)
    /// 
    /// CVaR is the average of returns worse than VaR.
    pub fn cvar(&self, confidence: Decimal) -> Decimal {
        if self.returns.is_empty() {
            return Decimal::ZERO;
        }

        let var_threshold = -self.var(confidence); // Convert back to return

        let tail_returns: Vec<Decimal> = self.returns
            .iter()
            .filter(|r| **r <= var_threshold)
            .copied()
            .collect();

        if tail_returns.is_empty() {
            return Decimal::ZERO;
        }

        // Average of tail returns (as positive loss)
        -(tail_returns.iter().sum::<Decimal>() / Decimal::from(tail_returns.len() as i32))
    }

    /// Sharpe ratio
    /// 
    /// Measures risk-adjusted return. Higher is better.
    /// Formula: (Mean Return - Risk Free Rate) / Standard Deviation
    pub fn sharpe_ratio(&self) -> Decimal {
        let mean_return = self.mean_return();
        let volatility = self.volatility();

        if volatility == Decimal::ZERO {
            return Decimal::ZERO;
        }

        // Annualize (assuming daily returns)
        let annual_mean = mean_return * Decimal::from(252);
        let annual_vol = volatility;

        (annual_mean - self.risk_free_rate) / annual_vol
    }

    /// Sortino ratio
    /// 
    /// Similar to Sharpe but only penalizes downside volatility.
    pub fn sortino_ratio(&self) -> Decimal {
        let mean_return = self.mean_return();
        let downside_dev = self.downside_deviation();

        if downside_dev == Decimal::ZERO {
            return Decimal::ZERO;
        }

        let annual_mean = mean_return * Decimal::from(252);
        let annual_downside = downside_dev * Decimal::from(252).sqrt().unwrap_or(Decimal::ONE);

        (annual_mean - self.risk_free_rate) / annual_downside
    }

    /// Maximum drawdown
    /// 
    /// Largest peak-to-trough decline.
    pub fn max_drawdown(&self) -> Decimal {
        // Convert returns to cumulative values (starting at 1.0)
        let mut cumulative = Decimal::ONE;
        let mut peak = Decimal::ONE;
        let mut max_dd = Decimal::ZERO;

        for ret in &self.returns {
            cumulative *= Decimal::ONE + ret;
            
            if cumulative > peak {
                peak = cumulative;
            }

            if peak > Decimal::ZERO {
                let dd = (peak - cumulative) / peak;
                if dd > max_dd {
                    max_dd = dd;
                }
            }
        }

        -max_dd // Return as negative
    }

    /// Calmar ratio
    /// 
    /// Return per unit of max drawdown.
    pub fn calmar_ratio(&self) -> Decimal {
        let annual_return = self.mean_return() * Decimal::from(252);
        let max_dd = self.max_drawdown().abs();

        if max_dd == Decimal::ZERO {
            return Decimal::ZERO;
        }

        annual_return / max_dd
    }

    /// Annualized volatility
    pub fn volatility(&self) -> Decimal {
        self.standard_deviation() * Decimal::from(252).sqrt().unwrap_or(Decimal::ONE)
    }

    /// Downside deviation (for Sortino ratio)
    /// Only considers returns below a target (usually 0 or risk-free)
    pub fn downside_deviation(&self) -> Decimal {
        let target = self.risk_free_rate / Decimal::from(252); // Daily target

        let downside_returns: Vec<Decimal> = self.returns
            .iter()
            .filter(|r| **r < target)
            .map(|r| (target - r) * (target - r))
            .collect();

        if downside_returns.is_empty() {
            return Decimal::ZERO;
        }

        let mean_downside = downside_returns.iter().sum::<Decimal>() 
            / Decimal::from(downside_returns.len() as i32);
        
        mean_downside.sqrt().unwrap_or(Decimal::ZERO)
    }

    // Private helper methods

    fn mean_return(&self) -> Decimal {
        if self.returns.is_empty() {
            return Decimal::ZERO;
        }

        self.returns.iter().sum::<Decimal>() / Decimal::from(self.returns.len() as i32)
    }

    fn standard_deviation(&self) -> Decimal {
        if self.returns.len() < 2 {
            return Decimal::ZERO;
        }

        let mean = self.mean_return();
        
        let variance = self.returns.iter()
            .map(|r| (*r - mean) * (*r - mean))
            .sum::<Decimal>() / Decimal::from(self.returns.len() as i32);

        variance.sqrt().unwrap_or(Decimal::ZERO)
    }
}

/// Portfolio risk tracker
pub struct PortfolioRiskTracker {
    daily_returns: Vec<(DateTime<Utc>, Decimal)>,
    lookback_days: usize,
}

impl PortfolioRiskTracker {
    /// Create a new risk tracker
    pub fn new(lookback_days: usize) -> Self {
        Self {
            daily_returns: Vec::new(),
            lookback_days,
        }
    }

    /// Add a daily return
    pub fn add_return(&mut self, date: DateTime<Utc>, return_pct: Decimal) {
        self.daily_returns.push((date, return_pct));
        
        // Keep only lookback period
        while self.daily_returns.len() > self.lookback_days {
            self.daily_returns.remove(0);
        }
    }

    /// Calculate current VaR
    pub fn current_var(&self, confidence: Decimal) -> Decimal {
        let returns: Vec<Decimal> = self.daily_returns.iter()
            .map(|(_, r)| *r)
            .collect();

        let analyzer = RiskAnalyzer::new(returns, Decimal::from(2) / Decimal::from(100));
        analyzer.var(confidence)
    }

    /// Check if current risk exceeds limit
    pub fn exceeds_limit(&self, var_limit: Decimal) -> bool {
        let current_var = self.current_var(Decimal::from(95) / Decimal::from(100));
        current_var > var_limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_returns() -> Vec<Decimal> {
        // 60 days of returns (approximately 3 months)
        vec![
            Decimal::from(1) / Decimal::from(100),   // 1%
            Decimal::from(-5) / Decimal::from(1000), // -0.5%
            Decimal::from(2) / Decimal::from(1000),  // 0.2%
            Decimal::from(-1) / Decimal::from(100),  // -1%
            Decimal::from(3) / Decimal::from(1000),  // 0.3%
        ]
        .into_iter()
        .cycle()
        .take(60)
        .collect()
    }

    #[test]
    fn test_var_calculation() {
        let returns = create_test_returns();
        let analyzer = RiskAnalyzer::new(returns, Decimal::from(2) / Decimal::from(100));

        let var_95 = analyzer.var(Decimal::from(95) / Decimal::from(100));
        
        // VaR should be positive (loss)
        assert!(var_95 >= Decimal::ZERO);
    }

    #[test]
    fn test_sharpe_ratio() {
        let returns = create_test_returns();
        let analyzer = RiskAnalyzer::new(returns, Decimal::from(2) / Decimal::from(100));

        let sharpe = analyzer.sharpe_ratio();
        
        // Sharpe should be a finite number
        assert!(sharpe.is_finite());
    }

    #[test]
    fn test_max_drawdown() {
        let returns = create_test_returns();
        let analyzer = RiskAnalyzer::new(returns, Decimal::ZERO);

        let max_dd = analyzer.max_drawdown();
        
        // Max drawdown should be negative or zero
        assert!(max_dd <= Decimal::ZERO);
    }
}
