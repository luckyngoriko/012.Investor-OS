//! Efficient Frontier Calculator
//!
//! Calculates the efficient frontier for mean-variance optimization

use rust_decimal::Decimal;
use tracing::info;

use super::Asset;

/// Efficient frontier calculator
#[derive(Debug)]
pub struct EfficientFrontier {
    risk_free_rate: Decimal,
    num_points: usize,
}

/// Point on the efficient frontier
#[derive(Debug, Clone)]
pub struct PortfolioPoint {
    pub expected_return: Decimal,
    pub risk: Decimal,
    pub weights: Vec<(String, Decimal)>,
    pub sharpe_ratio: f32,
}

impl EfficientFrontier {
    /// Create new calculator
    pub fn new() -> Self {
        Self {
            risk_free_rate: Decimal::try_from(0.02).unwrap(),
            num_points: 20,
        }
    }
    
    /// Set risk-free rate
    pub fn set_risk_free_rate(&mut self, rate: Decimal) {
        self.risk_free_rate = rate;
    }
    
    /// Set number of points
    pub fn set_num_points(&mut self, points: usize) {
        self.num_points = points.max(5).min(100);
    }
    
    /// Calculate efficient frontier
    pub fn calculate(&self, assets: &[Asset], points: usize) -> Vec<PortfolioPoint> {
        if assets.len() < 2 {
            return Vec::new();
        }
        
        let num_points = points.max(5).min(100);
        
        // Find return range
        let min_return = assets.iter()
            .map(|a| a.expected_return)
            .min()
            .unwrap_or(Decimal::ZERO);
        
        let max_return = assets.iter()
            .map(|a| a.expected_return)
            .max()
            .unwrap_or(Decimal::from(1));
        
        // Build covariance matrix
        let cov_matrix = self.build_covariance_matrix(assets);
        
        // Generate frontier points
        let mut frontier = Vec::new();
        
        for i in 0..num_points {
            let t = i as f32 / (num_points - 1) as f32;
            let target_return = min_return + (max_return - min_return) * Decimal::try_from(t).unwrap();
            
            if let Some(point) = self.optimize_for_return(assets, target_return, &cov_matrix) {
                frontier.push(point);
            }
        }
        
        // Sort by risk
        frontier.sort_by(|a, b| a.risk.partial_cmp(&b.risk).unwrap());
        
        info!(
            "Efficient frontier calculated: {} points, return range [{:.2}%, {:.2}%]",
            frontier.len(),
            min_return * Decimal::from(100),
            max_return * Decimal::from(100)
        );
        
        frontier
    }
    
    /// Optimize for specific target return
    fn optimize_for_return(
        &self,
        assets: &[Asset],
        target_return: Decimal,
        cov_matrix: &[Vec<f32>],
    ) -> Option<PortfolioPoint> {
        // Simplified optimization: find combination of two assets that hits target
        let n = assets.len();
        
        // Try all pairs
        let mut best_risk = f32::INFINITY;
        let mut best_weights: Option<Vec<Decimal>> = None;
        
        for i in 0..n {
            for j in (i + 1)..n {
                let ret_i = assets[i].expected_return;
                let ret_j = assets[j].expected_return;
                
                // Skip if both have same return
                if (ret_i - ret_j).abs() < Decimal::try_from(0.0001).unwrap() {
                    continue;
                }
                
                // Calculate weight for asset i to hit target return
                // w_i * r_i + (1 - w_i) * r_j = target
                // w_i = (target - r_j) / (r_i - r_j)
                
                let numerator = target_return - ret_j;
                let denominator = ret_i - ret_j;
                
                if denominator == Decimal::ZERO {
                    continue;
                }
                
                let w_i: f32 = (numerator / denominator).try_into().unwrap_or(0.5);
                let w_i = w_i.clamp(0.0, 1.0);
                let w_j = 1.0 - w_i;
                
                // Build weight vector
                let mut weights = vec![Decimal::ZERO; n];
                weights[i] = Decimal::try_from(w_i).unwrap();
                weights[j] = Decimal::try_from(w_j).unwrap();
                
                // Calculate risk
                let risk = self.calculate_portfolio_risk(&weights, cov_matrix);
                
                if risk < best_risk {
                    best_risk = risk;
                    best_weights = Some(weights);
                }
            }
        }
        
        // Also try single asset solutions
        for (i, asset) in assets.iter().enumerate() {
            if (asset.expected_return - target_return).abs() < Decimal::try_from(0.01).unwrap() {
                let mut weights = vec![Decimal::ZERO; n];
                weights[i] = Decimal::from(1);
                
                let risk = self.calculate_portfolio_risk(&weights, cov_matrix);
                
                if risk < best_risk {
                    best_risk = risk;
                    best_weights = Some(weights);
                }
            }
        }
        
        best_weights.map(|weights| {
            let risk = Decimal::try_from(best_risk).unwrap_or(Decimal::ZERO);
            let excess_return = target_return - self.risk_free_rate;
            let sharpe = if risk > Decimal::ZERO {
                let ratio: f32 = (excess_return / risk).try_into().unwrap_or(0.0f32);
                ratio
            } else {
                0.0
            };
            
            let weight_pairs: Vec<(String, Decimal)> = assets.iter()
                .enumerate()
                .filter(|(i, _)| weights[*i] > Decimal::ZERO)
                .map(|(i, a)| (a.symbol.clone(), weights[i]))
                .collect();
            
            PortfolioPoint {
                expected_return: target_return,
                risk,
                weights: weight_pairs,
                sharpe_ratio: sharpe,
            }
        })
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
    
    /// Calculate portfolio risk
    fn calculate_portfolio_risk(
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
        
        variance.sqrt().max(0.0)
    }
    
    /// Find tangency portfolio (max Sharpe)
    pub fn find_tangency_portfolio(&self, frontier: &[PortfolioPoint]) -> Option<PortfolioPoint> {
        frontier.iter().max_by(|a, b| {
            a.sharpe_ratio.partial_cmp(&b.sharpe_ratio).unwrap()
        }).cloned()
    }
    
    /// Find global minimum variance portfolio
    pub fn find_global_minimum_variance(&self, frontier: &[PortfolioPoint]) -> Option<PortfolioPoint> {
        frontier.iter().min_by(|a, b| {
            a.risk.partial_cmp(&b.risk).unwrap()
        }).cloned()
    }
    
    /// Calculate capital allocation line (CAL)
    pub fn calculate_cal(&self, tangency_portfolio: &PortfolioPoint, num_points: usize) -> Vec<(Decimal, Decimal)> {
        let mut cal = Vec::new();
        
        for i in 0..num_points {
            let t = i as f32 / (num_points - 1) as f32;
            let allocation_to_risky = Decimal::try_from(t).unwrap(); // 0 to 1
            
            // Portfolio on CAL
            let portfolio_return = self.risk_free_rate 
                + allocation_to_risky * (tangency_portfolio.expected_return - self.risk_free_rate);
            
            let portfolio_risk = allocation_to_risky * tangency_portfolio.risk;
            
            cal.push((portfolio_risk, portfolio_return));
        }
        
        cal
    }
    
    /// Export frontier to CSV format
    pub fn to_csv(&self, frontier: &[PortfolioPoint]) -> String {
        let mut csv = String::new();
        csv.push_str("Risk,Return,Sharpe\n");
        
        for point in frontier {
            let risk_f32: f32 = point.risk.try_into().unwrap_or(0.0f32);
            let ret_f32: f32 = point.expected_return.try_into().unwrap_or(0.0f32);
            
            csv.push_str(&format!(
                "{:.4},{:.4},{:.4}\n",
                risk_f32,
                ret_f32,
                point.sharpe_ratio
            ));
        }
        
        csv
    }
    
    /// Plot frontier as ASCII chart
    pub fn plot_ascii(&self, frontier: &[PortfolioPoint], width: usize, height: usize) -> String {
        if frontier.is_empty() {
            return "Empty frontier".to_string();
        }
        
        let min_risk: f32 = frontier.iter().map(|p| p.risk.try_into().unwrap_or(0.0f32)).fold(f32::INFINITY, f32::min);
        let max_risk: f32 = frontier.iter().map(|p| p.risk.try_into().unwrap_or(0.0f32)).fold(0.0f32, f32::max);
        
        let min_ret: f32 = frontier.iter().map(|p| p.expected_return.try_into().unwrap_or(0.0f32)).fold(f32::INFINITY, f32::min);
        let max_ret: f32 = frontier.iter().map(|p| p.expected_return.try_into().unwrap_or(0.0f32)).fold(0.0f32, f32::max);
        
        let mut plot = vec![vec![' '; width]; height];
        
        // Draw axes
        for y in 0..height {
            plot[y][0] = '|';
        }
        for x in 0..width {
            plot[height - 1][x] = '-';
        }
        plot[height - 1][0] = '+';
        
        // Plot points
        for point in frontier {
            let risk_f32: f32 = point.risk.try_into().unwrap_or(0.0f32);
            let ret_f32: f32 = point.expected_return.try_into().unwrap_or(0.0f32);
            
            let x = if max_risk > min_risk {
                ((risk_f32 - min_risk) / (max_risk - min_risk) * (width - 2) as f32) as usize
            } else {
                0
            };
            
            let y = if max_ret > min_ret {
                height - 2 - ((ret_f32 - min_ret) / (max_ret - min_ret) * (height - 2) as f32) as usize
            } else {
                0
            };
            
            if x < width && y < height {
                plot[y][x + 1] = '*';
            }
        }
        
        // Convert to string
        let mut result = String::new();
        result.push_str(&format!("Return ({}% - {}%)\n", min_ret * 100.0, max_ret * 100.0));
        
        for row in plot {
            result.push_str(&row.into_iter().collect::<String>());
            result.push('\n');
        }
        
        result.push_str(&format!("Risk ({}% - {}%)\n", min_risk * 100.0, max_risk * 100.0));
        
        result
    }
}

impl Default for EfficientFrontier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_asset(symbol: &str, expected_return: f32, risk: f32) -> Asset {
        Asset {
            symbol: symbol.to_string(),
            expected_return: Decimal::try_from(expected_return).unwrap(),
            risk: Decimal::try_from(risk).unwrap(),
            correlations: HashMap::new(),
        }
    }

    #[test]
    fn test_calculator_creation() {
        let ef = EfficientFrontier::new();
        assert_eq!(ef.num_points, 20);
    }

    #[test]
    fn test_calculate_frontier() {
        let ef = EfficientFrontier::new();
        
        let assets = vec![
            create_test_asset("STOCKS", 0.12, 0.20),
            create_test_asset("BONDS", 0.05, 0.08),
        ];
        
        let frontier = ef.calculate(&assets, 10);
        
        assert!(!frontier.is_empty());
        assert_eq!(frontier.len(), 10);
        
        // Should be sorted by risk
        for i in 1..frontier.len() {
            assert!(frontier[i].risk >= frontier[i - 1].risk);
        }
    }

    #[test]
    fn test_tangency_portfolio() {
        let ef = EfficientFrontier::new();
        
        let mut stocks = create_test_asset("STOCKS", 0.12, 0.20);
        let mut bonds = create_test_asset("BONDS", 0.05, 0.08);
        
        stocks.set_correlation("BONDS", 0.3);
        bonds.set_correlation("STOCKS", 0.3);
        
        let assets = vec![stocks, bonds];
        let frontier = ef.calculate(&assets, 20);
        
        let tangency = ef.find_tangency_portfolio(&frontier);
        assert!(tangency.is_some());
        
        let tp = tangency.unwrap();
        assert!(tp.sharpe_ratio > 0.0);
    }

    #[test]
    fn test_global_minimum_variance() {
        let ef = EfficientFrontier::new();
        
        let assets = vec![
            create_test_asset("A", 0.12, 0.25),
            create_test_asset("B", 0.10, 0.15),
            create_test_asset("C", 0.08, 0.10),
        ];
        
        let frontier = ef.calculate(&assets, 15);
        let gmv = ef.find_global_minimum_variance(&frontier);
        
        assert!(gmv.is_some());
        
        // GMV should have lowest risk
        let min_risk = gmv.unwrap().risk;
        for point in &frontier {
            assert!(point.risk >= min_risk || (point.risk - min_risk).abs() < Decimal::try_from(0.0001).unwrap());
        }
    }

    #[test]
    fn test_calculate_cal() {
        let ef = EfficientFrontier::new();
        
        let tp = PortfolioPoint {
            expected_return: Decimal::try_from(0.10).unwrap(),
            risk: Decimal::try_from(0.15).unwrap(),
            weights: vec![],
            sharpe_ratio: 0.5,
        };
        
        let cal = ef.calculate_cal(&tp, 5);
        
        assert_eq!(cal.len(), 5);
        
        // First point should be risk-free (0 risk)
        assert!(cal[0].0 < Decimal::try_from(0.001).unwrap());
        
        // Last point should be at tangency portfolio risk
        assert!(cal[4].0 >= tp.risk - Decimal::try_from(0.001).unwrap());
    }

    #[test]
    fn test_csv_export() {
        let ef = EfficientFrontier::new();
        
        let frontier = vec![
            PortfolioPoint {
                expected_return: Decimal::try_from(0.05).unwrap(),
                risk: Decimal::try_from(0.08).unwrap(),
                weights: vec![],
                sharpe_ratio: 0.3,
            },
            PortfolioPoint {
                expected_return: Decimal::try_from(0.10).unwrap(),
                risk: Decimal::try_from(0.15).unwrap(),
                weights: vec![],
                sharpe_ratio: 0.5,
            },
        ];
        
        let csv = ef.to_csv(&frontier);
        
        assert!(csv.contains("Risk,Return,Sharpe"));
        assert!(csv.contains("0.0800,0.0500,0.3000"));
    }

    #[test]
    fn test_empty_assets() {
        let ef = EfficientFrontier::new();
        let frontier = ef.calculate(&[], 10);
        assert!(frontier.is_empty());
    }

    #[test]
    fn test_single_asset() {
        let ef = EfficientFrontier::new();
        let assets = vec![create_test_asset("ONLY", 0.10, 0.20)];
        let frontier = ef.calculate(&assets, 10);
        assert!(frontier.is_empty() || frontier.len() == 1);
    }
}
