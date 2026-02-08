//! Performance Attribution
//!
//! S7-D3: Returns breakdown by factor

use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::analytics::Result;

/// Performance attribution analyzer
pub struct AttributionAnalyzer;

/// Attribution result
#[derive(Debug, Clone)]
pub struct AttributionResult {
    pub total_return: Decimal,
    pub allocation_effect: Decimal,
    pub selection_effect: Decimal,
    pub interaction_effect: Decimal,
    pub sector_attributions: Vec<SectorAttribution>,
    pub factor_attributions: Vec<FactorAttribution>,
}

/// Sector-level attribution
#[derive(Debug, Clone)]
pub struct SectorAttribution {
    pub sector: String,
    pub portfolio_weight: Decimal,
    pub benchmark_weight: Decimal,
    pub portfolio_return: Decimal,
    pub benchmark_return: Decimal,
    pub allocation_effect: Decimal,
    pub selection_effect: Decimal,
    pub interaction_effect: Decimal,
    pub total_effect: Decimal,
}

/// Factor attribution
#[derive(Debug, Clone)]
pub struct FactorAttribution {
    pub factor: String,
    pub exposure: Decimal,
    pub return_contribution: Decimal,
    pub risk_contribution: Decimal,
}

impl AttributionAnalyzer {
    /// Brinson-Fachler attribution model
    /// 
    /// Decomposes excess return into:
    /// - Allocation effect: From over/under-weighting sectors
    /// - Selection effect: From picking better stocks within sectors
    /// - Interaction effect: Combined effect
    pub fn brinson_attribution(
        portfolio_weights: &HashMap<String, Decimal>,
        benchmark_weights: &HashMap<String, Decimal>,
        portfolio_returns: &HashMap<String, Decimal>,
        benchmark_returns: &HashMap<String, Decimal>,
    ) -> AttributionResult {
        let mut allocation_effect = Decimal::ZERO;
        let mut selection_effect = Decimal::ZERO;
        let mut interaction_effect = Decimal::ZERO;
        let mut sector_attributions = Vec::new();

        // Get all sectors
        let mut all_sectors: Vec<String> = portfolio_weights.keys()
            .chain(benchmark_weights.keys())
            .cloned()
            .collect();
        all_sectors.sort();
        all_sectors.dedup();

        for sector in &all_sectors {
            let w_p = *portfolio_weights.get(sector).unwrap_or(&Decimal::ZERO);
            let w_b = *benchmark_weights.get(sector).unwrap_or(&Decimal::ZERO);
            let r_p = *portfolio_returns.get(sector).unwrap_or(&Decimal::ZERO);
            let r_b = *benchmark_returns.get(sector).unwrap_or(&Decimal::ZERO);

            // Brinson-Fachler formulas
            let alloc = (w_p - w_b) * r_b;
            let select = w_b * (r_p - r_b);
            let interact = (w_p - w_b) * (r_p - r_b);

            allocation_effect += alloc;
            selection_effect += select;
            interaction_effect += interact;

            sector_attributions.push(SectorAttribution {
                sector: sector.clone(),
                portfolio_weight: w_p,
                benchmark_weight: w_b,
                portfolio_return: r_p,
                benchmark_return: r_b,
                allocation_effect: alloc,
                selection_effect: select,
                interaction_effect: interact,
                total_effect: alloc + select + interact,
            });
        }

        let total_return = portfolio_returns.values().sum::<Decimal>()
            - benchmark_returns.values().sum::<Decimal>();

        AttributionResult {
            total_return,
            allocation_effect,
            selection_effect,
            interaction_effect,
            sector_attributions,
            factor_attributions: Vec::new(),
        }
    }

    /// Factor attribution using regression
    /// 
    /// Identifies exposure to common factors:
    /// - Market
    /// - Size (SMB)
    /// - Value (HML)
    /// - Momentum (MOM)
    pub fn factor_attribution(
        portfolio_returns: &[Decimal],
        factor_returns: &HashMap<String, Vec<Decimal>>,
    ) -> Vec<FactorAttribution> {
        let mut attributions = Vec::new();

        // Simplified - would use regression in full implementation
        for (factor_name, returns) in factor_returns {
            if returns.len() != portfolio_returns.len() {
                continue;
            }

            // Calculate correlation (simplified as "exposure")
            let exposure = Self::calculate_beta(portfolio_returns, returns);
            
            // Calculate contribution
            let avg_factor_return = returns.iter().sum::<Decimal>() 
                / Decimal::from(returns.len() as i32);
            let contribution = exposure * avg_factor_return;

            attributions.push(FactorAttribution {
                factor: factor_name.clone(),
                exposure,
                return_contribution: contribution,
                risk_contribution: contribution.abs(), // Simplified
            });
        }

        attributions
    }

    /// Calculate beta (simplified)
    fn calculate_beta(portfolio_returns: &[Decimal], factor_returns: &[Decimal]) -> Decimal {
        if portfolio_returns.len() != factor_returns.len() || portfolio_returns.is_empty() {
            return Decimal::ZERO;
        }

        let n = Decimal::from(portfolio_returns.len() as i32);
        
        let mean_p = portfolio_returns.iter().sum::<Decimal>() / n;
        let mean_f = factor_returns.iter().sum::<Decimal>() / n;

        // Covariance
        let cov: Decimal = portfolio_returns.iter()
            .zip(factor_returns.iter())
            .map(|(p, f)| (*p - mean_p) * (*f - mean_f))
            .sum::<Decimal>() / n;

        // Variance of factor
        let var_f: Decimal = factor_returns.iter()
            .map(|f| (*f - mean_f) * (*f - mean_f))
            .sum::<Decimal>() / n;

        if var_f == Decimal::ZERO {
            return Decimal::ZERO;
        }

        cov / var_f
    }
}

/// Return contribution analysis
pub struct ContributionAnalyzer;

impl ContributionAnalyzer {
    /// Calculate contribution of each position to total return
    pub fn position_contributions(
        positions: &[(String, Decimal, Decimal, Decimal)], // (ticker, weight, return, pnl)
    ) -> Vec<PositionContribution> {
        let total_pnl: Decimal = positions.iter().map(|(_, _, _, pnl)| pnl).sum();

        positions.iter()
            .map(|(ticker, weight, ret, pnl)| {
                PositionContribution {
                    ticker: ticker.clone(),
                    weight: *weight,
                    return_pct: *ret,
                    pnl: *pnl,
                    contribution_pct: if total_pnl != Decimal::ZERO {
                        *pnl / total_pnl
                    } else {
                        Decimal::ZERO
                    },
                }
            })
            .collect()
    }
}

/// Position contribution
#[derive(Debug, Clone)]
pub struct PositionContribution {
    pub ticker: String,
    pub weight: Decimal,
    pub return_pct: Decimal,
    pub pnl: Decimal,
    pub contribution_pct: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brinson_attribution() {
        let portfolio_weights: HashMap<String, Decimal> = [
            ("Tech".to_string(), Decimal::from(60) / Decimal::from(100)),
            ("Finance".to_string(), Decimal::from(40) / Decimal::from(100)),
        ].into();

        let benchmark_weights: HashMap<String, Decimal> = [
            ("Tech".to_string(), Decimal::from(50) / Decimal::from(100)),
            ("Finance".to_string(), Decimal::from(50) / Decimal::from(100)),
        ].into();

        let portfolio_returns: HashMap<String, Decimal> = [
            ("Tech".to_string(), Decimal::from(10) / Decimal::from(100)),
            ("Finance".to_string(), Decimal::from(5) / Decimal::from(100)),
        ].into();

        let benchmark_returns: HashMap<String, Decimal> = [
            ("Tech".to_string(), Decimal::from(8) / Decimal::from(100)),
            ("Finance".to_string(), Decimal::from(6) / Decimal::from(100)),
        ].into();

        let result = AttributionAnalyzer::brinson_attribution(
            &portfolio_weights,
            &benchmark_weights,
            &portfolio_returns,
            &benchmark_returns,
        );

        // Portfolio return: 0.6 * 0.10 + 0.4 * 0.05 = 0.08
        // Benchmark return: 0.5 * 0.08 + 0.5 * 0.06 = 0.07
        // Excess return: 0.01

        assert!(result.total_return > Decimal::ZERO);
        assert!(result.allocation_effect != Decimal::ZERO || 
                result.selection_effect != Decimal::ZERO);
    }
}
