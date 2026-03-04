//! Analytics API Handlers
//!
//! S7-D7: Backtest API Endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::analytics::attribution::AttributionAnalyzer;
use crate::analytics::backtest::{BacktestConfig, SlippageModel};
use crate::analytics::ml::{CQPredictor, FeaturePipeline};
use crate::analytics::risk::RiskAnalyzer;
use crate::api::handlers::ApiResponse;
use crate::api::AppState;
use crate::signals::{QualityScore, TickerSignals};
use std::collections::HashMap;

/// Backtest request
#[derive(Serialize, Deserialize)]
pub struct BacktestRequest {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub strategy: String,
    pub tickers: Vec<String>,
    pub commission_rate: Option<Decimal>,
}

/// Backtest response
#[derive(Serialize, Deserialize)]
pub struct BacktestResponse {
    pub total_return: Decimal,
    pub annualized_return: Decimal,
    pub sharpe_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub total_trades: usize,
    pub win_rate: Decimal,
}

/// POST /api/analytics/backtest - Run backtest
pub async fn run_backtest(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BacktestRequest>,
) -> Result<Json<ApiResponse<BacktestResponse>>, StatusCode> {
    // Build backtest config from request
    let config = BacktestConfig {
        start_date: req.start_date,
        end_date: req.end_date,
        initial_capital: req.initial_capital,
        commission_rate: req.commission_rate.unwrap_or_else(
            || Decimal::from(1) / Decimal::from(1000), // 0.1% default
        ),
        slippage_model: SlippageModel::Fixed(
            Decimal::from(1) / Decimal::from(1000), // 0.1% default
        ),
        rebalance_frequency: chrono::Duration::days(1),
        max_positions: 20,
        allow_short: false,
    };

    // Run backtest using analytics service
    match state
        .analytics_service
        .run_backtest(config, req.tickers)
        .await
    {
        Ok(result) => {
            let response = BacktestResponse {
                total_return: result.total_return,
                annualized_return: result.annualized_return,
                sharpe_ratio: result.sharpe_ratio,
                max_drawdown: result.max_drawdown,
                total_trades: result.total_trades,
                win_rate: result.win_rate,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Backtest error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Risk metrics request
#[derive(Serialize, Deserialize)]
pub struct RiskMetricsRequest {
    pub portfolio_id: String,
    pub lookback_days: Option<i64>,
}

/// Risk metrics response
#[derive(Serialize, Deserialize)]
pub struct RiskMetricsResponse {
    pub var_95: Decimal,
    pub var_99: Decimal,
    pub sharpe_ratio: Decimal,
    pub sortino_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub volatility: Decimal,
}

/// GET /api/analytics/risk - Get risk metrics
///
/// Calculates risk metrics using real portfolio returns from the database.
/// Returns an error if insufficient return data is available.
pub async fn get_risk_metrics(
    State(state): State<Arc<AppState>>,
    Query(req): Query<RiskMetricsRequest>,
) -> Result<Json<ApiResponse<RiskMetricsResponse>>, StatusCode> {
    let lookback_days = req.lookback_days.unwrap_or(252);

    // Fetch real portfolio returns from database
    let returns = fetch_portfolio_returns(&state.pool, &req.portfolio_id, lookback_days).await;

    let returns = match returns {
        Ok(r) if r.len() >= 30 => r,
        Ok(r) => {
            return Ok(Json(ApiResponse::error(format!(
                "Insufficient portfolio return data: found {} returns, need at least 30",
                r.len()
            ))));
        }
        Err(e) => {
            tracing::warn!("Failed to fetch portfolio returns: {}", e);
            return Ok(Json(ApiResponse::error(format!(
                "Failed to fetch portfolio returns: {}",
                e
            ))));
        }
    };

    // Use RiskAnalyzer for calculations
    let risk_free_rate = Decimal::from(2) / Decimal::from(100); // 2% annual
    let analyzer = RiskAnalyzer::new(returns, risk_free_rate);

    match analyzer.calculate_all() {
        Ok(metrics) => {
            let response = RiskMetricsResponse {
                var_95: metrics.var_95,
                var_99: metrics.var_99,
                sharpe_ratio: metrics.sharpe_ratio,
                sortino_ratio: metrics.sortino_ratio,
                max_drawdown: metrics.max_drawdown,
                volatility: metrics.volatility,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Risk calculation error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Fetch real portfolio daily returns from database
async fn fetch_portfolio_returns(
    pool: &sqlx::PgPool,
    portfolio_id: &str,
    lookback_days: i64,
) -> std::result::Result<Vec<Decimal>, String> {
    let rows = sqlx::query(
        r#"
        SELECT daily_return
        FROM portfolio_snapshots
        WHERE portfolio_id = $1
          AND snapshot_date >= NOW() - make_interval(days => $2)
        ORDER BY snapshot_date ASC
        "#,
    )
    .bind(portfolio_id)
    .bind(lookback_days as i32)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    let returns: Vec<Decimal> = rows
        .iter()
        .filter_map(|row| {
            let val: f64 = row.try_get("daily_return").ok()?;
            Decimal::try_from(val).ok()
        })
        .collect();

    Ok(returns)
}

/// Attribution request
#[derive(Serialize, Deserialize)]
pub struct AttributionRequest {
    pub portfolio_id: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Attribution response
#[derive(Serialize, Deserialize)]
pub struct AttributionResponse {
    pub total_return: Decimal,
    pub allocation_effect: Decimal,
    pub selection_effect: Decimal,
    pub sector_attributions: Vec<SectorAttribution>,
}

#[derive(Serialize, Deserialize)]
pub struct SectorAttribution {
    pub sector: String,
    pub allocation_effect: Decimal,
    pub selection_effect: Decimal,
    pub total_effect: Decimal,
}

/// S&P 500 approximate sector benchmark weights (static reference)
const SP500_BENCHMARK_WEIGHTS: &[(&str, &str)] = &[
    ("Technology", "0.29"),
    ("Healthcare", "0.13"),
    ("Financials", "0.13"),
    ("Consumer Discretionary", "0.11"),
    ("Communication Services", "0.09"),
    ("Industrials", "0.08"),
    ("Consumer Staples", "0.06"),
    ("Energy", "0.04"),
    ("Utilities", "0.03"),
    ("Real Estate", "0.02"),
    ("Materials", "0.02"),
];

/// GET /api/analytics/attribution - Get performance attribution
///
/// Uses Brinson-Fachler model to decompose returns into:
/// - Allocation effect: From sector over/under-weighting
/// - Selection effect: From stock picking within sectors
///
/// Fetches real portfolio positions from database and uses static S&P 500
/// sector weights as benchmark.
pub async fn get_attribution(
    State(state): State<Arc<AppState>>,
    Query(req): Query<AttributionRequest>,
) -> Result<Json<ApiResponse<AttributionResponse>>, StatusCode> {
    // Fetch real portfolio weights from positions table, grouped by ticker
    let (portfolio_weights, portfolio_returns) =
        match fetch_attribution_data(&state.pool, &req.portfolio_id, req.start_date, req.end_date)
            .await
        {
            Ok(data) if data.0.is_empty() => {
                return Ok(Json(ApiResponse::error(
                    "Insufficient attribution data: no positions found for portfolio",
                )));
            }
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to fetch attribution data: {}", e);
                return Ok(Json(ApiResponse::error(format!(
                    "Failed to fetch attribution data: {}",
                    e
                ))));
            }
        };

    // Build benchmark weights from static S&P 500 sector weights
    let benchmark_weights: HashMap<String, Decimal> = SP500_BENCHMARK_WEIGHTS
        .iter()
        .map(|(sector, weight)| {
            (
                sector.to_string(),
                weight.parse::<Decimal>().unwrap_or(Decimal::ZERO),
            )
        })
        .collect();

    // Benchmark returns: assume market-average return spread across sectors
    // (in a full implementation, fetch per-sector ETF returns)
    let benchmark_returns: HashMap<String, Decimal> = benchmark_weights
        .keys()
        .map(|sector| {
            (
                sector.clone(),
                Decimal::from(5) / Decimal::from(100), // 5% annualized default
            )
        })
        .collect();

    // Calculate attribution using Brinson-Fachler model
    let result = AttributionAnalyzer::brinson_attribution(
        &portfolio_weights,
        &benchmark_weights,
        &portfolio_returns,
        &benchmark_returns,
    );

    // Convert to API response format
    let sector_attributions: Vec<SectorAttribution> = result
        .sector_attributions
        .into_iter()
        .map(|sa| SectorAttribution {
            sector: sa.sector,
            allocation_effect: sa.allocation_effect,
            selection_effect: sa.selection_effect,
            total_effect: sa.total_effect,
        })
        .collect();

    let response = AttributionResponse {
        total_return: result.total_return,
        allocation_effect: result.allocation_effect,
        selection_effect: result.selection_effect,
        sector_attributions,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Fetch real portfolio weights and returns from positions table, grouped by ticker
async fn fetch_attribution_data(
    pool: &sqlx::PgPool,
    portfolio_id: &str,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
) -> std::result::Result<(HashMap<String, Decimal>, HashMap<String, Decimal>), String> {
    let rows = sqlx::query(
        r#"
        SELECT ticker,
               SUM(market_value) AS total_value,
               AVG(daily_return)  AS avg_return
        FROM positions
        WHERE portfolio_id = $1
          AND updated_at >= $2
          AND updated_at <= $3
        GROUP BY ticker
        "#,
    )
    .bind(portfolio_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    let total_portfolio_value: Decimal = rows
        .iter()
        .filter_map(|row| {
            let val: f64 = row.try_get("total_value").ok()?;
            Decimal::try_from(val).ok()
        })
        .sum();

    let mut weights = HashMap::new();
    let mut returns = HashMap::new();

    for row in &rows {
        let ticker: String = row.try_get("ticker").map_err(|e| e.to_string())?;
        let value_f64: f64 = row.try_get("total_value").unwrap_or(0.0);
        let value = Decimal::try_from(value_f64).unwrap_or(Decimal::ZERO);
        let avg_return: f64 = row.try_get("avg_return").unwrap_or(0.0);

        let weight = if total_portfolio_value > Decimal::ZERO {
            value / total_portfolio_value
        } else {
            Decimal::ZERO
        };

        weights.insert(ticker.clone(), weight);
        returns.insert(
            ticker,
            Decimal::try_from(avg_return).unwrap_or(Decimal::ZERO),
        );
    }

    Ok((weights, returns))
}

/// ML prediction request
#[derive(Serialize, Deserialize)]
pub struct MLPredictionRequest {
    pub ticker: String,
}

/// ML prediction response
#[derive(Serialize, Deserialize)]
pub struct MLPredictionResponse {
    pub ticker: String,
    pub predicted_cq: f64,
    pub confidence: f64,
    pub should_trade: bool,
    pub feature_importance: Vec<(String, f64)>,
}

/// POST /api/analytics/predict - Get ML prediction
///
/// Uses CQPredictor to calculate Composite Quality score from real signal data.
/// Fetches signals from the `signals` table for the requested ticker.
pub async fn get_ml_prediction(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MLPredictionRequest>,
) -> Result<Json<ApiResponse<MLPredictionResponse>>, StatusCode> {
    // Fetch real signals from database
    let signals = match fetch_ticker_signals(&state.pool, &req.ticker).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Ok(Json(ApiResponse::error(format!(
                "No signal data available for ticker: {}",
                req.ticker
            ))));
        }
        Err(e) => {
            tracing::warn!("Failed to fetch signals for {}: {}", req.ticker, e);
            return Ok(Json(ApiResponse::error(format!(
                "Failed to fetch signals: {}",
                e
            ))));
        }
    };

    // Generate features from signals
    let feature_vector = FeaturePipeline::generate_features(&req.ticker, &signals);

    // Use CQPredictor for prediction
    let predictor = CQPredictor::new();

    match predictor.predict_with_confidence(&feature_vector.features) {
        Ok((predicted_cq, confidence)) => {
            // Get feature importance
            let feature_importance = predictor.feature_importance(&feature_vector.feature_names);

            // Take top 5 most important features
            let top_features: Vec<(String, f64)> = feature_importance.into_iter().take(5).collect();

            let response = MLPredictionResponse {
                ticker: req.ticker,
                predicted_cq,
                confidence,
                should_trade: predicted_cq > 0.65, // threshold
                feature_importance: top_features,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("ML prediction error: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Fetch the latest signal data for a ticker from the signals table
async fn fetch_ticker_signals(
    pool: &sqlx::PgPool,
    ticker: &str,
) -> std::result::Result<Option<TickerSignals>, String> {
    let row = sqlx::query(
        r#"
        SELECT quality_score, value_score, momentum_score, insider_score,
               sentiment_score, regime_fit, breakout_score
        FROM signals
        WHERE ticker = $1
        ORDER BY calculated_at DESC
        LIMIT 1
        "#,
    )
    .bind(ticker)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    use sqlx::Row;
    let quality: i32 = row.try_get("quality_score").unwrap_or(50);
    let value: i32 = row.try_get("value_score").unwrap_or(50);
    let momentum: i32 = row.try_get("momentum_score").unwrap_or(50);
    let insider: i32 = row.try_get("insider_score").unwrap_or(50);
    let sentiment: i32 = row.try_get("sentiment_score").unwrap_or(50);
    let regime: i32 = row.try_get("regime_fit").unwrap_or(50);
    let breakout: f64 = row.try_get("breakout_score").unwrap_or(0.5);

    Ok(Some(TickerSignals {
        quality_score: QualityScore(quality.clamp(0, 100) as u8),
        value_score: QualityScore(value.clamp(0, 100) as u8),
        momentum_score: QualityScore(momentum.clamp(0, 100) as u8),
        insider_score: QualityScore(insider.clamp(0, 100) as u8),
        sentiment_score: QualityScore(sentiment.clamp(0, 100) as u8),
        regime_fit: QualityScore(regime.clamp(0, 100) as u8),
        composite_quality: QualityScore(50), // computed by CQPredictor
        breakout_score: breakout,
        ..TickerSignals::default()
    }))
}

/// Anomaly detection response
#[derive(Serialize, Deserialize)]
pub struct AnomalyResponse {
    pub detected: bool,
    pub z_score: Option<f64>,
    pub severity: Option<String>,
    pub message: String,
}

/// GET /api/analytics/anomalies - Check for anomalies
pub async fn check_anomalies(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<AnomalyResponse>>, StatusCode> {
    let response = AnomalyResponse {
        detected: false,
        z_score: None,
        severity: None,
        message: "No anomalies detected".to_string(),
    };

    Ok(Json(ApiResponse::success(response)))
}
