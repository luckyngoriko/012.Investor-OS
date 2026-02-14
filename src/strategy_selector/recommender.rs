//! Strategy Recommender
//!
//! Provides personalized strategy recommendations based on user profile

use rust_decimal::Decimal;
use tracing::info;

use super::{MarketRegime, RiskTolerance, Strategy, StrategyType};

/// Strategy recommender
#[derive(Debug)]
pub struct StrategyRecommender {
    min_capital_threshold: Decimal,
    diversification_target: u32,
}

/// Recommendation
#[derive(Debug, Clone)]
pub struct Recommendation {
    pub strategy_id: uuid::Uuid,
    pub strategy_type: StrategyType,
    pub strategy_name: String,
    pub rank: u32,
    pub score: f32,
    pub reason: String,
    pub expected_return: f32,
    pub risk_level: RiskLevel,
    pub min_capital: Decimal,
    pub suitability_pct: f32, // 0-100%
}

/// Risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

impl RiskLevel {
    pub fn from_max_drawdown(drawdown: f32) -> Self {
        match drawdown {
            d if d < 0.05 => RiskLevel::VeryLow,
            d if d < 0.10 => RiskLevel::Low,
            d if d < 0.20 => RiskLevel::Medium,
            d if d < 0.30 => RiskLevel::High,
            _ => RiskLevel::VeryHigh,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            RiskLevel::VeryLow => "Very Low",
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium",
            RiskLevel::High => "High",
            RiskLevel::VeryHigh => "Very High",
        }
    }
    
    pub fn is_compatible_with(&self, tolerance: RiskTolerance) -> bool {
        match tolerance {
            RiskTolerance::Conservative => matches!(self, RiskLevel::VeryLow | RiskLevel::Low),
            RiskTolerance::Moderate => matches!(self, RiskLevel::Low | RiskLevel::Medium),
            RiskTolerance::Aggressive => matches!(self, RiskLevel::Medium | RiskLevel::High),
            RiskTolerance::Speculative => true, // All risk levels acceptable
        }
    }
}

impl StrategyRecommender {
    /// Create new recommender
    pub fn new() -> Self {
        Self {
            min_capital_threshold: Decimal::from(1000),
            diversification_target: 3,
        }
    }
    
    /// Generate recommendations
    pub fn recommend(
        &self,
        strategies: &[&Strategy],
        capital: Decimal,
        risk_tolerance: RiskTolerance,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();
        let mut rank = 1u32;
        
        for strategy in strategies {
            // Check capital requirement
            if strategy.min_capital > capital {
                continue;
            }
            
            // Check risk compatibility
            let risk_level = RiskLevel::from_max_drawdown(strategy.max_drawdown);
            if !risk_level.is_compatible_with(risk_tolerance) {
                continue;
            }
            
            // Calculate suitability
            let suitability = self.calculate_suitability(strategy, capital, risk_tolerance);
            
            // Generate reason
            let reason = self.generate_reason(strategy, risk_tolerance);
            
            let rec = Recommendation {
                strategy_id: strategy.id,
                strategy_type: strategy.strategy_type,
                strategy_name: strategy.name.clone(),
                rank,
                score: suitability,
                reason,
                expected_return: strategy.avg_return,
                risk_level,
                min_capital: strategy.min_capital,
                suitability_pct: (suitability * 100.0),
            };
            
            recommendations.push(rec);
            rank += 1;
            
            // Limit to top recommendations
            if recommendations.len() >= self.diversification_target as usize {
                break;
            }
        }
        
        info!(
            "Generated {} recommendations for capital={} tolerance={:?}",
            recommendations.len(), capital, risk_tolerance
        );
        
        recommendations
    }
    
    /// Calculate suitability score (0.0 - 1.0)
    fn calculate_suitability(
        &self,
        strategy: &Strategy,
        capital: Decimal,
        risk_tolerance: RiskTolerance,
    ) -> f32 {
        let mut score = 0.0;
        
        // Capital efficiency (0.0 - 0.25)
        let capital_ratio: f32 = (capital / strategy.min_capital).try_into().unwrap_or(1.0f32);
        score += capital_ratio.min(2.0) * 0.125; // Up to 0.25
        
        // Sharpe ratio component (0.0 - 0.25)
        score += (strategy.sharpe_ratio / 2.0).min(1.0) * 0.25;
        
        // Win rate component (0.0 - 0.20)
        score += strategy.win_rate * 0.20;
        
        // Risk match component (0.0 - 0.30)
        let risk_score = self.calculate_risk_match(strategy, risk_tolerance);
        score += risk_score * 0.30;
        
        score.min(1.0)
    }
    
    /// Calculate risk match score
    fn calculate_risk_match(&self, strategy: &Strategy, tolerance: RiskTolerance) -> f32 {
        let risk_level = RiskLevel::from_max_drawdown(strategy.max_drawdown);
        
        match (tolerance, risk_level) {
            // Perfect matches
            (RiskTolerance::Conservative, RiskLevel::VeryLow) => 1.0,
            (RiskTolerance::Moderate, RiskLevel::Low) => 1.0,
            (RiskTolerance::Aggressive, RiskLevel::Medium) => 1.0,
            (RiskTolerance::Speculative, RiskLevel::High) => 1.0,
            
            // Acceptable but not ideal
            (RiskTolerance::Conservative, RiskLevel::Low) => 0.8,
            (RiskTolerance::Moderate, RiskLevel::Medium) => 0.8,
            (RiskTolerance::Aggressive, RiskLevel::High) => 0.8,
            (RiskTolerance::Speculative, RiskLevel::VeryHigh) => 0.8,
            
            // Borderline
            (RiskTolerance::Conservative, RiskLevel::Medium) => 0.5,
            (RiskTolerance::Moderate, RiskLevel::High) => 0.5,
            (RiskTolerance::Aggressive, RiskLevel::Low) => 0.6,
            (RiskTolerance::Aggressive, RiskLevel::VeryHigh) => 0.6,
            
            // Mismatches
            _ => 0.2,
        }
    }
    
    /// Generate recommendation reason
    fn generate_reason(&self, strategy: &Strategy, tolerance: RiskTolerance) -> String {
        let risk_level = RiskLevel::from_max_drawdown(strategy.max_drawdown);
        
        let base_reason = match (tolerance, risk_level) {
            (RiskTolerance::Conservative, _) => {
                format!(
                    "Conservative choice with {} max drawdown and {:.1}% Sharpe ratio",
                    risk_level.name(),
                    strategy.sharpe_ratio
                )
            }
            (RiskTolerance::Aggressive, _) => {
                format!(
                    "High-return potential with {:.1}% average return",
                    strategy.avg_return * 100.0
                )
            }
            _ => {
                format!(
                    "Balanced risk/return with {:.1}% win rate",
                    strategy.win_rate * 100.0
                )
            }
        };
        
        base_reason
    }
    
    /// Recommend portfolio allocation across strategies
    pub fn recommend_allocation(
        &self,
        strategies: &[&Strategy],
        capital: Decimal,
        risk_tolerance: RiskTolerance,
    ) -> Vec<(uuid::Uuid, f32)> {
        let recommendations = self.recommend(strategies, capital, risk_tolerance);
        
        if recommendations.is_empty() {
            return Vec::new();
        }
        
        // Simple equal weight allocation
        let count = recommendations.len() as f32;
        let weight = 1.0 / count;
        
        recommendations
            .into_iter()
            .map(|r| (r.strategy_id, weight))
            .collect()
    }
    
    /// Get strategy for regime
    pub fn recommend_for_regime(
        &self,
        strategies: &[&Strategy],
        regime: MarketRegime,
        capital: Decimal,
    ) -> Option<Recommendation> {
        let suitable: Vec<_> = strategies
            .iter()
            .filter(|s| s.strategy_type.is_suitable_for(regime))
            .filter(|s| s.min_capital <= capital)
            .cloned()
            .collect();
        
        if suitable.is_empty() {
            return None;
        }
        
        // Pick best Sharpe ratio
        let best = suitable
            .into_iter()
            .max_by(|a, b| a.sharpe_ratio.partial_cmp(&b.sharpe_ratio).unwrap())?;
        
        Some(Recommendation {
            strategy_id: best.id,
            strategy_type: best.strategy_type,
            strategy_name: best.name.clone(),
            rank: 1,
            score: best.sharpe_ratio / 2.0,
            reason: format!("Best performing strategy for {:?} regime", regime),
            expected_return: best.avg_return,
            risk_level: RiskLevel::from_max_drawdown(best.max_drawdown),
            min_capital: best.min_capital,
            suitability_pct: 85.0,
        })
    }
    
    /// Set diversification target
    pub fn set_diversification_target(&mut self, target: u32) {
        self.diversification_target = target;
    }
}

impl Default for StrategyRecommender {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_strategy(name: &str, strategy_type: StrategyType, min_capital: i64, drawdown: f32) -> Strategy {
        Strategy {
            id: Uuid::new_v4(),
            name: name.to_string(),
            strategy_type,
            description: "Test".to_string(),
            min_capital: Decimal::from(min_capital),
            max_drawdown: drawdown,
            avg_return: 0.12,
            sharpe_ratio: 1.2,
            win_rate: 0.55,
            trades_per_month: 10,
            created_at: Utc::now(),
            is_active: true,
            current_allocation: 0.0,
        }
    }

    #[test]
    fn test_recommender_creation() {
        let recommender = StrategyRecommender::new();
        let strategies: Vec<&Strategy> = vec![];
        let recs = recommender.recommend(&strategies, Decimal::from(10000), RiskTolerance::Moderate);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_recommendation_filtering() {
        let recommender = StrategyRecommender::new();
        
        let s1 = create_test_strategy("Low Risk", StrategyType::MeanReversion, 5000, 0.05);
        let s2 = create_test_strategy("High Risk", StrategyType::Momentum, 5000, 0.30);
        
        let strategies: Vec<&Strategy> = vec![&s1, &s2];
        
        // Conservative should only get low risk
        let conservative = recommender.recommend(&strategies, Decimal::from(10000), RiskTolerance::Conservative);
        assert_eq!(conservative.len(), 1);
        assert_eq!(conservative[0].strategy_name, "Low Risk");
        
        // Speculative can get both
        let speculative = recommender.recommend(&strategies, Decimal::from(10000), RiskTolerance::Speculative);
        assert_eq!(speculative.len(), 2);
    }

    #[test]
    fn test_capital_filtering() {
        let recommender = StrategyRecommender::new();
        
        let s1 = create_test_strategy("Expensive", StrategyType::Momentum, 50000, 0.10);
        let s2 = create_test_strategy("Affordable", StrategyType::MeanReversion, 5000, 0.10);
        
        let strategies: Vec<&Strategy> = vec![&s1, &s2];
        let recs = recommender.recommend(&strategies, Decimal::from(10000), RiskTolerance::Moderate);
        
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].strategy_name, "Affordable");
    }

    #[test]
    fn test_risk_level_classification() {
        assert_eq!(RiskLevel::from_max_drawdown(0.03), RiskLevel::VeryLow);
        assert_eq!(RiskLevel::from_max_drawdown(0.08), RiskLevel::Low);
        assert_eq!(RiskLevel::from_max_drawdown(0.15), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_max_drawdown(0.25), RiskLevel::High);
        assert_eq!(RiskLevel::from_max_drawdown(0.35), RiskLevel::VeryHigh);
    }

    #[test]
    fn test_risk_compatibility() {
        assert!(RiskLevel::Low.is_compatible_with(RiskTolerance::Conservative));
        assert!(!RiskLevel::High.is_compatible_with(RiskTolerance::Conservative));
        assert!(RiskLevel::High.is_compatible_with(RiskTolerance::Speculative));
    }

    #[test]
    fn test_recommend_for_regime() {
        let recommender = StrategyRecommender::new();
        
        let momentum = create_test_strategy("Momentum", StrategyType::Momentum, 5000, 0.15);
        let mean_rev = create_test_strategy("MeanRev", StrategyType::MeanReversion, 5000, 0.10);
        
        let strategies: Vec<&Strategy> = vec![&momentum, &mean_rev];
        
        // Trending regime should suggest momentum
        let rec = recommender.recommend_for_regime(&strategies, MarketRegime::Trending, Decimal::from(10000));
        assert!(rec.is_some());
        assert_eq!(rec.unwrap().strategy_type, StrategyType::Momentum);
        
        // Ranging regime should suggest mean reversion
        let rec = recommender.recommend_for_regime(&strategies, MarketRegime::Ranging, Decimal::from(10000));
        assert!(rec.is_some());
        assert_eq!(rec.unwrap().strategy_type, StrategyType::MeanReversion);
    }

    #[test]
    fn test_recommend_allocation() {
        let recommender = StrategyRecommender::new();
        
        let s1 = create_test_strategy("S1", StrategyType::Momentum, 5000, 0.10);
        let s2 = create_test_strategy("S2", StrategyType::MeanReversion, 5000, 0.10);
        
        let strategies: Vec<&Strategy> = vec![&s1, &s2];
        let allocation = recommender.recommend_allocation(&strategies, Decimal::from(10000), RiskTolerance::Moderate);
        
        assert_eq!(allocation.len(), 2);
        assert_eq!(allocation[0].1, 0.5); // Equal weight
        assert_eq!(allocation[1].1, 0.5);
    }
}
