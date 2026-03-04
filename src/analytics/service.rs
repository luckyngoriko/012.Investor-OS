//! Analytics Service
//!
//! High-level analytics API that coordinates backtesting, risk analysis,
//! and portfolio attribution.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use crate::analytics::{
    backtest::{Backtest, BacktestConfig, BacktestResult},
    risk::{RiskAnalyzer, RiskMetrics},
    AnalyticsError, PriceBar, Result,
};

/// Historical data provider abstraction used by analytics backtesting.
pub trait HistoricalDataProvider: Send + Sync + std::fmt::Debug {
    /// Human-readable source name for logs/telemetry.
    fn source_name(&self) -> &'static str;

    /// Fetch historical OHLCV bars for a ticker in the requested range.
    fn fetch_historical_data(
        &self,
        ticker: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PriceBar>>;
}

/// CSV-backed historical data provider.
///
/// Expected file layout:
/// - `<root>/<TICKER>.csv`
/// - columns: `timestamp,open,high,low,close,volume`
#[derive(Debug, Clone)]
pub struct CsvHistoricalDataProvider {
    root: PathBuf,
}

impl CsvHistoricalDataProvider {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn ticker_path(&self, ticker: &str) -> PathBuf {
        self.root.join(format!("{}.csv", sanitize_ticker(ticker)))
    }
}

impl Default for CsvHistoricalDataProvider {
    fn default() -> Self {
        let root = std::env::var("ANALYTICS_MARKET_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/market"));

        Self::new(root)
    }
}

impl HistoricalDataProvider for CsvHistoricalDataProvider {
    fn source_name(&self) -> &'static str {
        "csv_market_data"
    }

    fn fetch_historical_data(
        &self,
        ticker: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PriceBar>> {
        let path = self.ticker_path(ticker);
        let content = fs::read_to_string(&path).map_err(|e| {
            AnalyticsError::InsufficientData(format!(
                "Historical data file is missing for ticker '{}' at '{}': {}",
                ticker,
                path.display(),
                e
            ))
        })?;

        let mut bars = Vec::new();

        for (idx, raw_line) in content.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if idx == 0 && line.to_ascii_lowercase().starts_with("timestamp,") {
                continue;
            }

            let columns: Vec<&str> = line.split(',').map(|v| v.trim()).collect();
            if columns.len() < 6 {
                return Err(AnalyticsError::InvalidParameters(format!(
                    "Invalid historical row format for ticker '{}' on line {} in '{}': expected 6 columns",
                    ticker,
                    idx + 1,
                    path.display()
                )));
            }

            let timestamp = parse_timestamp(columns[0]).map_err(|e| {
                AnalyticsError::InvalidParameters(format!(
                    "Invalid timestamp '{}' for ticker '{}' on line {} in '{}': {}",
                    columns[0],
                    ticker,
                    idx + 1,
                    path.display(),
                    e
                ))
            })?;

            if timestamp < start || timestamp > end {
                continue;
            }

            let open = columns[1].parse::<f64>().map_err(|e| {
                AnalyticsError::InvalidParameters(format!(
                    "Invalid open '{}' for ticker '{}' on line {} in '{}': {}",
                    columns[1],
                    ticker,
                    idx + 1,
                    path.display(),
                    e
                ))
            })?;
            let high = columns[2].parse::<f64>().map_err(|e| {
                AnalyticsError::InvalidParameters(format!(
                    "Invalid high '{}' for ticker '{}' on line {} in '{}': {}",
                    columns[2],
                    ticker,
                    idx + 1,
                    path.display(),
                    e
                ))
            })?;
            let low = columns[3].parse::<f64>().map_err(|e| {
                AnalyticsError::InvalidParameters(format!(
                    "Invalid low '{}' for ticker '{}' on line {} in '{}': {}",
                    columns[3],
                    ticker,
                    idx + 1,
                    path.display(),
                    e
                ))
            })?;
            let close = columns[4].parse::<f64>().map_err(|e| {
                AnalyticsError::InvalidParameters(format!(
                    "Invalid close '{}' for ticker '{}' on line {} in '{}': {}",
                    columns[4],
                    ticker,
                    idx + 1,
                    path.display(),
                    e
                ))
            })?;
            let volume = columns[5].parse::<i64>().map_err(|e| {
                AnalyticsError::InvalidParameters(format!(
                    "Invalid volume '{}' for ticker '{}' on line {} in '{}': {}",
                    columns[5],
                    ticker,
                    idx + 1,
                    path.display(),
                    e
                ))
            })?;

            bars.push(PriceBar {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
            });
        }

        if bars.is_empty() {
            return Err(AnalyticsError::InsufficientData(format!(
                "No historical data rows in requested range for ticker '{}' from source '{}'",
                ticker,
                self.source_name()
            )));
        }

        bars.sort_by_key(|bar| bar.timestamp);
        Ok(bars)
    }
}

/// Analytics service providing unified access to all analytics functionality.
#[derive(Debug, Clone)]
pub struct AnalyticsService {
    historical_data_provider: Arc<dyn HistoricalDataProvider>,
}

impl AnalyticsService {
    /// Create a new analytics service using the default CSV provider.
    pub fn new() -> Self {
        Self {
            historical_data_provider: Arc::new(CsvHistoricalDataProvider::default()),
        }
    }

    /// Create analytics service with a custom historical data provider.
    pub fn with_provider(provider: Arc<dyn HistoricalDataProvider>) -> Self {
        Self {
            historical_data_provider: provider,
        }
    }

    /// Run a backtest with the given configuration.
    pub async fn run_backtest(
        &self,
        config: BacktestConfig,
        tickers: Vec<String>,
    ) -> Result<BacktestResult> {
        if tickers.is_empty() {
            return Err(AnalyticsError::InvalidParameters(
                "Backtest requires at least one ticker".to_string(),
            ));
        }

        let strategy = Box::new(SimpleBuyHoldStrategy::new(tickers));

        let historical_data = self
            .load_historical_data(&config, strategy.tickers())
            .await?;

        info!(
            source = self.historical_data_provider.source_name(),
            tickers = strategy.tickers().len(),
            "Running backtest with historical market data"
        );

        let mut backtest = Backtest::new(config, strategy);
        backtest.run(&historical_data).await
    }

    /// Calculate risk metrics from a series of returns
    pub fn calculate_risk_metrics(&self, returns: Vec<Decimal>) -> Result<RiskMetrics> {
        let risk_free_rate = Decimal::from(2) / Decimal::from(100); // 2% risk-free rate
        let analyzer = RiskAnalyzer::new(returns, risk_free_rate);
        analyzer.calculate_all()
    }

    async fn load_historical_data(
        &self,
        config: &BacktestConfig,
        tickers: &[String],
    ) -> Result<HashMap<String, Vec<PriceBar>>> {
        let mut data = HashMap::new();

        for ticker in tickers {
            let bars = self.historical_data_provider.fetch_historical_data(
                ticker,
                config.start_date,
                config.end_date,
            )?;

            validate_price_series(ticker, &bars)?;
            data.insert(ticker.clone(), bars);
        }

        if data.is_empty() {
            return Err(AnalyticsError::InsufficientData(
                "No historical data loaded for requested tickers".to_string(),
            ));
        }

        Ok(data)
    }
}

impl Default for AnalyticsService {
    fn default() -> Self {
        Self::new()
    }
}

fn sanitize_ticker(ticker: &str) -> String {
    ticker
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn parse_timestamp(value: &str) -> std::result::Result<DateTime<Utc>, String> {
    if let Ok(ts) = DateTime::parse_from_rfc3339(value) {
        return Ok(ts.with_timezone(&Utc));
    }

    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let dt = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| "invalid date time components".to_string())?;
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }

    Err("supported formats: RFC3339 or YYYY-MM-DD".to_string())
}

fn validate_price_series(ticker: &str, bars: &[PriceBar]) -> Result<()> {
    if bars.len() < 2 {
        return Err(AnalyticsError::InsufficientData(format!(
            "Ticker '{}' has insufficient historical rows: {}",
            ticker,
            bars.len()
        )));
    }

    for (idx, bar) in bars.iter().enumerate() {
        if bar.open <= 0.0 || bar.high <= 0.0 || bar.low <= 0.0 || bar.close <= 0.0 {
            return Err(AnalyticsError::InvalidParameters(format!(
                "Ticker '{}' has non-positive OHLC values at index {}",
                ticker, idx
            )));
        }

        let max_oc = bar.open.max(bar.close);
        let min_oc = bar.open.min(bar.close);

        if bar.high < max_oc || bar.low > min_oc || bar.low > bar.high {
            return Err(AnalyticsError::InvalidParameters(format!(
                "Ticker '{}' has invalid OHLC geometry at index {}",
                ticker, idx
            )));
        }
    }

    for (idx, pair) in bars.windows(2).enumerate() {
        if pair[0].timestamp >= pair[1].timestamp {
            return Err(AnalyticsError::InvalidParameters(format!(
                "Ticker '{}' has non-increasing timestamps at index {}",
                ticker, idx
            )));
        }
    }

    Ok(())
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

    fn position_size(
        &self,
        _signal: &crate::analytics::Signal,
        portfolio_value: Decimal,
    ) -> Decimal {
        // Equal weight across all tickers
        if self.tickers.is_empty() {
            return Decimal::ZERO;
        }

        let weight = Decimal::ONE / Decimal::from(self.tickers.len() as i32);
        portfolio_value * weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn temp_data_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "investor_os_analytics_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&dir).expect("failed to create temp data dir");
        dir
    }

    fn write_csv(root: &PathBuf, ticker: &str, content: &str) {
        let path = root.join(format!("{}.csv", ticker));
        fs::write(path, content).expect("failed to write csv fixture");
    }

    fn parse_utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("valid timestamp")
            .with_timezone(&Utc)
    }

    fn make_config(start: DateTime<Utc>, end: DateTime<Utc>) -> BacktestConfig {
        BacktestConfig {
            start_date: start,
            end_date: end,
            initial_capital: Decimal::from(100000),
            commission_rate: Decimal::from(1) / Decimal::from(1000),
            slippage_model: crate::analytics::backtest::SlippageModel::Fixed(
                Decimal::from(1) / Decimal::from(1000),
            ),
            rebalance_frequency: Duration::days(1),
            max_positions: 10,
            allow_short: false,
        }
    }

    #[test]
    fn test_csv_provider_loads_historical_rows() {
        let root = temp_data_dir();
        write_csv(
            &root,
            "AAPL",
            "timestamp,open,high,low,close,volume\n2024-01-01,100,102,99,101,1000000\n2024-01-02,101,103,100,102,1100000\n2024-01-03,102,104,101,103,1200000\n",
        );

        let provider = CsvHistoricalDataProvider::new(&root);
        let start = parse_utc("2024-01-01T00:00:00Z");
        let end = parse_utc("2024-01-03T23:59:59Z");

        let bars = provider
            .fetch_historical_data("AAPL", start, end)
            .expect("csv provider should load rows");

        assert_eq!(bars.len(), 3);
        assert!(bars[0].timestamp < bars[1].timestamp);
    }

    #[tokio::test]
    async fn test_run_backtest_requires_real_data_files() {
        let root = temp_data_dir();
        let service =
            AnalyticsService::with_provider(Arc::new(CsvHistoricalDataProvider::new(&root)));

        let start = parse_utc("2024-01-01T00:00:00Z");
        let end = parse_utc("2024-01-03T00:00:00Z");
        let config = make_config(start, end);

        let result = service.run_backtest(config, vec!["AAPL".to_string()]).await;
        assert!(matches!(result, Err(AnalyticsError::InsufficientData(_))));
    }

    #[tokio::test]
    async fn test_run_backtest_with_csv_data() {
        let root = temp_data_dir();
        write_csv(
            &root,
            "AAPL",
            "timestamp,open,high,low,close,volume\n2024-01-01,100,102,99,101,1000000\n2024-01-02,101,103,100,102,1100000\n2024-01-03,102,104,101,103,1200000\n2024-01-04,103,105,102,104,1300000\n",
        );

        let service =
            AnalyticsService::with_provider(Arc::new(CsvHistoricalDataProvider::new(&root)));
        let start = parse_utc("2024-01-01T00:00:00Z");
        let end = parse_utc("2024-01-05T00:00:00Z");
        let config = make_config(start, end);

        let result = service
            .run_backtest(config, vec!["AAPL".to_string()])
            .await
            .expect("backtest should run with CSV historical data");

        assert!(result.total_return > Decimal::from(-1));
        assert!(result.total_trades > 0);
    }

    #[tokio::test]
    async fn test_run_backtest_rejects_invalid_ohlc_geometry() {
        let root = temp_data_dir();
        write_csv(
            &root,
            "AAPL",
            "timestamp,open,high,low,close,volume\n2024-01-01,100,99,98,101,1000000\n2024-01-02,101,103,100,102,1100000\n",
        );

        let service =
            AnalyticsService::with_provider(Arc::new(CsvHistoricalDataProvider::new(&root)));
        let start = parse_utc("2024-01-01T00:00:00Z");
        let end = parse_utc("2024-01-03T00:00:00Z");
        let config = make_config(start, end);

        let result = service.run_backtest(config, vec!["AAPL".to_string()]).await;
        assert!(matches!(result, Err(AnalyticsError::InvalidParameters(_))));
    }
}
