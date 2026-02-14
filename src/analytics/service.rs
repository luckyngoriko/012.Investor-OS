//! Analytics Service
//!
//! High-level analytics API that coordinates backtesting, risk analysis,
//! and portfolio attribution.

use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::analytics::{
    backtest::{Backtest, BacktestConfig, BacktestResult},
    risk::{RiskAnalyzer, RiskMetrics}, PriceBar, Result,
};

/// Analytics service providing unified access to all analytics functionality
#[derive(Debug, Clone)]
pub struct AnalyticsService;

impl AnalyticsService {
    /// Create a new analytics service
    pub fn new() -> Self {
        Self
    }

    /// Run a backtest with the given configuration
    /// 
    /// For now, this uses a simple buy-and-hold strategy as a placeholder.
    /// In production, this would accept a strategy configuration and run
    /// the actual strategy logic.
    pub async fn run_backtest(
        &self,
        config: BacktestConfig,
        tickers: Vec<String>,
    ) -> Result<BacktestResult> {
        // Create a simple buy-and-hold strategy for demonstration
        let strategy = Box::new(SimpleBuyHoldStrategy::new(tickers));
        
        // Generate mock historical data
        // In production, this would fetch from market data provider
        let historical_data = self.generate_mock_data(&config, strategy.tickers()).await?;
        
        // Run backtest
        let mut backtest = Backtest::new(config, strategy);
        backtest.run(&historical_data).await
    }

    /// Calculate risk metrics from a series of returns
    pub fn calculate_risk_metrics(&self, returns: Vec<Decimal>) -> Result<RiskMetrics> {
        let risk_free_rate = Decimal::from(2) / Decimal::from(100); // 2% risk-free rate
        let analyzer = RiskAnalyzer::new(returns, risk_free_rate);
        analyzer.calculate_all()
    }

    /// Generate mock historical data for testing
    /// 
    /// In production, this would be replaced with real market data
    async fn generate_mock_data(
        &self,
        config: &BacktestConfig,
        tickers: &[String],
    ) -> Result<HashMap<String, Vec<PriceBar>>> {
        use chrono::Duration;
        
        let mut data = HashMap::new();
        let mut rng = rand::thread_rng();
        use rand::Rng;

        for ticker in tickers {
            let mut bars = Vec::new();
            let mut current_date = config.start_date;
            let mut price = 100.0; // Starting price

            while current_date < config.end_date {
                // Generate random price movement
                let change: f64 = rng.gen_range(-0.02..0.02); // ±2% daily change
                price *= 1.0 + change;

                let bar = PriceBar {
                    timestamp: current_date,
                    open: price * 0.99,
                    high: price * 1.01,
                    low: price * 0.98,
                    close: price,
                    volume: rng.gen_range(1000000..10000000),
                };

                bars.push(bar);
                current_date += Duration::days(1);
            }

            data.insert(ticker.clone(), bars);
        }

        Ok(data)
    }
}

impl Default for AnalyticsService {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple buy-and-hold strategy for demonstration
struct SimpleBuyHoldStrategy {
    tickers: Vec<String>,
    has_entered: bool,
}

impl SimpleBuyHoldStrategy {
    fn new(tickers: Vec<String>) -> Self {
        Self {
            tickers,
            has_entered: false,
        }
    }

    fn tickers(&self) -> &[String] {
        &self.tickers
    }
}

#[async_trait::async_trait]
impl crate::analytics::Strategy for SimpleBuyHoldStrategy {
    fn name(&self) -> &str {
        "Simple Buy & Hold"
    }

    async fn generate_signals(
        &self,
        _data: &crate::analytics::MarketData,
    ) -> Vec<crate::analytics::Signal> {
        // Buy once at the beginning, hold forever
        if self.has_entered {
            return Vec::new();
        }

        self.tickers
            .iter()
            .map(|ticker| crate::analytics::Signal {
                ticker: ticker.clone(),
                direction: crate::analytics::SignalDirection::Long,
                strength: 1.0,
                confidence: 1.0,
            })
            .collect()
    }

    fn position_size(&self, _signal: &crate::analytics::Signal, portfolio_value: Decimal) -> Decimal {
        // Equal weight across all tickers
        if self.tickers.is_empty() {
            return Decimal::ZERO;
        }
        
        let weight = Decimal::ONE / Decimal::from(self.tickers.len() as i32);
        portfolio_value * weight
    }
}

use rand;
