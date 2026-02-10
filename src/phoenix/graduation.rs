//! Phoenix Graduation Criteria v2.0
//! 
//! Реалистична система за оценка готовността на AI трейдинг система
//! за live trading с постепенно увеличаване на капитала.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// ==================== КОНФИГУРАЦИЯ ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraduationConfig {
    /// Начален капитал за симулация
    pub initial_capital: Decimal,
    
    /// Реалистични CAGR цели по нива
    pub cagr_targets: CagrTargets,
    
    /// Риск лимити
    pub risk_limits: RiskLimits,
    
    /// Статистически изисквания
    pub statistical_requirements: StatisticalRequirements,
    
    /// Period конфигурация
    pub periods: PeriodConfig,
    
    /// Разходи и такси
    pub cost_model: CostModel,
    
    /// Stress test конфигурация
    pub stress_test: StressTestConfig,
}

impl Default for GraduationConfig {
    fn default() -> Self {
        Self {
            initial_capital: Decimal::from(1000),
            cagr_targets: CagrTargets::default(),
            risk_limits: RiskLimits::default(),
            statistical_requirements: StatisticalRequirements::default(),
            periods: PeriodConfig::default(),
            cost_model: CostModel::default(),
            stress_test: StressTestConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CagrTargets {
    /// Минимален CAGR за ниво 1 (Paper) - 15%
    pub level1_min: f64,
    /// Целеви CAGR за ниво 2 (Micro) - 20%
    pub level2_target: f64,
    /// Целеви CAGR за ниво 3 (Small) - 25%
    pub level3_target: f64,
    /// Оптимален CAGR за ниво 4 (Full) - 30%
    pub level4_optimal: f64,
    /// Максимален CAGR (преди съмнения за overfitting) - 50%
    pub max_suspicious: f64,
}

impl Default for CagrTargets {
    fn default() -> Self {
        Self {
            level1_min: 0.15,        // 15%
            level2_target: 0.20,     // 20%
            level3_target: 0.25,     // 25%
            level4_optimal: 0.30,    // 30%
            max_suspicious: 0.50,    // 50% - над това = overfitting alarm
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLimits {
    /// Максимален drawdown за всяко ниво
    pub max_drawdown: Decimal,
    /// Максимален daily loss
    pub max_daily_loss: Decimal,
    /// Максимален weekly loss
    pub max_weekly_loss: Decimal,
    /// Risk of ruin (Kelly/Optimal F)
    pub max_risk_of_ruin: f64,
    /// Максимален beta към пазара
    pub max_beta: f64,
    /// Минимален Calmar ratio
    pub min_calmar: f64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_drawdown: Decimal::from_str_exact("0.15").unwrap_or(Decimal::from(15) / Decimal::from(100)),
            max_daily_loss: Decimal::from_str_exact("0.03").unwrap_or(Decimal::from(3) / Decimal::from(100)),
            max_weekly_loss: Decimal::from_str_exact("0.05").unwrap_or(Decimal::from(5) / Decimal::from(100)),
            max_risk_of_ruin: 0.01,
            max_beta: 0.7,
            min_calmar: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalRequirements {
    /// Минимален брой трейдове за статистическа значимост
    pub min_total_trades: u32,
    /// Минимален брой трейдове на месец
    pub min_trades_per_month: u32,
    /// Минимален брой печеливши месеци
    pub min_profitable_months: u32,
    /// Минимален % печеливши месеца
    pub min_profitable_month_pct: f64,
    /// Минимален Sharpe ratio
    pub min_sharpe: f64,
    /// Минимален Sortino ratio
    pub min_sortino: f64,
    /// Минимален win rate
    pub min_win_rate: f64,
    /// Минимален profit factor
    pub min_profit_factor: f64,
    /// Максимален payoff ratio (за да избегнем lottery tickets)
    pub max_payoff_ratio: f64,
}

impl Default for StatisticalRequirements {
    fn default() -> Self {
        Self {
            min_total_trades: 100,
            min_trades_per_month: 4,
            min_profitable_months: 6,
            min_profitable_month_pct: 0.60,
            min_sharpe: 1.2,
            min_sortino: 1.8,
            min_win_rate: 0.52,
            min_profit_factor: 1.3,
            max_payoff_ratio: 5.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodConfig {
    /// In-sample период (тренировка)
    pub in_sample_years: u32,
    /// Out-of-sample период (тест)
    pub out_of_sample_years: u32,
    /// Walk-forward window (train)
    pub walk_forward_train_months: u32,
    /// Walk-forward window (test)
    pub walk_forward_test_months: u32,
    /// Минимален период за live paper trading
    pub min_paper_trading_months: u32,
}

impl Default for PeriodConfig {
    fn default() -> Self {
        Self {
            in_sample_years: 5,
            out_of_sample_years: 2,
            walk_forward_train_months: 24,
            walk_forward_test_months: 6,
            min_paper_trading_months: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModel {
    /// Комисионна на трейд
    pub commission_per_trade: Decimal,
    /// Slippage (процент)
    pub slippage_pct: Decimal,
    /// Spread (процент)
    pub spread_pct: Decimal,
    /// Market impact за големи поръчки
    pub market_impact_factor: Decimal,
    /// Borrow cost за short positions
    pub borrow_cost_annual: Decimal,
    /// Finra fees (за US)
    pub finra_fee_per_share: Decimal,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            commission_per_trade: Decimal::from_str_exact("1.00").unwrap_or(Decimal::from(1)),
            slippage_pct: Decimal::from_str_exact("0.001").unwrap_or(Decimal::from(1) / Decimal::from(1000)),
            spread_pct: Decimal::from_str_exact("0.0005").unwrap_or(Decimal::from(5) / Decimal::from(10000)),
            market_impact_factor: Decimal::from_str_exact("0.0001").unwrap_or(Decimal::from(1) / Decimal::from(10000)),
            borrow_cost_annual: Decimal::from_str_exact("0.03").unwrap_or(Decimal::from(3) / Decimal::from(100)),
            finra_fee_per_share: Decimal::from_str_exact("0.000145").unwrap_or(Decimal::from(145) / Decimal::from(1000000)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestConfig {
    /// Сценарии за тестване
    pub scenarios: Vec<StressScenario>,
    /// Минимален survival rate (% от капитала оцелял)
    pub min_survival_rate: f64,
    /// Максимална загуба в най-лошия сценарий
    pub max_crisis_drawdown: Decimal,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        use StressScenarioType::*;
        Self {
            scenarios: vec![
                StressScenario::new(CovidCrash2020, "2020-02-19", "2020-03-23", -0.35),
                StressScenario::new(GFC2008, "2007-10-09", "2009-03-09", -0.57),
                StressScenario::new(DotComCrash, "2000-03-10", "2002-10-09", -0.78),
                StressScenario::new(FlashCrash2010, "2010-05-06", "2010-05-06", -0.10),
                StressScenario::new(RateShock1994, "1994-01-01", "1994-12-31", -0.15),
                StressScenario::new(BlackMonday1987, "1987-10-19", "1987-10-19", -0.22),
                StressScenario::new(RussiaDefault1998, "1998-08-01", "1998-10-01", -0.25),
                StressScenario::new(CovidRecovery, "2020-03-23", "2021-12-31", 1.00),
            ],
            min_survival_rate: 0.70,
            max_crisis_drawdown: Decimal::from_str_exact("0.30").unwrap_or(Decimal::from(30) / Decimal::from(100)),
        }
    }
}

/// ==================== СТРУКТУРИ ЗА РЕЗУЛТАТИ ====================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    // Основни метрики
    pub total_return: Decimal,
    pub cagr: f64,
    pub volatility: f64,
    pub max_drawdown: Decimal,
    pub max_drawdown_duration_days: u32,
    
    // Risk-adjusted
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub treynor_ratio: f64,
    pub information_ratio: f64,
    
    // Trade statistics
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub win_rate: f64,
    pub avg_win: Decimal,
    pub avg_loss: Decimal,
    pub payoff_ratio: f64,
    pub profit_factor: f64,
    pub expectancy: f64,
    
    // Consistency
    pub monthly_returns: Vec<MonthlyReturn>,
    pub profitable_months: u32,
    pub profitable_month_pct: f64,
    pub avg_monthly_return: f64,
    pub monthly_return_std: f64,
    pub max_consecutive_loss_months: u32,
    
    // Correlation & Beta
    pub correlation_with_market: f64,
    pub beta: f64,
    pub alpha: f64,
    
    // Costs
    pub gross_return: Decimal,
    pub total_costs: Decimal,
    pub net_return: Decimal,
    pub cost_impact_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyReturn {
    pub year: i32,
    pub month: u8,
    pub return_pct: f64,
    pub trades_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimePerformance {
    pub risk_on: RegimeMetrics,
    pub uncertain: RegimeMetrics,
    pub risk_off: RegimeMetrics,
    pub crisis: RegimeMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegimeMetrics {
    pub period_count: u32,
    pub avg_return: f64,
    pub win_rate: f64,
    pub max_drawdown: Decimal,
    pub sharpe: f64,
}

#[derive(Debug, Clone, Default)]
pub struct WalkForwardResult {
    pub windows_tested: u32,
    pub windows_profitable: u32,
    pub avg_correlation: f64,
    pub consistency_score: f64,
    pub is_consistent: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MonteCarloResult {
    pub simulations: u32,
    pub survival_rate: f64,
    pub median_cagr: f64,
    pub percentile_5: f64,
    pub percentile_95: f64,
    pub probability_of_ruin: f64,
    pub probability_target_cagr: f64,
}

#[derive(Debug, Clone, Default)]
pub struct StressTestResult {
    pub scenario_results: Vec<ScenarioResult>,
    pub worst_scenario: String,
    pub worst_drawdown: Decimal,
    pub avg_drawdown: Decimal,
    pub survival_rate: f64,
    pub passed: bool,
}

#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub scenario: StressScenarioType,
    pub start_capital: Decimal,
    pub end_capital: Decimal,
    pub return_pct: f64,
    pub max_drawdown: Decimal,
    pub survived: bool,
}

/// ==================== НИВА НА ДИПЛОМИРАНЕ ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GraduationLevel {
    /// Ниво 0: Не е готово
    NotReady {
        reasons: Vec<FailReason>,
        recommendations: Vec<String>,
    },
    
    /// Ниво 1: Paper Trading Approved
    PaperTrading {
        max_position_size: Decimal,
        max_positions: u32,
        max_leverage: f64,
        duration_months: u32,
    },
    
    /// Ниво 2: Micro Live
    MicroLive {
        max_capital: Decimal,
        max_position_size: Decimal,
        max_daily_loss: Decimal,
        max_positions: u32,
        duration_months: u32,
    },
    
    /// Ниво 3: Small Live
    SmallLive {
        max_capital: Decimal,
        max_position_size: Decimal,
        max_portfolio_heat: Decimal,
        allow_margin: bool,
        duration_months: u32,
    },
    
    /// Ниво 4: Full Strategy
    FullStrategy {
        max_capital: Decimal,
        max_position_pct: Decimal,
        allow_options: bool,
        allow_short: bool,
        allow_margin: bool,
    },
    
    /// Ниво 5: Master Level
    MasterLevel {
        max_aum: Decimal,
        can_manage_others_capital: bool,
        track_record_years: f64,
    },
}

impl GraduationLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::NotReady { .. } => "Not Ready",
            Self::PaperTrading { .. } => "Paper Trading",
            Self::MicroLive { .. } => "Micro Live",
            Self::SmallLive { .. } => "Small Live",
            Self::FullStrategy { .. } => "Full Strategy",
            Self::MasterLevel { .. } => "Master Level",
        }
    }
    
    pub fn max_capital(&self) -> Option<Decimal> {
        match self {
            Self::PaperTrading { .. } => None,
            Self::MicroLive { max_capital, .. } => Some(*max_capital),
            Self::SmallLive { max_capital, .. } => Some(*max_capital),
            Self::FullStrategy { max_capital, .. } => Some(*max_capital),
            Self::MasterLevel { max_aum, .. } => Some(*max_aum),
            _ => None,
        }
    }
}

/// ==================== РЕЗУЛТАТ ОТ ОЦЕНКАТА ====================

#[derive(Debug, Clone)]
pub struct GraduationAssessment {
    pub level: GraduationLevel,
    pub overall_score: f64,
    pub metrics: PerformanceMetrics,
    pub regime_performance: RegimePerformance,
    pub walk_forward: WalkForwardResult,
    pub monte_carlo: MonteCarloResult,
    pub stress_test: StressTestResult,
    pub assessment_date: String,
    pub improvement_areas: Vec<ImprovementArea>,
}

#[derive(Debug, Clone)]
pub struct ImprovementArea {
    pub category: String,
    pub current_value: String,
    pub target_value: String,
    pub priority: Priority,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// ==================== ГРЕШКИ И ПРИЧИНИ ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FailReason {
    InsufficientReturns { current: f64, required: f64 },
    ExcessiveDrawdown { current: Decimal, max_allowed: Decimal },
    LowSharpe { current: f64, required: f64 },
    LowWinRate { current: f64, required: f64 },
    InsufficientTrades { current: u32, required: u32 },
    InsufficientProfitableMonths { current: u32, required: u32 },
    HighPayoffRatio { current: f64, max_allowed: f64 },
    HighBeta { current: f64, max_allowed: f64 },
    FailedStressTest { scenario: String, loss: Decimal },
    OverfittingDetected { in_sample_cagr: f64, out_of_sample_cagr: f64 },
    HighCostImpact { cost_pct: f64 },
    InconsistentWalkForward { correlation: f64 },
    HighRiskOfRuin { probability: f64 },
    PoorCrisisPerformance { regime: String, return_pct: f64 },
    SuspiciousPerformance { cagr: f64, explanation: String },
}

impl FailReason {
    pub fn description(&self) -> String {
        match self {
            Self::InsufficientReturns { current, required } => {
                format!("CAGR {:.1}% is below minimum {:.1}%", current * 100.0, required * 100.0)
            }
            Self::ExcessiveDrawdown { current, max_allowed } => {
                format!("Max drawdown {}% exceeds limit {}%", current, max_allowed)
            }
            Self::LowSharpe { current, required } => {
                format!("Sharpe ratio {:.2} is below required {:.2}", current, required)
            }
            Self::LowWinRate { current, required } => {
                format!("Win rate {:.1}% is below required {:.1}%", current * 100.0, required * 100.0)
            }
            Self::InsufficientTrades { current, required } => {
                format!("Only {} trades, need at least {} for statistical significance", current, required)
            }
            Self::InsufficientProfitableMonths { current, required } => {
                format!("Only {} profitable months, need at least {}", current, required)
            }
            Self::HighPayoffRatio { current, max_allowed } => {
                format!("Payoff ratio {:.2} suggests lottery-ticket strategy (max {})", current, max_allowed)
            }
            Self::HighBeta { current, max_allowed } => {
                format!("Beta {:.2} too high - just following the market (max {})", current, max_allowed)
            }
            Self::FailedStressTest { scenario, loss } => {
                format!("Failed stress test '{}': lost {}%", scenario, loss)
            }
            Self::OverfittingDetected { in_sample_cagr, out_of_sample_cagr } => {
                format!("Overfitting: in-sample {:.1}% vs out-of-sample {:.1}%", in_sample_cagr * 100.0, out_of_sample_cagr * 100.0)
            }
            Self::HighCostImpact { cost_pct } => {
                format!("Transaction costs eat {:.1}% of returns", cost_pct * 100.0)
            }
            Self::InconsistentWalkForward { correlation } => {
                format!("Walk-forward inconsistent: correlation only {:.2}", correlation)
            }
            Self::HighRiskOfRuin { probability } => {
                format!("Risk of ruin is {:.1}% - too high", probability * 100.0)
            }
            Self::PoorCrisisPerformance { regime, return_pct } => {
                format!("Poor performance in {} regime: {:.1}%", regime, return_pct * 100.0)
            }
            Self::SuspiciousPerformance { cagr, explanation } => {
                format!("Suspicious CAGR {:.1}%: {}", cagr * 100.0, explanation)
            }
        }
    }
}

/// ==================== STRESS SCENARIOS ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StressScenarioType {
    CovidCrash2020,
    GFC2008,
    DotComCrash,
    FlashCrash2010,
    RateShock1994,
    BlackMonday1987,
    RussiaDefault1998,
    CovidRecovery,
    Custom(String),
}

impl std::fmt::Display for StressScenarioType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CovidCrash2020 => write!(f, "COVID-19 Crash (2020)"),
            Self::GFC2008 => write!(f, "Global Financial Crisis (2008)"),
            Self::DotComCrash => write!(f, "Dot-Com Bubble Burst (2000-2002)"),
            Self::FlashCrash2010 => write!(f, "Flash Crash (2010)"),
            Self::RateShock1994 => write!(f, "Rate Shock (1994)"),
            Self::BlackMonday1987 => write!(f, "Black Monday (1987)"),
            Self::RussiaDefault1998 => write!(f, "Russia Default (1998)"),
            Self::CovidRecovery => write!(f, "COVID Recovery (2020-2021)"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressScenario {
    pub scenario_type: StressScenarioType,
    pub start_date: String,
    pub end_date: String,
    pub market_return: f64,
}

impl StressScenario {
    pub fn new(scenario_type: StressScenarioType, start: &str, end: &str, market_return: f64) -> Self {
        Self {
            scenario_type,
            start_date: start.to_string(),
            end_date: end.to_string(),
            market_return,
        }
    }
}
