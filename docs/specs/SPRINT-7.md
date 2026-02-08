# Sprint 7: Advanced Analytics & Backtesting

> **Duration:** Week 13-14
> **Goal:** Backtesting framework, risk analytics, and ML-enhanced signals
> **Ref:** [SPEC-v1.0](./SPEC-v1.0.md) | [Golden Path](./GOLDEN-PATH.md) | [ROADMAP](../ROADMAP.md)

---

## Scope

### ✅ In Scope
- Backtesting framework with walk-forward analysis
- Transaction cost modeling
- Risk analytics (VaR, Sharpe, drawdown)
- Performance attribution
- ML feature engineering
- XGBoost CQ prediction model
- Anomaly detection for regime changes

### ❌ Out of Scope
- High-frequency trading (HFT)
- Alternative data integration (satellite, credit cards)
- Real-time ML inference (batch only)
- Portfolio optimization (Markowitz)

---

## Deliverables

| ID | Deliverable | Acceptance Criteria |
|----|-------------|---------------------|
| S7-D1 | Backtesting engine | Walk-forward, transaction costs, slippage |
| S7-D2 | Risk metrics | VaR, CVaR, Sharpe, Sortino, Calmar |
| S7-D3 | Performance attribution | Returns breakdown by factor |
| S7-D4 | ML feature pipeline | Feature engineering from signals |
| S7-D5 | XGBoost CQ model | Train/test split, >70% accuracy |
| S7-D6 | Anomaly detection | Regime change alerts |
| S7-D7 | Backtest API | `/api/backtest/run`, `/api/backtest/results` |
| S7-D8 | Analytics dashboard | Grafana panels for risk metrics |

---

## Technical Implementation

### S7-D1: Backtesting Engine

```rust
// crates/investor-analytics/src/backtest.rs
pub struct Backtest {
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    initial_capital: Money,
    strategy: Box<dyn Strategy>,
    transaction_cost: TransactionCostModel,
    slippage: SlippageModel,
    rebalance_frequency: Duration,
}

impl Backtest {
    pub async fn run(&self) -> Result<BacktestResult> {
        let mut portfolio = Portfolio::with_capital(self.initial_capital);
        let mut daily_results = Vec::new();
        
        // Walk-forward analysis
        let mut current_date = self.start_date;
        while current_date < self.end_date {
            // Get signals for universe
            let signals = self.get_signals(current_date).await?;
            
            // Generate proposals
            let proposals = self.strategy.generate_proposals(&signals, &portfolio);
            
            // Execute (with costs)
            for proposal in proposals {
                let execution = self.simulate_execution(&proposal, current_date).await?;
                portfolio.apply_execution(execution);
            }
            
            // Record daily snapshot
            daily_results.push(DailyResult {
                date: current_date,
                nav: portfolio.nav(),
                positions: portfolio.positions().len(),
            });
            
            current_date += self.rebalance_frequency;
        }
        
        Ok(BacktestResult {
            total_return: self.calculate_return(&daily_results),
            sharpe_ratio: self.calculate_sharpe(&daily_results),
            max_drawdown: self.calculate_max_drawdown(&daily_results),
            daily_results,
        })
    }
    
    async fn simulate_execution(&self, proposal: &TradeProposal, date: DateTime<Utc>) -> Result<Execution> {
        // Get historical price for date
        let price = self.get_historical_price(&proposal.ticker, date).await?;
        
        // Apply slippage
        let slippage = self.slippage.apply(proposal, price);
        
        // Apply transaction costs
        let commission = self.transaction_cost.calculate(proposal, price);
        
        Ok(Execution {
            ticker: proposal.ticker.clone(),
            shares: proposal.shares,
            price: price + slippage,
            commission,
            timestamp: date,
        })
    }
}

pub struct WalkForwardConfig {
    pub train_window: Duration,      // 252 days (1 year)
    pub test_window: Duration,       // 63 days (1 quarter)
    pub step_size: Duration,         // 63 days
}

impl Backtest {
    pub async fn walk_forward(&self, config: &WalkForwardConfig) -> Result<WalkForwardResult> {
        let mut results = Vec::new();
        
        let mut train_start = self.start_date;
        while train_start + config.train_window + config.test_window < self.end_date {
            let train_end = train_start + config.train_window;
            let test_end = train_end + config.test_window;
            
            // Train model on training period
            let model = self.train_model(train_start, train_end).await?;
            
            // Test on test period
            let test_result = self.test_model(&model, train_end, test_end).await?;
            results.push(test_result);
            
            train_start += config.step_size;
        }
        
        Ok(WalkForwardResult { periods: results })
    }
}
```

### S7-D2: Risk Metrics

```rust
// crates/investor-analytics/src/risk.rs
pub struct RiskAnalyzer {
    returns: Vec<f64>,
    risk_free_rate: f64,
}

impl RiskAnalyzer {
    /// Value at Risk (historical simulation)
    pub fn var(&self, confidence: f64) -> f64 {
        let mut sorted_returns = self.returns.clone();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((1.0 - confidence) * sorted_returns.len() as f64) as usize;
        sorted_returns[index]
    }
    
    /// Conditional VaR (Expected Shortfall)
    pub fn cvar(&self, confidence: f64) -> f64 {
        let var = self.var(confidence);
        let tail_returns: Vec<f64> = self.returns.iter()
            .filter(|r| **r <= var)
            .copied()
            .collect();
        
        tail_returns.iter().sum::<f64>() / tail_returns.len() as f64
    }
    
    /// Sharpe ratio
    pub fn sharpe(&self) -> f64 {
        let excess_returns: Vec<f64> = self.returns.iter()
            .map(|r| r - self.risk_free_rate / 252.0)  // Daily risk-free
            .collect();
        
        let mean = excess_returns.iter().sum::<f64>() / excess_returns.len() as f64;
        let std = self.standard_deviation(&excess_returns);
        
        mean / std * (252.0_f64).sqrt()  // Annualized
    }
    
    /// Sortino ratio (downside deviation only)
    pub fn sortino(&self) -> f64 {
        let mean_return = self.returns.iter().sum::<f64>() / self.returns.len() as f64;
        let downside_returns: Vec<f64> = self.returns.iter()
            .filter(|r| **r < self.risk_free_rate / 252.0)
            .map(|r| (r - self.risk_free_rate / 252.0).powi(2))
            .collect();
        
        let downside_std = (downside_returns.iter().sum::<f64>() / downside_returns.len() as f64).sqrt();
        
        (mean_return - self.risk_free_rate / 252.0) / downside_std * (252.0_f64).sqrt()
    }
    
    /// Maximum drawdown
    pub fn max_drawdown(&self) -> f64 {
        let mut peak = 1.0;
        let mut max_dd = 0.0;
        
        let mut cumulative = 1.0;
        for ret in &self.returns {
            cumulative *= 1.0 + ret;
            if cumulative > peak {
                peak = cumulative;
            }
            let dd = (peak - cumulative) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
        
        -max_dd
    }
    
    /// Calmar ratio (return / max drawdown)
    pub fn calmar(&self) -> f64 {
        let annual_return = self.returns.iter().sum::<f64>() / self.returns.len() as f64 * 252.0;
        let max_dd = self.max_drawdown().abs();
        
        annual_return / max_dd
    }
}
```

### S7-D3: Performance Attribution

```rust
// crates/investor-analytics/src/attribution.rs
pub struct PerformanceAttribution {
    pub factor_returns: HashMap<String, f64>,
    pub selection_return: f64,
    pub timing_return: f64,
    pub total_return: f64,
}

impl PerformanceAttribution {
    /// Brinson-Fachler attribution model
    pub fn calculate(
        portfolio_weights: &HashMap<String, f64>,
        benchmark_weights: &HashMap<String, f64>,
        portfolio_returns: &HashMap<String, f64>,
        benchmark_returns: &HashMap<String, f64>,
    ) -> Self {
        // Allocation effect
        let allocation_effect: f64 = portfolio_weights.iter()
            .map(|(sector, w_p)| {
                let w_b = benchmark_weights.get(sector).unwrap_or(&0.0);
                let r_b = benchmark_returns.get(sector).unwrap_or(&0.0);
                (w_p - w_b) * r_b
            })
            .sum();
        
        // Selection effect
        let selection_effect: f64 = portfolio_weights.iter()
            .map(|(sector, w_p)| {
                let r_p = portfolio_returns.get(sector).unwrap_or(&0.0);
                let r_b = benchmark_returns.get(sector).unwrap_or(&0.0);
                w_p * (r_p - r_b)
            })
            .sum();
        
        // Interaction effect
        let interaction_effect: f64 = portfolio_weights.iter()
            .map(|(sector, w_p)| {
                let w_b = benchmark_weights.get(sector).unwrap_or(&0.0);
                let r_p = portfolio_returns.get(sector).unwrap_or(&0.0);
                let r_b = benchmark_returns.get(sector).unwrap_or(&0.0);
                (w_p - w_b) * (r_p - r_b)
            })
            .sum();
        
        Self {
            factor_returns: HashMap::new(),
            selection_return: selection_effect,
            timing_return: allocation_effect,
            total_return: allocation_effect + selection_effect + interaction_effect,
        }
    }
}
```

### S7-D4: ML Feature Pipeline

```rust
// crates/investor-analytics/src/features.rs
pub struct FeatureEngineering;

impl FeatureEngineering {
    /// Generate features from signals for ML model
    pub fn generate_features(signals: &TickerSignals) -> Vec<f64> {
        vec![
            // Quality features
            signals.quality_score.inner() as f64,
            signals.value_score.inner() as f64,
            signals.momentum_score.inner() as f64,
            
            // Insider features
            signals.insider_score.inner() as f64,
            signals.insider_flow_ratio,
            signals.insider_cluster_signal,
            
            // Sentiment features
            signals.sentiment_score.inner() as f64,
            signals.news_sentiment,
            signals.social_sentiment,
            
            // Regime features
            signals.regime_fit.inner() as f64,
            signals.vix_level,
            signals.market_breadth,
            
            // Technical features
            signals.breakout_score,
            signals.atr_trend,
            signals.rsi_14,
            signals.macd_signal,
            
            // Interactions
            signals.quality_score.inner() * signals.value_score.inner(),
            signals.momentum_score.inner() * signals.regime_fit.inner(),
        ]
    }
    
    /// Generate time-series features (rolling)
    pub fn generate_ts_features(price_history: &[PriceBar]) -> Vec<f64> {
        let closes: Vec<f64> = price_history.iter().map(|p| p.close).collect();
        
        vec![
            // Returns
            Self::calculate_return(&closes, 1),
            Self::calculate_return(&closes, 5),
            Self::calculate_return(&closes, 20),
            
            // Volatility
            Self::calculate_volatility(&closes, 20),
            
            // Moving averages
            Self::calculate_sma(&closes, 20),
            Self::calculate_sma(&closes, 50),
            
            // Technical indicators
            Self::calculate_rsi(&closes, 14),
        ]
    }
}
```

### S7-D5: XGBoost CQ Model

```rust
// crates/investor-analytics/src/ml.rs
use xgboost::{DMatrix, Booster, parameters};

pub struct CQPredictionModel {
    booster: Booster,
}

impl CQPredictionModel {
    pub fn train(
        features: &DMatrix,
        labels: &[f32],
        validation_split: f64,
    ) -> Result<Self> {
        let train_params = parameters::TrainingParametersBuilder::default()
            .dtrain(features)
            .objective(parameters::Objective::RegSquareError)
            .eval_metric(parameters::EvalMetric::RMSE)
            .max_depth(6)
            .eta(0.1)
            .subsample(0.8)
            .colsample_bytree(0.8)
            .build()?;
        
        let booster = Booster::train(&train_params)?;
        
        Ok(Self { booster })
    }
    
    pub fn predict(&self, features: &DMatrix) -> Result<Vec<f32>> {
        self.booster.predict(features)
    }
    
    /// Feature importance
    pub fn feature_importance(&self) -> Result<Vec<(String, f32)>> {
        let importance = self.booster.get_score(
            "",
            &parameters::FeatureImportanceType::Gain,
        )?;
        
        Ok(importance.into_iter().collect())
    }
}
```

### S7-D6: Anomaly Detection

```rust
// crates/investor-analytics/src/anomaly.rs
pub struct AnomalyDetector {
    threshold: f64,
    lookback: usize,
}

impl AnomalyDetector {
    /// Detect regime change using statistical methods
    pub fn detect_regime_change(&self, returns: &[f64]) -> Vec<RegimeChange> {
        let mut changes = Vec::new();
        let window_size = self.lookback;
        
        for i in window_size..returns.len() {
            let window1 = &returns[i - window_size..i];
            let window2 = &returns[i..(i + window_size).min(returns.len())];
            
            // Two-sample t-test
            let t_stat = self.welch_t_test(window1, window2);
            
            if t_stat.abs() > self.threshold {
                changes.push(RegimeChange {
                    index: i,
                    t_statistic: t_stat,
                    p_value: self.p_value(t_stat),
                });
            }
        }
        
        changes
    }
    
    /// Detect outliers using Z-score
    pub fn detect_outliers(&self, values: &[f64]) -> Vec<usize> {
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let std = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            .sqrt() / values.len() as f64;
        
        values.iter()
            .enumerate()
            .filter(|(_, v)| ((**v - mean) / std).abs() > self.threshold)
            .map(|(i, _)| i)
            .collect()
    }
}
```

### S7-D7: Backtest API

```rust
// crates/investor-api/src/handlers/backtest.rs
pub async fn run_backtest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BacktestRequest>,
) -> Json<ApiResponse<BacktestResult>> {
    let backtest = Backtest::new()
        .start_date(req.start_date)
        .end_date(req.end_date)
        .initial_capital(req.initial_capital)
        .strategy(create_strategy(&req.strategy_config))
        .transaction_cost(TransactionCostModel::ibkr())
        .slippage(SlippageModel::fixed(0.001));
    
    let result = backtest.run().await?;
    
    // Store result
    state.store_backtest_result(&result).await?;
    
    Json(ApiResponse::success(result))
}

pub async fn get_backtest_results(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Json<ApiResponse<Vec<BacktestSummary>>> {
    let results = state.get_backtest_results(params).await?;
    
    Json(ApiResponse::success(results))
}
```

### S7-D8: Analytics Dashboard

```yaml
# config/grafana/dashboards/analytics.json (additional panels)
{
  "panels": [
    {
      "title": "Sharpe Ratio",
      "type": "stat",
      "targets": [{"expr": "investor_os_sharpe_ratio"}]
    },
    {
      "title": "Value at Risk (95%)",
      "type": "stat",
      "targets": [{"expr": "investor_os_var_95 * 100"}],
      "fieldConfig": {
        "unit": "percent",
        "thresholds": {
          "steps": [
            {"value": -5, "color": "red"},
            {"value": -2, "color": "yellow"},
            {"value": 0, "color": "green"}
          ]
        }
      }
    },
    {
      "title": "Backtest Equity Curve",
      "type": "timeseries",
      "targets": [{"expr": "investor_os_backtest_nav"}]
    },
    {
      "title": "Factor Attribution",
      "type": "barchart",
      "targets": [{"expr": "investor_os_factor_attribution"}]
    }
  ]
}
```

---

## Golden Path Tests

### S7-GP-01: Backtest Returns Calculation
```rust
#[tokio::test]
async fn test_backtest_returns() {
    let backtest = Backtest::new()
        .start_date(date!(2024-01-01))
        .end_date(date!(2024-12-31))
        .initial_capital(Money::new(dec!(100000)))
        .strategy(Box::new(BuyAndHold::new("AAPL")));
    
    let result = backtest.run().await.unwrap();
    
    assert!(result.total_return > -1.0); // Can't lose more than 100%
    assert!(result.sharpe_ratio.is_finite());
}
```

### S7-GP-02: VaR Calculation
```rust
#[test]
fn test_var_calculation() {
    let returns = vec![0.01, -0.02, 0.015, -0.01, 0.005, -0.03, 0.02];
    let analyzer = RiskAnalyzer::new(returns, 0.02);
    
    let var_95 = analyzer.var(0.95);
    
    assert!(var_95 < 0.0); // VaR should be negative (loss)
    assert!(var_95 >= -0.03); // Worst return was -3%
}
```

### S7-GP-03: Sharpe Ratio
```rust
#[test]
fn test_sharpe_ratio() {
    // 10% annual return, 15% volatility
    let returns: Vec<f64> = (0..252).map(|_| 0.10/252.0).collect();
    let analyzer = RiskAnalyzer::new(returns, 0.02);
    
    let sharpe = analyzer.sharpe();
    
    // Expected: (0.10 - 0.02) / 0.15 = 0.53
    assert!(sharpe > 0.5 && sharpe < 0.6);
}
```

### S7-GP-04: XGBoost Training
```rust
#[tokio::test]
async fn test_xgboost_training() {
    let (train_x, train_y) = load_training_data().await;
    let dtrain = DMatrix::from_dense(&train_x, train_y.len()).unwrap();
    
    let model = CQPredictionModel::train(&dtrain, &train_y, 0.2).unwrap();
    
    // Test prediction
    let (test_x, test_y) = load_test_data().await;
    let dtest = DMatrix::from_dense(&test_x, test_y.len()).unwrap();
    let predictions = model.predict(&dtest).unwrap();
    
    // Calculate accuracy
    let mse = predictions.iter().zip(test_y.iter())
        .map(|(p, y)| (p - y).powi(2))
        .sum::<f32>() / predictions.len() as f32;
    
    assert!(mse < 0.1); // Reasonable error
}
```

### S7-GP-05: Anomaly Detection
```rust
#[test]
fn test_regime_change_detection() {
    // Simulate regime change (high vol to low vol)
    let returns: Vec<f64> = (0..100)
        .map(|i| if i < 50 { 0.02 } else { 0.01 })
        .collect();
    
    let detector = AnomalyDetector::new(2.0, 20);
    let changes = detector.detect_regime_change(&returns);
    
    assert!(!changes.is_empty());
    assert!(changes.iter().any(|c| c.index > 45 && c.index < 55));
}
```

### S7-GP-06: Performance Attribution
```rust
#[test]
fn test_performance_attribution() {
    let portfolio_weights = HashMap::from([
        ("tech".to_string(), 0.6),
        ("finance".to_string(), 0.4),
    ]);
    
    let benchmark_weights = HashMap::from([
        ("tech".to_string(), 0.5),
        ("finance".to_string(), 0.5),
    ]);
    
    let portfolio_returns = HashMap::from([
        ("tech".to_string(), 0.15),
        ("finance".to_string(), 0.08),
    ]);
    
    let benchmark_returns = HashMap::from([
        ("tech".to_string(), 0.12),
        ("finance".to_string(), 0.10),
    ]);
    
    let attribution = PerformanceAttribution::calculate(
        &portfolio_weights,
        &benchmark_weights,
        &portfolio_returns,
        &benchmark_returns,
    );
    
    assert_ne!(attribution.total_return, 0.0);
}
```

---

## Schedule

| Day | Focus |
|-----|-------|
| Day 1 | Backtesting engine core |
| Day 2 | Transaction costs & slippage models |
| Day 3 | Risk metrics (VaR, Sharpe, drawdown) |
| Day 4 | Performance attribution |
| Day 5 | ML feature engineering |
| Day 6 | XGBoost model training |
| Day 7 | Anomaly detection |
| Day 8 | Analytics dashboard, tests |

---

## Exit Criteria

Sprint 7 is **COMPLETE** when:
- ✅ All 6 Golden Path tests pass
- ✅ Backtest runs with transaction costs
- ✅ Risk metrics calculate correctly
- ✅ XGBoost model achieves >70% accuracy
- ✅ Anomaly detection finds regime changes
- ✅ Analytics dashboard shows new panels
