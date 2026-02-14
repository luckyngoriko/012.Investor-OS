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

use crate::api::handlers::ApiResponse;
use crate::api::AppState;
use crate::analytics::backtest::{BacktestConfig, SlippageModel};
use crate::analytics::risk::RiskAnalyzer;
use crate::analytics::ml::{CQPredictor, FeaturePipeline};
use crate::analytics::attribution::AttributionAnalyzer;
use crate::signals::{TickerSignals, QualityScore};
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
        commission_rate: req.commission_rate.unwrap_or_else(|| 
            Decimal::from(1) / Decimal::from(1000) // 0.1% default
        ),
        slippage_model: SlippageModel::Fixed(
            Decimal::from(1) / Decimal::from(1000) // 0.1% default
        ),
        rebalance_frequency: chrono::Duration::days(1),
        max_positions: 20,
        allow_short: false,
    };

    // Run backtest using analytics service
    match state.analytics_service.run_backtest(config, req.tickers).await {
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
/// Calculates risk metrics using historical returns.
/// For now uses simulated returns based on lookback period.
pub async fn get_risk_metrics(
    State(_state): State<Arc<AppState>>,
    Query(req): Query<RiskMetricsRequest>,
) -> Result<Json<ApiResponse<RiskMetricsResponse>>, StatusCode> {
    // Generate simulated daily returns for demonstration
    // In production, this would fetch actual portfolio returns from database
    let lookback_days = req.lookback_days.unwrap_or(252); // Default 1 year
    let returns = generate_simulated_returns(lookback_days);
    
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

/// Generate simulated daily returns for risk calculation
/// 
/// In production, this would be replaced with actual portfolio returns
/// fetched from the database based on portfolio_id.
fn generate_simulated_returns(days: i64) -> Vec<Decimal> {
    use rand::Rng;
    use rust_decimal::MathematicalOps;
    
    let mut rng = rand::thread_rng();
    let mut returns = Vec::new();
    
    // Simulate daily returns with mean ~0.0004 (10% annual) and std ~0.02
    for _ in 0..days {
        // Generate random return using Box-Muller transform approximation
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        
        // Convert to daily return (mean 0.04%, std 2%)
        let daily_return = 0.0004 + z * 0.02;
        returns.push(Decimal::try_from(daily_return).unwrap_or(Decimal::ZERO));
    }
    
    returns
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

/// GET /api/analytics/attribution - Get performance attribution
/// 
/// Uses Brinson-Fachler model to decompose returns into:
/// - Allocation effect: From sector over/under-weighting
/// - Selection effect: From stock picking within sectors
/// 
/// For now uses simulated portfolio/benchmark data.
pub async fn get_attribution(
    State(_state): State<Arc<AppState>>,
    Query(req): Query<AttributionRequest>,
) -> Result<Json<ApiResponse<AttributionResponse>>, StatusCode> {
    // Generate simulated portfolio and benchmark data
    // In production, fetch real data from portfolio database
    let (portfolio_weights, benchmark_weights, portfolio_returns, benchmark_returns) = 
        generate_simulated_attribution_data(&req.portfolio_id);
    
    // Calculate attribution using Brinson-Fachler model
    let result = AttributionAnalyzer::brinson_attribution(
        &portfolio_weights,
        &benchmark_weights,
        &portfolio_returns,
        &benchmark_returns,
    );
    
    // Convert to API response format
    let sector_attributions: Vec<SectorAttribution> = result.sector_attributions
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

/// Generate simulated attribution data
/// 
/// In production, this would fetch real portfolio and benchmark data
fn generate_simulated_attribution_data(
    _portfolio_id: &str
) -> (
    HashMap<String, Decimal>, // portfolio weights
    HashMap<String, Decimal>, // benchmark weights
    HashMap<String, Decimal>, // portfolio returns
    HashMap<String, Decimal>, // benchmark returns
) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let sectors = vec!["Technology", "Finance", "Healthcare", "Energy", "Consumer"];
    
    let mut portfolio_weights = HashMap::new();
    let mut benchmark_weights = HashMap::new();
    let mut portfolio_returns = HashMap::new();
    let mut benchmark_returns = HashMap::new();
    
    // Equal benchmark weights
    let bench_weight = Decimal::from(1) / Decimal::from(sectors.len() as i32);
    
    for sector in sectors {
        // Portfolio weights vary from benchmark
        let weight_variation: f64 = rng.gen_range(-0.1..0.1);
        let port_weight = (bench_weight + Decimal::try_from(weight_variation).unwrap_or(Decimal::ZERO))
            .max(Decimal::ZERO);
        
        // Returns vary by sector
        let port_return: f64 = rng.gen_range(-0.05..0.15); // -5% to +15%
        let bench_return: f64 = rng.gen_range(-0.03..0.10); // -3% to +10%
        
        portfolio_weights.insert(sector.to_string(), port_weight);
        benchmark_weights.insert(sector.to_string(), bench_weight);
        portfolio_returns.insert(sector.to_string(), Decimal::try_from(port_return).unwrap_or(Decimal::ZERO));
        benchmark_returns.insert(sector.to_string(), Decimal::try_from(bench_return).unwrap_or(Decimal::ZERO));
    }
    
    (portfolio_weights, benchmark_weights, portfolio_returns, benchmark_returns)
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
/// Uses CQPredictor to calculate Composite Quality score from signals.
/// In production, this would fetch real signals from the database.
pub async fn get_ml_prediction(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<MLPredictionRequest>,
) -> Result<Json<ApiResponse<MLPredictionResponse>>, StatusCode> {
    // Generate simulated signals for the ticker
    // In production, fetch real signals from database
    let signals = generate_simulated_signals(&req.ticker);
    
    // Generate features from signals
    let feature_vector = FeaturePipeline::generate_features(&req.ticker, &signals);
    
    // Use CQPredictor for prediction
    let predictor = CQPredictor::new();
    
    match predictor.predict_with_confidence(&feature_vector.features) {
        Ok((predicted_cq, confidence)) => {
            // Get feature importance
            let feature_importance = predictor.feature_importance(&feature_vector.feature_names);
            
            // Take top 5 most important features
            let top_features: Vec<(String, f64)> = feature_importance
                .into_iter()
                .take(5)
                .collect();
            
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

/// Generate simulated signals for ML prediction
/// 
/// In production, these would come from the signals pipeline.
fn generate_simulated_signals(_ticker: &str) -> TickerSignals {
    use rand::Rng;
    
    let mut rng = rand::thread_rng();
    
    // Generate random scores between 30 and 90 (u8 for QualityScore)
    TickerSignals {
        quality_score: QualityScore(rng.gen_range(30..90)),
        value_score: QualityScore(rng.gen_range(30..90)),
        momentum_score: QualityScore(rng.gen_range(30..90)),
        insider_score: QualityScore(rng.gen_range(30..90)),
        sentiment_score: QualityScore(rng.gen_range(30..90)),
        regime_fit: QualityScore(rng.gen_range(30..90)),
        composite_quality: QualityScore(rng.gen_range(30..90)),
        
        insider_flow_ratio: rng.gen_range(0.5..1.5),
        insider_cluster_signal: rng.gen_bool(0.5),
        
        news_sentiment: rng.gen_range(-0.5..0.5),
        social_sentiment: rng.gen_range(-0.5..0.5),
        
        vix_level: rng.gen_range(15.0..30.0),
        market_breadth: rng.gen_range(0.4..0.8),
        
        breakout_score: rng.gen_range(0.3..0.8),
        atr_trend: rng.gen_range(-0.1..0.1),
        rsi_14: rng.gen_range(30.0..70.0),
        macd_signal: rng.gen_range(-0.5..0.5),
    }
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
