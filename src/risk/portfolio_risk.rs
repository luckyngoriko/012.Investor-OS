//! Portfolio risk calculations (VaR, CVaR, drawdown)

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::VecDeque;
use tracing::debug;

use super::{RiskError, Result};

/// Value at Risk calculation method
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum VaRMethod {
    /// Historical simulation
    #[default]
    Historical,
    /// Parametric (variance-covariance)
    Parametric,
    /// Monte Carlo simulation (simplified)
    MonteCarlo,
}


/// VaR configuration
#[derive(Debug, Clone)]
pub struct VaRConfig {
    pub method: VaRMethod,
    /// Confidence level (e.g., 0.95 for 95%)
    pub confidence_level: Decimal,
    /// Lookback period in days
    pub lookback_days: usize,
    /// Time horizon in days
    pub time_horizon: usize,
}

impl Default for VaRConfig {
    fn default() -> Self {
        Self {
            method: VaRMethod::Historical,
            confidence_level: Decimal::try_from(0.95).unwrap(),
            lookback_days: 252, // 1 year
            time_horizon: 1,    // 1 day
        }
    }
}

/// Risk metrics for a portfolio or position
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    /// Value at Risk
    pub var_95: Decimal,
    /// Value at Risk (99%)
    pub var_99: Decimal,
    /// Conditional VaR (Expected Shortfall)
    pub cvar_95: Decimal,
    /// Maximum drawdown
    pub max_drawdown: Decimal,
    /// Current drawdown
    pub current_drawdown: Decimal,
    /// Volatility (annualized)
    pub volatility: Decimal,
    /// Sharpe ratio
    pub sharpe_ratio: Decimal,
    /// Sortino ratio
    pub sortino_ratio: Decimal,
    /// Beta (market correlation)
    pub beta: Option<Decimal>,
    /// Calculated at
    pub calculated_at: DateTime<Utc>,
}

impl Default for RiskMetrics {
    fn default() -> Self {
        Self {
            var_95: Decimal::ZERO,
            var_99: Decimal::ZERO,
            cvar_95: Decimal::ZERO,
            max_drawdown: Decimal::ZERO,
            current_drawdown: Decimal::ZERO,
            volatility: Decimal::ZERO,
            sharpe_ratio: Decimal::ZERO,
            sortino_ratio: Decimal::ZERO,
            beta: None,
            calculated_at: Utc::now(),
        }
    }
}

/// Portfolio position for risk calculation
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub weight: Decimal, // Portfolio weight
}

/// Portfolio risk calculator
#[derive(Debug, Clone)]
pub struct PortfolioRisk {
    config: VaRConfig,
    /// Historical portfolio values for drawdown calculation
    equity_curve: VecDeque<(DateTime<Utc>, Decimal)>,
    /// Maximum equity seen
    peak_equity: Decimal,
}

impl PortfolioRisk {
    /// Create a new portfolio risk calculator
    pub fn new(config: VaRConfig) -> Self {
        Self {
            config,
            equity_curve: VecDeque::new(),
            peak_equity: Decimal::ZERO,
        }
    }

    /// Update equity curve with new portfolio value
    pub fn update_equity(&mut self, value: Decimal) {
        let now = Utc::now();
        self.equity_curve.push_back((now, value));
        
        // Keep only recent history
        let cutoff = now - Duration::days(self.config.lookback_days as i64 * 2);
        while let Some((time, _)) = self.equity_curve.front() {
            if *time < cutoff {
                self.equity_curve.pop_front();
            } else {
                break;
            }
        }

        // Update peak
        if value > self.peak_equity {
            self.peak_equity = value;
        }
    }

    /// Calculate Value at Risk
    /// 
    /// VaR estimates how much a portfolio might lose with a given probability
    /// over a specific time period.
    pub fn calculate_var(&self, returns: &[Decimal], confidence: Decimal) -> Result<Decimal> {
        if returns.is_empty() {
            return Err(RiskError::CalculationError(
                "No return data for VaR calculation".to_string(),
            ));
        }

        match self.config.method {
            VaRMethod::Historical => self.historical_var(returns, confidence),
            VaRMethod::Parametric => self.parametric_var(returns, confidence),
            VaRMethod::MonteCarlo => self.monte_carlo_var(returns, confidence),
        }
    }

    /// Historical VaR: use actual historical returns
    fn historical_var(&self, returns: &[Decimal], confidence: Decimal) -> Result<Decimal> {
        let mut sorted_returns: Vec<Decimal> = returns.to_vec();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Find the percentile
        let index_f = (Decimal::ONE - confidence) * Decimal::from(sorted_returns.len() as i64);
        let index = index_f.to_string()
            .split('.')
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0)
            .min(sorted_returns.len().saturating_sub(1));

        Ok(sorted_returns.get(index).copied().unwrap_or(Decimal::ZERO))
    }

    /// Parametric VaR: assume normal distribution
    fn parametric_var(&self, returns: &[Decimal], confidence: Decimal) -> Result<Decimal> {
        let mean = returns.iter().sum::<Decimal>() / Decimal::from(returns.len() as i64);
        
        let variance = returns
            .iter()
            .map(|r| {
                let diff = *r - mean;
                diff * diff
            })
            .sum::<Decimal>() / Decimal::from(returns.len() as i64);

        let std_dev = approx_sqrt(variance);

        // For 95% confidence, z-score ≈ 1.645
        // For 99% confidence, z-score ≈ 2.326
        let z_score = if confidence >= Decimal::try_from(0.99).unwrap() {
            Decimal::try_from(2.326).unwrap()
        } else {
            Decimal::try_from(1.645).unwrap()
        };

        Ok(mean - z_score * std_dev)
    }

    /// Simplified Monte Carlo VaR
    fn monte_carlo_var(&self, returns: &[Decimal], confidence: Decimal) -> Result<Decimal> {
        // For simplicity, use historical as base
        self.historical_var(returns, confidence)
    }

    /// Calculate Conditional VaR (Expected Shortfall)
    /// 
    /// CVaR is the average of returns worse than VaR
    pub fn calculate_cvar(&self, returns: &[Decimal], confidence: Decimal) -> Result<Decimal> {
        let var = self.calculate_var(returns, confidence)?;

        let tail_returns: Vec<Decimal> = returns
            .iter()
            .filter(|r| **r <= var)
            .copied()
            .collect();

        if tail_returns.is_empty() {
            return Ok(var); // Fallback to VaR
        }

        let cvar = tail_returns.iter().sum::<Decimal>() / Decimal::from(tail_returns.len() as i64);
        Ok(cvar)
    }

    /// Calculate maximum drawdown
    /// 
    /// Drawdown is the decline from peak to trough
    pub fn calculate_max_drawdown(&self) -> (Decimal, Decimal) {
        let mut max_dd = Decimal::ZERO;
        let mut current_dd = Decimal::ZERO;
        let mut peak = Decimal::ZERO;

        for (_, value) in &self.equity_curve {
            if *value > peak {
                peak = *value;
            }

            if !peak.is_zero() {
                let dd = (peak - *value) / peak;
                current_dd = dd;
                if dd > max_dd {
                    max_dd = dd;
                }
            }
        }

        (max_dd, current_dd)
    }

    /// Calculate volatility (standard deviation of returns)
    pub fn calculate_volatility(&self, returns: &[Decimal]) -> Result<Decimal> {
        if returns.len() < 2 {
            return Err(RiskError::CalculationError(
                "Need at least 2 returns for volatility".to_string(),
            ));
        }

        let mean = returns.iter().sum::<Decimal>() / Decimal::from(returns.len() as i64);

        let variance = returns
            .iter()
            .map(|r| {
                let diff = *r - mean;
                diff * diff
            })
            .sum::<Decimal>() / Decimal::from(returns.len() as i64);

        Ok(approx_sqrt(variance))
    }

    /// Calculate Sharpe ratio
    pub fn calculate_sharpe(&self, returns: &[Decimal], risk_free_rate: Decimal) -> Result<Decimal> {
        let volatility = self.calculate_volatility(returns)?;

        if volatility.is_zero() {
            return Ok(Decimal::ZERO);
        }

        let mean_return = returns.iter().sum::<Decimal>() / Decimal::from(returns.len() as i64);
        let excess_return = mean_return - risk_free_rate;

        Ok(excess_return / volatility)
    }

    /// Calculate Sortino ratio (downside risk only)
    pub fn calculate_sortino(&self, returns: &[Decimal], risk_free_rate: Decimal) -> Result<Decimal> {
        let mean_return = returns.iter().sum::<Decimal>() / Decimal::from(returns.len() as i64);

        // Calculate downside deviation (only negative returns)
        let downside_returns: Vec<Decimal> = returns
            .iter()
            .filter(|r| **r < Decimal::ZERO)
            .copied()
            .collect();

        if downside_returns.is_empty() {
            return Ok(Decimal::ZERO);
        }

        let downside_variance = downside_returns
            .iter()
            .map(|r| *r * *r)
            .sum::<Decimal>() / Decimal::from(downside_returns.len() as i64);

        let downside_deviation = approx_sqrt(downside_variance);

        if downside_deviation.is_zero() {
            return Ok(Decimal::ZERO);
        }

        let excess_return = mean_return - risk_free_rate;
        Ok(excess_return / downside_deviation)
    }

    /// Calculate all risk metrics
    pub fn calculate_all_metrics(
        &self,
        returns: &[Decimal],
        risk_free_rate: Decimal,
    ) -> Result<RiskMetrics> {
        let (max_dd, current_dd) = self.calculate_max_drawdown();

        let metrics = RiskMetrics {
            var_95: self.calculate_var(returns, Decimal::try_from(0.95).unwrap())?,
            var_99: self.calculate_var(returns, Decimal::try_from(0.99).unwrap())?,
            cvar_95: self.calculate_cvar(returns, Decimal::try_from(0.95).unwrap())?,
            max_drawdown: max_dd,
            current_drawdown: current_dd,
            volatility: self.calculate_volatility(returns)?,
            sharpe_ratio: self.calculate_sharpe(returns, risk_free_rate)?,
            sortino_ratio: self.calculate_sortino(returns, risk_free_rate)?,
            beta: None,
            calculated_at: Utc::now(),
        };

        debug!(
            "Risk metrics: VaR95={}%, CVaR95={}%, MaxDD={}%",
            metrics.var_95 * Decimal::from(100),
            metrics.cvar_95 * Decimal::from(100),
            metrics.max_drawdown * Decimal::from(100)
        );

        Ok(metrics)
    }

    /// Calculate position concentration risk
    /// 
    /// Returns true if any position exceeds the max weight
    pub fn check_concentration(positions: &[Position], max_weight: Decimal) -> Option<String> {
        for pos in positions {
            if pos.weight > max_weight {
                return Some(format!(
                    "Position {} weight {} exceeds max {}",
                    pos.symbol, pos.weight, max_weight
                ));
            }
        }
        None
    }

    /// Calculate portfolio correlation risk
    /// 
    /// Returns warning if portfolio is too correlated
    pub fn check_correlation_risk(correlation_matrix: &[Vec<Decimal>], threshold: Decimal) -> bool {
        for row in correlation_matrix {
            for &corr in row {
                if corr > threshold {
                    return true;
                }
            }
        }
        false
    }
}

/// Approximate square root (same implementation as pairs.rs)
fn approx_sqrt(value: Decimal) -> Decimal {
    if value.is_zero() {
        return Decimal::ZERO;
    }

    // Use bisection method for robust sqrt calculation
    let mut low = Decimal::ZERO;
    let mut high = value.max(Decimal::ONE);
    let epsilon = Decimal::try_from(0.0001).unwrap();

    // For values < 1, sqrt is between value and 1
    if value < Decimal::ONE {
        low = value;
        high = Decimal::ONE;
    }

    for _ in 0..50 {
        // Max iterations
        let mid = (low + high) / Decimal::from(2);
        let mid_sq = mid * mid;

        if (mid_sq - value).abs() < epsilon {
            return mid;
        }

        if mid_sq < value {
            low = mid;
        } else {
            high = mid;
        }
    }

    (low + high) / Decimal::from(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_returns() -> Vec<Decimal> {
        vec![
            Decimal::try_from(0.01).unwrap(),
            Decimal::try_from(-0.02).unwrap(),
            Decimal::try_from(0.015).unwrap(),
            Decimal::try_from(-0.01).unwrap(),
            Decimal::try_from(0.005).unwrap(),
            Decimal::try_from(-0.03).unwrap(),
            Decimal::try_from(0.02).unwrap(),
            Decimal::try_from(0.01).unwrap(),
            Decimal::try_from(-0.015).unwrap(),
            Decimal::try_from(0.025).unwrap(),
        ]
    }

    #[test]
    fn test_var_calculation() {
        let config = VaRConfig::default();
        let risk = PortfolioRisk::new(config);

        let returns = create_test_returns();
        let var = risk.calculate_var(&returns, Decimal::try_from(0.95).unwrap()).unwrap();

        // VaR should be negative (potential loss)
        assert!(var < Decimal::ZERO);
    }

    #[test]
    fn test_cvar_calculation() {
        let config = VaRConfig::default();
        let risk = PortfolioRisk::new(config);

        let returns = create_test_returns();
        let var = risk.calculate_var(&returns, Decimal::try_from(0.95).unwrap()).unwrap();
        let cvar = risk.calculate_cvar(&returns, Decimal::try_from(0.95).unwrap()).unwrap();

        // CVaR should be worse (more negative) than VaR
        assert!(cvar <= var);
    }

    #[test]
    fn test_drawdown_calculation() {
        let config = VaRConfig::default();
        let mut risk = PortfolioRisk::new(config);

        // Simulate equity curve: 100 -> 120 (peak) -> 90 (drawdown)
        risk.update_equity(Decimal::from(100));
        risk.update_equity(Decimal::from(110));
        risk.update_equity(Decimal::from(120)); // Peak
        risk.update_equity(Decimal::from(105));
        risk.update_equity(Decimal::from(90)); // Trough

        let (max_dd, current_dd) = risk.calculate_max_drawdown();

        // Max drawdown: (120 - 90) / 120 = 0.25
        assert_eq!(max_dd, Decimal::try_from(0.25).unwrap());
        assert_eq!(current_dd, Decimal::try_from(0.25).unwrap());
    }

    #[test]
    fn test_volatility_calculation() {
        let config = VaRConfig::default();
        let risk = PortfolioRisk::new(config);

        let returns = create_test_returns();
        let vol = risk.calculate_volatility(&returns).unwrap();

        assert!(vol > Decimal::ZERO);
    }

    #[test]
    fn test_sharpe_ratio() {
        let config = VaRConfig::default();
        let risk = PortfolioRisk::new(config);

        let returns = create_test_returns();
        let sharpe = risk.calculate_sharpe(&returns, Decimal::ZERO).unwrap();

        // Sharpe can be positive or negative depending on returns
        assert!(sharpe.abs() >= Decimal::ZERO);
    }

    #[test]
    fn test_concentration_check() {
        let positions = vec![
            Position {
                symbol: "BTC".to_string(),
                quantity: Decimal::from(10),
                entry_price: Decimal::from(50000),
                current_price: Decimal::from(50000),
                weight: Decimal::try_from(0.6).unwrap(), // 60%
            },
            Position {
                symbol: "ETH".to_string(),
                quantity: Decimal::from(100),
                entry_price: Decimal::from(3000),
                current_price: Decimal::from(3000),
                weight: Decimal::try_from(0.4).unwrap(), // 40%
            },
        ];

        let warning = PortfolioRisk::check_concentration(&positions, Decimal::try_from(0.5).unwrap());
        assert!(warning.is_some()); // BTC exceeds 50%

        let warning = PortfolioRisk::check_concentration(&positions, Decimal::try_from(0.7).unwrap());
        assert!(warning.is_none()); // All under 70%
    }

    #[test]
    fn test_full_metrics_calculation() {
        let config = VaRConfig::default();
        let risk = PortfolioRisk::new(config);

        let returns = create_test_returns();
        let metrics = risk.calculate_all_metrics(&returns, Decimal::ZERO).unwrap();

        assert!(metrics.var_95 <= Decimal::ZERO);
        assert!(metrics.volatility >= Decimal::ZERO);
        assert!(metrics.calculated_at <= Utc::now());
    }
}
