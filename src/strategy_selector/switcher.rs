//! Strategy Switcher
//!
//! Manages dynamic switching between strategies with confidence thresholds

use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{MarketRegime, SelectionScore};

/// Strategy switcher
#[derive(Debug)]
pub struct StrategySwitcher {
    config: SwitchConfig,
    switch_history: Vec<SwitchRecord>,
    last_switch_time: Option<DateTime<Utc>>,
}

/// Switch configuration
#[derive(Debug, Clone)]
pub struct SwitchConfig {
    pub min_score_improvement: f32, // Minimum score improvement to switch
    pub min_hold_period_seconds: i64, // Minimum time before switching again
    pub confidence_threshold: f32,  // Minimum confidence in new strategy
    pub max_switches_per_day: u32,  // Rate limiting
    pub require_regime_change: bool, // Only switch if regime changed
    pub momentum_penalty: f32,      // Penalty for switching too often
}

impl Default for SwitchConfig {
    fn default() -> Self {
        Self {
            min_score_improvement: 0.10,  // 10% better
            min_hold_period_seconds: 300, // 5 minutes
            confidence_threshold: 0.70,   // 70% confidence
            max_switches_per_day: 10,
            require_regime_change: false,
            momentum_penalty: 0.05, // 5% penalty
        }
    }
}

/// Switch record
#[derive(Debug, Clone)]
pub struct SwitchRecord {
    pub id: Uuid,
    pub from_strategy: Option<Uuid>,
    pub to_strategy: Uuid,
    pub timestamp: DateTime<Utc>,
    pub regime: MarketRegime,
    pub reason: String,
    pub score_delta: f32,
}

impl StrategySwitcher {
    /// Create new switcher
    pub fn new(config: SwitchConfig) -> Self {
        Self {
            config,
            switch_history: Vec::new(),
            last_switch_time: None,
        }
    }

    /// Determine if should switch strategy
    pub fn should_switch(&self, current: &SelectionScore, candidate: &SelectionScore) -> bool {
        // Check minimum hold period
        if let Some(last_switch) = self.last_switch_time {
            let elapsed = (Utc::now() - last_switch).num_seconds();
            if elapsed < self.config.min_hold_period_seconds {
                debug!(
                    "Hold period not met: {}s < {}s",
                    elapsed, self.config.min_hold_period_seconds
                );
                return false;
            }
        }

        // Check confidence threshold
        if candidate.confidence < self.config.confidence_threshold {
            debug!(
                "Candidate confidence too low: {} < {}",
                candidate.confidence, self.config.confidence_threshold
            );
            return false;
        }

        // Calculate effective scores
        let current_score = self.apply_penalties(current);
        let candidate_score = candidate.overall_score;

        // Check minimum improvement
        let improvement = candidate_score - current_score;
        if improvement < self.config.min_score_improvement {
            debug!(
                "Improvement too small: {} < {}",
                improvement, self.config.min_score_improvement
            );
            return false;
        }

        // Check daily switch limit
        if self.daily_switch_count() >= self.config.max_switches_per_day {
            warn!("Daily switch limit reached");
            return false;
        }

        info!(
            "Switch approved: {} -> {} (improvement: {:.2})",
            current.overall_score, candidate_score, improvement
        );

        true
    }

    /// Apply penalties to current score based on switch history
    fn apply_penalties(&self, score: &SelectionScore) -> f32 {
        let base_score = score.overall_score;

        // Apply momentum penalty for recent switches
        let recent_switches = self.recent_switch_count(3600); // Last hour
        let penalty = recent_switches as f32 * self.config.momentum_penalty;

        (base_score - penalty).max(0.0)
    }

    /// Record a switch
    pub fn record_switch(&mut self, to_strategy: Uuid, regime: MarketRegime, reason: String) {
        let from_strategy = self.switch_history.last().map(|r| r.to_strategy);

        let record = SwitchRecord {
            id: Uuid::new_v4(),
            from_strategy,
            to_strategy,
            timestamp: Utc::now(),
            regime,
            reason,
            score_delta: 0.0, // Would be calculated from actual scores
        };

        self.switch_history.push(record);
        self.last_switch_time = Some(Utc::now());

        info!(
            "Strategy switch recorded: {:?} -> {}",
            from_strategy, to_strategy
        );
    }

    /// Get daily switch count
    fn daily_switch_count(&self) -> u32 {
        let today = Utc::now().date_naive();
        self.switch_history
            .iter()
            .filter(|r| r.timestamp.date_naive() == today)
            .count() as u32
    }

    /// Get recent switch count (within seconds)
    fn recent_switch_count(&self, seconds: i64) -> u32 {
        let cutoff = Utc::now() - chrono::Duration::seconds(seconds);
        self.switch_history
            .iter()
            .filter(|r| r.timestamp > cutoff)
            .count() as u32
    }

    /// Get switch history
    pub fn get_history(&self) -> &[SwitchRecord] {
        &self.switch_history
    }

    /// Get last switch time
    pub fn last_switch_time(&self) -> Option<DateTime<Utc>> {
        self.last_switch_time
    }

    /// Get time since last switch
    pub fn time_since_last_switch(&self) -> Option<i64> {
        self.last_switch_time
            .map(|t| (Utc::now() - t).num_seconds())
    }

    /// Can switch now (hold period met)
    pub fn can_switch_now(&self) -> bool {
        if let Some(last) = self.last_switch_time {
            let elapsed = (Utc::now() - last).num_seconds();
            elapsed >= self.config.min_hold_period_seconds
        } else {
            true // No previous switch
        }
    }

    /// Get remaining hold time
    pub fn remaining_hold_seconds(&self) -> i64 {
        if let Some(last) = self.last_switch_time {
            let elapsed = (Utc::now() - last).num_seconds();
            (self.config.min_hold_period_seconds - elapsed).max(0)
        } else {
            0
        }
    }

    /// Get switches by regime
    pub fn switches_by_regime(&self, regime: MarketRegime) -> Vec<&SwitchRecord> {
        self.switch_history
            .iter()
            .filter(|r| r.regime == regime)
            .collect()
    }

    /// Get total switch count
    pub fn total_switches(&self) -> usize {
        self.switch_history.len()
    }

    /// Clean old history (keep last N days)
    pub fn clean_history(&mut self, days: i64) {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        self.switch_history.retain(|r| r.timestamp > cutoff);
    }

    /// Update config
    pub fn update_config(&mut self, config: SwitchConfig) {
        self.config = config;
    }

    /// Get current config
    pub fn config(&self) -> &SwitchConfig {
        &self.config
    }
}

impl Default for StrategySwitcher {
    fn default() -> Self {
        Self::new(SwitchConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy_selector::StrategyType;

    fn create_test_score(
        strategy_type: StrategyType,
        overall: f32,
        confidence: f32,
    ) -> SelectionScore {
        SelectionScore {
            strategy_id: Uuid::new_v4(),
            strategy_type,
            regime_fit_score: 0.8,
            performance_score: 0.7,
            risk_adjusted_score: 0.6,
            recency_score: 0.9,
            overall_score: overall,
            confidence,
        }
    }

    #[test]
    fn test_switcher_creation() {
        let switcher = StrategySwitcher::new(SwitchConfig::default());
        assert_eq!(switcher.total_switches(), 0);
        assert!(switcher.can_switch_now());
    }

    #[test]
    fn test_should_switch_improvement() {
        let switcher = StrategySwitcher::new(SwitchConfig::default());

        let current = create_test_score(StrategyType::Momentum, 0.7, 0.8);
        let candidate = create_test_score(StrategyType::MeanReversion, 0.9, 0.8);

        // Should switch (20% improvement > 10% threshold)
        assert!(switcher.should_switch(&current, &candidate));
    }

    #[test]
    fn test_should_not_switch_insufficient_improvement() {
        let switcher = StrategySwitcher::new(SwitchConfig::default());

        let current = create_test_score(StrategyType::Momentum, 0.85, 0.8);
        let candidate = create_test_score(StrategyType::MeanReversion, 0.90, 0.8);

        // Should not switch (5% improvement < 10% threshold)
        assert!(!switcher.should_switch(&current, &candidate));
    }

    #[test]
    fn test_should_not_switch_low_confidence() {
        let switcher = StrategySwitcher::new(SwitchConfig::default());

        let current = create_test_score(StrategyType::Momentum, 0.5, 0.8);
        let candidate = create_test_score(StrategyType::MeanReversion, 0.9, 0.5); // Low confidence

        // Should not switch (confidence 0.5 < 0.7 threshold)
        assert!(!switcher.should_switch(&current, &candidate));
    }

    #[test]
    fn test_record_switch() {
        let mut switcher = StrategySwitcher::new(SwitchConfig::default());
        let strategy_id = Uuid::new_v4();

        switcher.record_switch(
            strategy_id,
            MarketRegime::Trending,
            "Better performance".to_string(),
        );

        assert_eq!(switcher.total_switches(), 1);
        assert!(!switcher.can_switch_now()); // Hold period not met
    }

    #[test]
    fn test_daily_switch_limit() {
        let config = SwitchConfig {
            max_switches_per_day: 2,
            min_hold_period_seconds: 0, // No hold period for test
            ..Default::default()
        };
        let mut switcher = StrategySwitcher::new(config);

        // Record max switches
        switcher.record_switch(Uuid::new_v4(), MarketRegime::Trending, "Test1".to_string());
        switcher.record_switch(Uuid::new_v4(), MarketRegime::Ranging, "Test2".to_string());

        // Try to switch again
        let current = create_test_score(StrategyType::Momentum, 0.5, 0.9);
        let candidate = create_test_score(StrategyType::MeanReversion, 0.9, 0.9);

        assert!(!switcher.should_switch(&current, &candidate));
    }

    #[test]
    fn test_switches_by_regime() {
        let mut switcher = StrategySwitcher::new(SwitchConfig::default());

        switcher.record_switch(Uuid::new_v4(), MarketRegime::Trending, "T1".to_string());
        switcher.record_switch(Uuid::new_v4(), MarketRegime::Trending, "T2".to_string());
        switcher.record_switch(Uuid::new_v4(), MarketRegime::Ranging, "R1".to_string());

        let trending_switches = switcher.switches_by_regime(MarketRegime::Trending);
        assert_eq!(trending_switches.len(), 2);

        let ranging_switches = switcher.switches_by_regime(MarketRegime::Ranging);
        assert_eq!(ranging_switches.len(), 1);
    }
}
