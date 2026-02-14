//! Strategy Selector
//!
//! Selects optimal trading strategy based on market regime and performance metrics

use super::{MarketRegime, Result, SelectionScore, SelectorError, Strategy};

/// Strategy selector
#[derive(Debug)]
pub struct StrategySelector {
    regime_weight: f32,
    performance_weight: f32,
    risk_weight: f32,
    recency_weight: f32,
}

/// Selection criteria
#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    pub min_sharpe: f32,
    pub max_drawdown: f32,
    pub min_win_rate: f32,
    pub lookback_days: u32,
    pub require_proven: bool,
    pub prefer_lower_turnover: bool,
}

impl Default for SelectionCriteria {
    fn default() -> Self {
        Self {
            min_sharpe: 1.0,
            max_drawdown: 0.20,
            min_win_rate: 0.50,
            lookback_days: 90,
            require_proven: true,
            prefer_lower_turnover: false,
        }
    }
}

impl StrategySelector {
    /// Create new selector
    pub fn new() -> Self {
        Self {
            regime_weight: 0.35,
            performance_weight: 0.30,
            risk_weight: 0.20,
            recency_weight: 0.15,
        }
    }
    
    /// Select best strategy for regime
    pub fn select_best(
        &self,
        strategies: &[&Strategy],
        regime: MarketRegime,
        criteria: SelectionCriteria,
    ) -> Result<SelectionScore> {
        if strategies.is_empty() {
            return Err(SelectorError::InsufficientData(
                "No strategies provided".to_string()
            ));
        }
        
        let mut best_score: Option<SelectionScore> = None;
        
        for strategy in strategies {
            let score = self.calculate_score(strategy, regime, &criteria);
            
            if let Some(ref best) = best_score {
                if score.overall_score > best.overall_score {
                    best_score = Some(score);
                }
            } else {
                best_score = Some(score);
            }
        }
        
        best_score.ok_or_else(|| {
            SelectorError::InsufficientData("No suitable strategy found".to_string())
        })
    }
    
    /// Select top N strategies
    pub fn select_top_n(
        &self,
        strategies: &[&Strategy],
        regime: MarketRegime,
        criteria: SelectionCriteria,
        n: usize,
    ) -> Vec<SelectionScore> {
        let mut scores: Vec<SelectionScore> = strategies
            .iter()
            .map(|s| self.calculate_score(s, regime, &criteria))
            .collect();
        
        // Sort by overall score descending
        scores.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap());
        
        scores.into_iter().take(n).collect()
    }
    
    /// Calculate selection score for a strategy
    fn calculate_score(
        &self,
        strategy: &Strategy,
        regime: MarketRegime,
        criteria: &SelectionCriteria,
    ) -> SelectionScore {
        // Regime fit score
        let regime_fit = if strategy.strategy_type.is_suitable_for(regime) {
            1.0
        } else {
            0.0
        };
        
        // Performance score (based on Sharpe and returns)
        let performance = self.calculate_performance_score(strategy, criteria);
        
        // Risk-adjusted score
        let risk_adjusted = self.calculate_risk_score(strategy, criteria);
        
        // Recency score (how recent/updated is the strategy)
        let recency = 0.8; // Simplified
        
        // Overall weighted score
        let overall = 
            regime_fit * self.regime_weight +
            performance * self.performance_weight +
            risk_adjusted * self.risk_weight +
            recency * self.recency_weight;
        
        // Confidence based on data quality
        let confidence = self.calculate_confidence(strategy, criteria);
        
        SelectionScore {
            strategy_id: strategy.id,
            strategy_type: strategy.strategy_type,
            regime_fit_score: regime_fit,
            performance_score: performance,
            risk_adjusted_score: risk_adjusted,
            recency_score: recency,
            overall_score: overall,
            confidence,
        }
    }
    
    /// Calculate performance score
    fn calculate_performance_score(&self, strategy: &Strategy, criteria: &SelectionCriteria) -> f32 {
        let mut score = 0.0;
        
        // Sharpe ratio component (normalized to 0-1, assuming 2.0 is excellent)
        let sharpe_component = (strategy.sharpe_ratio / 2.0).min(1.0);
        score += sharpe_component * 0.4;
        
        // Average return component (normalized, assuming 20% is excellent)
        let return_component = (strategy.avg_return / 0.20).min(1.0);
        score += return_component * 0.3;
        
        // Win rate component
        score += strategy.win_rate * 0.3;
        
        // Apply minimum threshold penalties
        if strategy.sharpe_ratio < criteria.min_sharpe {
            score *= 0.5;
        }
        if strategy.win_rate < criteria.min_win_rate {
            score *= 0.5;
        }
        
        score
    }
    
    /// Calculate risk score
    fn calculate_risk_score(&self, strategy: &Strategy, criteria: &SelectionCriteria) -> f32 {
        let mut score = 1.0;
        
        // Penalize high drawdown
        if strategy.max_drawdown > criteria.max_drawdown {
            let excess = strategy.max_drawdown - criteria.max_drawdown;
            score -= excess * 2.0; // Heavy penalty
        }
        
        // Penalize low Sharpe
        if strategy.sharpe_ratio < 1.0 {
            score -= (1.0 - strategy.sharpe_ratio) * 0.3;
        }
        
        score.max(0.0)
    }
    
    /// Calculate confidence in selection
    fn calculate_confidence(&self, strategy: &Strategy, _criteria: &SelectionCriteria) -> f32 {
        let mut confidence = 0.5;
        
        // Higher confidence for strategies with more trades
        if strategy.trades_per_month > 5 {
            confidence += 0.1;
        }
        
        // Higher confidence for better Sharpe
        if strategy.sharpe_ratio > 1.5 {
            confidence += 0.15;
        }
        
        // Higher confidence for strategies running longer
        let age_days = (chrono::Utc::now() - strategy.created_at).num_days() as f32;
        if age_days > 90.0 {
            confidence += 0.1;
        }
        
        (confidence as f32).min(1.0)
    }
    
    /// Set weights
    pub fn set_weights(&mut self, regime: f32, performance: f32, risk: f32, recency: f32) {
        let total = regime + performance + risk + recency;
        self.regime_weight = regime / total;
        self.performance_weight = performance / total;
        self.risk_weight = risk / total;
        self.recency_weight = recency / total;
    }
    
    /// Compare two strategies
    pub fn compare(&self, a: &Strategy, b: &Strategy, regime: MarketRegime) -> std::cmp::Ordering {
        let score_a = self.calculate_score(a, regime, &SelectionCriteria::default());
        let score_b = self.calculate_score(b, regime, &SelectionCriteria::default());
        
        score_b.overall_score.partial_cmp(&score_a.overall_score).unwrap()
    }
}

impl Default for StrategySelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use crate::strategy_selector::StrategyType;
    use std::time::Duration;
    use uuid::Uuid;

    fn create_test_strategy(name: &str, strategy_type: StrategyType, sharpe: f32) -> Strategy {
        Strategy {
            id: Uuid::new_v4(),
            name: name.to_string(),
            strategy_type,
            description: "Test".to_string(),
            min_capital: Decimal::from(10000),
            max_drawdown: 0.15,
            avg_return: 0.12,
            sharpe_ratio: sharpe,
            win_rate: 0.55,
            trades_per_month: 10,
            created_at: chrono::Utc::now(),
            is_active: true,
            current_allocation: 0.0,
        }
    }

    #[test]
    fn test_selector_creation() {
        let selector = StrategySelector::new();
        let criteria = SelectionCriteria::default();
        assert_eq!(criteria.min_sharpe, 1.0);
    }

    #[test]
    fn test_select_best() {
        let selector = StrategySelector::new();
        let criteria = SelectionCriteria::default();
        
        let momentum = create_test_strategy("Momentum", StrategyType::Momentum, 1.5);
        let mean_rev = create_test_strategy("MeanRev", StrategyType::MeanReversion, 0.8);
        
        let strategies: Vec<&Strategy> = vec![&momentum, &mean_rev];
        
        // In trending regime, momentum should win
        let result = selector.select_best(&strategies, MarketRegime::Trending, criteria).unwrap();
        assert_eq!(result.strategy_type, StrategyType::Momentum);
    }

    #[test]
    fn test_select_top_n() {
        let selector = StrategySelector::new();
        let criteria = SelectionCriteria::default();
        
        let s1 = create_test_strategy("S1", StrategyType::Momentum, 1.8);
        let s2 = create_test_strategy("S2", StrategyType::MeanReversion, 1.2);
        let s3 = create_test_strategy("S3", StrategyType::Breakout, 1.0);
        
        let strategies: Vec<&Strategy> = vec![&s1, &s2, &s3];
        let top = selector.select_top_n(&strategies, MarketRegime::Trending, criteria, 2);
        
        assert_eq!(top.len(), 2);
        // Highest Sharpe should be first
        assert_eq!(top[0].strategy_type, StrategyType::Momentum);
    }

    #[test]
    fn test_regime_fit_score() {
        let selector = StrategySelector::new();
        let criteria = SelectionCriteria::default();
        
        let momentum = create_test_strategy("Momentum", StrategyType::Momentum, 1.0);
        
        // Should have high regime fit in trending
        let trending_score = selector.calculate_score(&momentum, MarketRegime::Trending, &criteria);
        assert_eq!(trending_score.regime_fit_score, 1.0);
        
        // Should have low regime fit in ranging
        let ranging_score = selector.calculate_score(&momentum, MarketRegime::Ranging, &criteria);
        assert_eq!(ranging_score.regime_fit_score, 0.0);
    }

    #[test]
    fn test_criteria_thresholds() {
        let selector = StrategySelector::new();
        let mut criteria = SelectionCriteria::default();
        criteria.min_sharpe = 2.0; // Very high requirement
        
        let low_sharpe = create_test_strategy("Low", StrategyType::Momentum, 0.8);
        let strategies: Vec<&Strategy> = vec![&low_sharpe];
        
        let score = selector.calculate_score(&low_sharpe, MarketRegime::Trending, &criteria);
        // Performance should be penalized
        assert!(score.performance_score < 0.5);
    }
}
