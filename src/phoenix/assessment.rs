//! Graduation Assessment Engine
//! 
//! Evaluates if a strategy is ready for live trading based on
//! realistic criteria (v2.0 - NOT the impossible 200x goal)

use super::graduation::*;

/// Engine for assessing graduation readiness
pub struct GraduationAssessor {
    config: GraduationConfig,
}

impl GraduationAssessor {
    pub fn new(config: GraduationConfig) -> Self {
        Self { config }
    }
    
    /// Perform comprehensive graduation assessment
    pub fn assess(
        &self,
        metrics: &PerformanceMetrics,
        _regime_perf: &RegimePerformance,
        stress_test: &StressTestResult,
        walk_forward: &WalkForwardResult,
        monte_carlo: &MonteCarloResult,
    ) -> GraduationAssessment {
        let mut fail_reasons = vec![];
        let mut improvement_areas = vec![];
        
        // CHECK 1: CAGR (Realistic targets)
        let cagr = metrics.cagr;
        let cagr_target = self.config.cagr_targets.level1_min;
        let cagr_max = self.config.cagr_targets.max_suspicious;
        
        if cagr < cagr_target {
            fail_reasons.push(FailReason::InsufficientReturns {
                current: cagr,
                required: cagr_target,
            });
            improvement_areas.push(ImprovementArea {
                category: "Returns".to_string(),
                current_value: format!("{:.1}%", cagr * 100.0),
                target_value: format!("{:.1}%", cagr_target * 100.0),
                priority: Priority::Critical,
                suggestion: "Improve strategy profitability".to_string(),
            });
        } else if cagr > cagr_max {
            fail_reasons.push(FailReason::SuspiciousPerformance {
                cagr,
                explanation: format!("CAGR {:.1}% exceeds realistic maximum", cagr * 100.0),
            });
        }
        
        // CHECK 2: Risk Metrics
        if metrics.max_drawdown < -self.config.risk_limits.max_drawdown {
            fail_reasons.push(FailReason::ExcessiveDrawdown {
                current: metrics.max_drawdown,
                max_allowed: self.config.risk_limits.max_drawdown,
            });
        }
        
        if metrics.sharpe_ratio < self.config.statistical_requirements.min_sharpe {
            fail_reasons.push(FailReason::LowSharpe {
                current: metrics.sharpe_ratio,
                required: self.config.statistical_requirements.min_sharpe,
            });
        }
        
        // CHECK 3: Statistical Significance
        if metrics.total_trades < self.config.statistical_requirements.min_total_trades {
            fail_reasons.push(FailReason::InsufficientTrades {
                current: metrics.total_trades,
                required: self.config.statistical_requirements.min_total_trades,
            });
        }
        
        // CHECK 3b: Payoff Ratio (avoid lottery-ticket strategies)
        if metrics.payoff_ratio > self.config.statistical_requirements.max_payoff_ratio {
            fail_reasons.push(FailReason::HighPayoffRatio {
                current: metrics.payoff_ratio,
                max_allowed: self.config.statistical_requirements.max_payoff_ratio,
            });
        }
        
        // CHECK 4: Stress Testing
        if !stress_test.passed {
            fail_reasons.push(FailReason::FailedStressTest {
                scenario: stress_test.worst_scenario.clone(),
                loss: stress_test.worst_drawdown,
            });
        }
        
        // CHECK 5: Walk-Forward Validation
        if !walk_forward.is_consistent {
            fail_reasons.push(FailReason::InconsistentWalkForward {
                correlation: walk_forward.avg_correlation,
            });
        }
        
        // CHECK 6: Risk of Ruin
        if monte_carlo.probability_of_ruin > self.config.risk_limits.max_risk_of_ruin {
            fail_reasons.push(FailReason::HighRiskOfRuin {
                probability: monte_carlo.probability_of_ruin,
            });
        }
        
        // DETERMINE GRADUATION LEVEL
        let level = if !fail_reasons.is_empty() {
            let recommendations = improvement_areas.iter()
                .map(|a| format!("[{:?}] {} -> {}: {}", 
                    a.priority, a.current_value, a.target_value, a.suggestion))
                .collect();
            
            GraduationLevel::NotReady {
                reasons: fail_reasons.clone(),
                recommendations,
            }
        } else {
            self.determine_graduation_level(metrics)
        };
        
        // Calculate overall score
        let overall_score = self.calculate_overall_score(
            metrics,
            stress_test,
            walk_forward,
            monte_carlo,
            &fail_reasons,
        );
        
        GraduationAssessment {
            level,
            overall_score,
            metrics: metrics.clone(),
            regime_performance: RegimePerformance::default(),
            walk_forward: walk_forward.clone(),
            monte_carlo: monte_carlo.clone(),
            stress_test: stress_test.clone(),
            assessment_date: chrono::Utc::now().to_rfc3339(),
            improvement_areas,
        }
    }
    
    fn determine_graduation_level(&self, metrics: &PerformanceMetrics) -> GraduationLevel {
        use rust_decimal::Decimal;
        
        let cagr = metrics.cagr;
        let sharpe = metrics.sharpe_ratio;
        let months = metrics.monthly_returns.len() as u32;
        
        // Level 5: Master Level (3+ years track record)
        if months >= 36 && cagr >= 0.30 && sharpe >= 1.5 {
            return GraduationLevel::MasterLevel {
                max_aum: Decimal::from(1000000),
                can_manage_others_capital: true,
                track_record_years: months as f64 / 12.0,
            };
        }
        
        // Level 4: Full Strategy (12+ months, 25%+ CAGR)
        if months >= 12 && cagr >= 0.25 && sharpe >= 1.3 {
            return GraduationLevel::FullStrategy {
                max_capital: Decimal::from(50000),
                max_position_pct: Decimal::from(5) / Decimal::from(100),
                allow_options: true,
                allow_short: true,
                allow_margin: true,
            };
        }
        
        // Level 3: Small Live (6+ months, 20%+ CAGR)
        if months >= 6 && cagr >= 0.20 && sharpe >= 1.2 {
            return GraduationLevel::SmallLive {
                max_capital: Decimal::from(5000),
                max_position_size: Decimal::from(500),
                max_portfolio_heat: Decimal::from(20) / Decimal::from(100),
                allow_margin: false,
                duration_months: 6,
            };
        }
        
        // Level 2: Micro Live (3+ months, 15%+ CAGR)
        if months >= 3 && cagr >= 0.15 && sharpe >= 1.0 {
            return GraduationLevel::MicroLive {
                max_capital: Decimal::from(1000),
                max_position_size: Decimal::from(100),
                max_daily_loss: Decimal::from(50),
                max_positions: 2,
                duration_months: 3,
            };
        }
        
        // Level 1: Paper Trading
        GraduationLevel::PaperTrading {
            max_position_size: Decimal::from(1000),
            max_positions: 5,
            max_leverage: 1.0,
            duration_months: 6,
        }
    }
    
    fn calculate_overall_score(
        &self,
        metrics: &PerformanceMetrics,
        stress: &StressTestResult,
        walk_forward: &WalkForwardResult,
        monte_carlo: &MonteCarloResult,
        fail_reasons: &[FailReason],
    ) -> f64 {
        let mut score = 0.0;
        let mut weights = 0.0;
        
        // Returns (30%)
        let return_score = (metrics.cagr / 0.30).min(1.0);
        score += return_score * 0.30;
        weights += 0.30;
        
        // Risk-adjusted (25%)
        let risk_score = (metrics.sharpe_ratio / 2.0).min(1.0);
        score += risk_score * 0.25;
        weights += 0.25;
        
        // Consistency (15%)
        score += walk_forward.consistency_score * 0.15;
        weights += 0.15;
        
        // Stress test (15%)
        score += stress.survival_rate * 0.15;
        weights += 0.15;
        
        // Monte Carlo (10%)
        score += monte_carlo.survival_rate * 0.10;
        weights += 0.10;
        
        // Penalize for failures
        let failure_penalty = fail_reasons.len() as f64 * 0.05;
        
        (score / weights - failure_penalty).max(0.0)
    }
}

impl Default for GraduationAssessment {
    fn default() -> Self {
        Self {
            level: GraduationLevel::NotReady {
                reasons: vec![],
                recommendations: vec![],
            },
            overall_score: 0.0,
            metrics: PerformanceMetrics::default(),
            regime_performance: RegimePerformance::default(),
            walk_forward: WalkForwardResult::default(),
            monte_carlo: MonteCarloResult::default(),
            stress_test: StressTestResult::default(),
            assessment_date: chrono::Utc::now().to_rfc3339(),
            improvement_areas: vec![],
        }
    }
}
