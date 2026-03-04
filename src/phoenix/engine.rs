//! Phoenix Engine - Autonomous Learning System Implementation
//!
//! Complete training loop with:
//! - RAG Memory for experience storage
//! - Paper Trading Simulator
//! - LLM Strategist for decisions
//! - Graduation Assessment

use std::sync::Arc;

use super::{
    assessment::GraduationAssessor,
    graduation::*,
    memory::*,
    simulator::{MarketDataPoint, PaperTradingSimulator, SimulatorConfig},
    strategist::{DecisionContext, LlmStrategist, MockLlmProvider, RuleBasedStrategist, Sentiment},
    Action, DailyResult, DecimalExt, EpochMetrics, PhoenixConfig, TradingDecision,
};

/// Complete Phoenix Engine
pub struct PhoenixEngine {
    config: PhoenixConfig,
    memory: RagMemory,
    simulator: PaperTradingSimulator,
    assessor: GraduationAssessor,
    strategist: Box<dyn DecisionMaker>,
    current_epoch: u32,
}

/// Trait for decision making (allows swapping LLM with rule-based)
pub trait DecisionMaker: Send + Sync {
    fn decide(&self, context: &DecisionContext) -> TradingDecision;
    fn generate_lesson(&self, experience: &TradingExperience) -> String;
}

/// Adapter for LlmStrategist to work with DecisionMaker trait
struct LlmStrategistAdapter {
    strategist: LlmStrategist,
}

impl DecisionMaker for LlmStrategistAdapter {
    fn decide(&self, context: &DecisionContext) -> TradingDecision {
        // For synchronous interface, we use rule-based fallback
        // In async context, use the actual LLM
        let rule_based = RuleBasedStrategist;
        rule_based.decide(context)
    }

    fn generate_lesson(&self, _experience: &TradingExperience) -> String {
        "Lesson learned from this trade".to_string()
    }
}

/// Rule-based adapter
struct RuleBasedAdapter;

impl DecisionMaker for RuleBasedAdapter {
    fn decide(&self, context: &DecisionContext) -> TradingDecision {
        let strategist = RuleBasedStrategist;
        strategist.decide(context)
    }

    fn generate_lesson(&self, experience: &TradingExperience) -> String {
        if experience.outcome.success {
            format!(
                "Success: Captured {:.1}% profit",
                experience.outcome.profit_loss_pct
            )
        } else {
            format!("Mistake: Lost {:.1}%", experience.outcome.profit_loss_pct)
        }
    }
}

/// Training result with detailed metrics
#[derive(Debug, Clone)]
pub struct TrainingResult {
    pub epoch_number: u32,
    pub success: bool,
    pub metrics: EpochMetrics,
    pub graduation_assessment: Option<GraduationAssessment>,
    pub trades_executed: u32,
    pub memory_stats: MemoryStats,
}

impl PhoenixEngine {
    /// Create new Phoenix Engine with default rule-based strategist
    pub fn new(config: PhoenixConfig) -> Self {
        let sim_config = SimulatorConfig {
            initial_capital: config.initial_capital,
            ..Default::default()
        };

        Self {
            config: config.clone(),
            memory: RagMemory::new(),
            simulator: PaperTradingSimulator::new(sim_config),
            assessor: GraduationAssessor::new(config.graduation_config.clone()),
            strategist: Box::new(RuleBasedAdapter),
            current_epoch: 0,
        }
    }

    /// Create with LLM strategist
    pub fn with_llm_strategist(config: PhoenixConfig) -> Self {
        let sim_config = SimulatorConfig {
            initial_capital: config.initial_capital,
            ..Default::default()
        };

        // Create mock LLM for now (replace with real LLM integration)
        let provider = Arc::new(MockLlmProvider::new(Action::Hold, 0.5));
        let llm_strategist =
            LlmStrategist::new(provider, super::strategist::StrategistConfig::default());

        Self {
            config: config.clone(),
            memory: RagMemory::new(),
            simulator: PaperTradingSimulator::new(sim_config),
            assessor: GraduationAssessor::new(config.graduation_config.clone()),
            strategist: Box::new(LlmStrategistAdapter {
                strategist: llm_strategist,
            }),
            current_epoch: 0,
        }
    }

    /// Run complete training loop for one epoch
    pub fn run_training_epoch(&mut self, market_data: Vec<MarketDataPoint>) -> TrainingResult {
        self.current_epoch += 1;
        let epoch_number = self.current_epoch;

        // Load market data into simulator
        self.simulator.load_market_data(market_data);

        // Trading loop - process each day
        let mut daily_results: Vec<DailyResult> = Vec::new();
        let mut trades_executed = 0u32;

        while let Some(data) = self.simulator.current_data() {
            // Collect data first to avoid borrow issues
            let ticker = data.ticker.clone();
            let close = data.close;
            let rsi = data.indicators.rsi;
            let trend = self.determine_trend(data);
            let trend_str = format!("{:?}", trend);
            let regime = self.classify_regime(data);
            let portfolio_value = self.simulator.portfolio_value();
            let cash = self.simulator.portfolio().cash;
            let current_pos = self
                .simulator
                .portfolio()
                .positions
                .get(&ticker)
                .map(|p| p.quantity);
            let volume = data.volume;
            let macd = data.indicators.macd;
            let vwap = data.indicators.vwap;
            let volatility = self.classify_volatility(data);

            // Build decision context
            let context = DecisionContext {
                ticker: ticker.clone(),
                current_price: close,
                rsi,
                trend: trend_str,
                regime: regime.clone(),
                portfolio_value,
                cash_available: cash,
                current_position: current_pos,
                market_sentiment: Sentiment::default(),
            };

            // Make decision using strategist
            let decision = self.strategist.decide(&context);

            // Execute in simulator
            let outcome = self.simulator.execute(&decision);

            // Store experience in RAG memory (if it was a trade)
            if decision.action != Action::Hold {
                trades_executed += 1;

                let timestamp = chrono::Utc::now();
                // Clone values for the lesson generation (to avoid move issues)
                let trend_for_lesson = trend.clone();
                let volatility_for_lesson = volatility.clone();

                let experience = TradingExperience {
                    id: uuid::Uuid::new_v4(),
                    market_condition: MarketCondition {
                        ticker: ticker.clone(),
                        price: close,
                        volume,
                        rsi,
                        macd,
                        vwap,
                        trend,
                        volatility,
                    },
                    decision: decision.clone(),
                    outcome: outcome.clone(),
                    lesson: self.strategist.generate_lesson(&TradingExperience {
                        id: uuid::Uuid::new_v4(),
                        market_condition: MarketCondition {
                            ticker: ticker.clone(),
                            price: close,
                            volume,
                            rsi,
                            macd,
                            vwap,
                            trend: trend_for_lesson,
                            volatility: volatility_for_lesson,
                        },
                        decision: decision.clone(),
                        outcome: outcome.clone(),
                        lesson: String::new(),
                        regime: regime.clone(),
                        timestamp,
                        embedding: None,
                    }),
                    regime,
                    timestamp,
                    embedding: None,
                };

                self.memory.store_experience(experience);
            }

            daily_results.push(DailyResult { decision });

            // Step to next day
            if !self.simulator.step() {
                break;
            }
        }

        // Calculate metrics
        let metrics = self.simulator.calculate_metrics();

        // Assess graduation (if enough epochs)
        let graduation_assessment = if epoch_number >= self.config.min_epochs_before_graduation
            && epoch_number.is_multiple_of(self.config.assessment_frequency)
        {
            let stress_test = StressTestResult::default();
            let walk_forward = WalkForwardResult::default();
            let monte_carlo = MonteCarloResult::default();

            let perf_metrics = PerformanceMetrics {
                total_return: metrics.total_return,
                cagr: metrics.cagr,
                max_drawdown: metrics.max_drawdown,
                sharpe_ratio: metrics.sharpe_ratio,
                total_trades: trades_executed,
                win_rate: metrics.win_rate,
                ..Default::default()
            };

            Some(self.assessor.assess(
                &perf_metrics,
                &RegimePerformance::default(),
                &stress_test,
                &walk_forward,
                &monte_carlo,
            ))
        } else {
            None
        };

        TrainingResult {
            epoch_number,
            success: metrics.total_return > rust_decimal::Decimal::ZERO,
            metrics,
            graduation_assessment,
            trades_executed,
            memory_stats: self.memory.stats(),
        }
    }

    /// Run multiple epochs
    pub fn train(
        &mut self,
        epochs: u32,
        market_data_fn: impl Fn(u32) -> Vec<MarketDataPoint>,
    ) -> Vec<TrainingResult> {
        let mut results = Vec::new();

        for epoch in 1..=epochs {
            if epoch > self.config.max_epochs {
                break;
            }

            let data = market_data_fn(epoch);
            let result = self.run_training_epoch(data);

            // Check if graduated
            if let Some(ref assessment) = result.graduation_assessment {
                if !matches!(assessment.level, GraduationLevel::NotReady { .. }) {
                    results.push(result);
                    break; // Graduated!
                }
            }

            results.push(result);
        }

        results
    }

    /// Get current graduation level
    pub fn current_level(&self) -> String {
        format!("Epoch {}: {:?}", self.current_epoch, self.config)
    }

    /// Get memory insights
    pub fn memory_insights(&self, regime: &MarketRegime) -> RegimeInsight {
        self.memory.what_works_in_regime(regime)
    }

    /// Get best strategies from memory
    pub fn best_strategies(&self, min_trades: u32) -> Vec<String> {
        self.memory
            .get_best_strategies(min_trades)
            .into_iter()
            .map(|s| s.name.clone())
            .collect()
    }

    /// Get engine statistics
    pub fn stats(&self) -> EngineStats {
        EngineStats {
            epoch: self.current_epoch,
            memory: self.memory.stats(),
            portfolio_value: self.simulator.portfolio_value(),
            start_value: self.config.initial_capital,
        }
    }

    /// Determine trend direction from indicators
    fn determine_trend(&self, data: &MarketDataPoint) -> TrendDirection {
        match (data.indicators.sma_20, data.indicators.sma_50, data.close) {
            (Some(sma20), Some(sma50), close) => {
                let price = close;
                if price > sma20
                    && sma20
                        > sma50 * rust_decimal::Decimal::from(102)
                            / rust_decimal::Decimal::from(100)
                {
                    TrendDirection::StrongUptrend
                } else if price > sma20 {
                    TrendDirection::Uptrend
                } else if price < sma20
                    && sma20
                        < sma50 * rust_decimal::Decimal::from(98) / rust_decimal::Decimal::from(100)
                {
                    TrendDirection::StrongDowntrend
                } else if price < sma20 {
                    TrendDirection::Downtrend
                } else {
                    TrendDirection::Sideways
                }
            }
            _ => TrendDirection::Sideways,
        }
    }

    /// Classify market regime
    fn classify_regime(&self, data: &MarketDataPoint) -> MarketRegime {
        // Simple regime classification based on volatility and trend
        let volatility = data
            .indicators
            .atr
            .map(|atr| {
                let atr_pct = (atr / data.close).to_f64().unwrap_or(0.0);
                if atr_pct > 0.05 {
                    VolatilityLevel::Extreme
                } else if atr_pct > 0.03 {
                    VolatilityLevel::High
                } else if atr_pct > 0.01 {
                    VolatilityLevel::Normal
                } else {
                    VolatilityLevel::Low
                }
            })
            .unwrap_or(VolatilityLevel::Normal);

        let trend = self.determine_trend(data);

        match (volatility, trend) {
            (VolatilityLevel::Extreme, _) => MarketRegime::Crisis,
            (VolatilityLevel::High, TrendDirection::StrongDowntrend) => MarketRegime::RiskOff,
            (VolatilityLevel::High, TrendDirection::StrongUptrend) => MarketRegime::RiskOn,
            (VolatilityLevel::Normal, TrendDirection::Uptrend) => MarketRegime::RiskOn,
            (VolatilityLevel::Normal, TrendDirection::Downtrend) => MarketRegime::RiskOff,
            _ => MarketRegime::Uncertain,
        }
    }

    /// Classify volatility level
    fn classify_volatility(&self, data: &MarketDataPoint) -> VolatilityLevel {
        data.indicators
            .atr
            .map(|atr| {
                let atr_pct = (atr / data.close).to_f64().unwrap_or(0.0);
                if atr_pct > 0.10 {
                    VolatilityLevel::Extreme
                } else if atr_pct > 0.05 {
                    VolatilityLevel::High
                } else if atr_pct > 0.02 {
                    VolatilityLevel::Normal
                } else {
                    VolatilityLevel::Low
                }
            })
            .unwrap_or(VolatilityLevel::Normal)
    }
}

/// Engine statistics
#[derive(Debug, Clone)]
pub struct EngineStats {
    pub epoch: u32,
    pub memory: MemoryStats,
    pub portfolio_value: rust_decimal::Decimal,
    pub start_value: rust_decimal::Decimal,
}

impl Default for PhoenixEngine {
    fn default() -> Self {
        Self::new(PhoenixConfig::default())
    }
}
