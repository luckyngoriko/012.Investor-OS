//! NATS message envelope and typed payloads (Sprint N1).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Standard envelope for all NATS messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsEnvelope<T: Serialize> {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub version: String,
    pub data: T,
}

impl<T: Serialize> NatsEnvelope<T> {
    pub fn new(symbol: &str, source: &str, data: T) -> Self {
        Self {
            symbol: symbol.to_string(),
            timestamp: Utc::now(),
            source: source.to_string(),
            version: "v1".to_string(),
            data,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

/// OHLCV price candle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceCandle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub trades: i32,
}

/// Computed technical features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSet {
    pub rsi_14: f64,
    pub macd_signal: f64,
    pub macd_histogram: f64,
    pub atr_14: f64,
    pub bb_position: f64,
    pub obv_trend: f64,
    pub volume_change_pct: f64,
    pub price_change_pct_5: f64,
    pub price_change_pct_20: f64,
}

/// Prediction result from any model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPredictionMsg {
    pub model_name: String,
    pub model_version: String,
    pub prediction_type: String,
    pub direction: Option<String>,
    pub predicted_return: Option<f64>,
    pub volatility: Option<f64>,
    pub var_95: Option<f64>,
    pub confidence: f64,
    pub latency_ms: u64,
    pub forecasts: Option<Vec<f64>>,
}

/// Consensus result combining multiple models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMsg {
    pub direction: String,
    pub predicted_return: f64,
    pub confidence: f64,
    pub agreement: f64,
    pub n_models: usize,
    pub model_contributions: Vec<String>,
}

/// Trade proposal generated from consensus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeProposalMsg {
    pub direction: String,
    pub confidence: f64,
    pub suggested_size_pct: f64,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub reason: String,
}

/// User-facing signal derived from consensus + user strategy preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSignalMsg {
    pub strategy_id: String,
    pub strategy_name: String,
    pub direction: String,
    pub confidence: f64,
    pub predicted_return: f64,
    pub models_used: Vec<String>,
    pub suggested_action: String,
}

/// User-facing trade proposal awaiting confirmation (semi-auto mode).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProposalMsg {
    pub proposal_id: String,
    pub strategy_id: String,
    pub direction: String,
    pub confidence: f64,
    pub suggested_size_pct: f64,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub reason: String,
    pub expires_in_secs: u64,
}

/// Trade execution command (full-auto mode, after risk checks pass).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecuteMsg {
    pub strategy_id: String,
    pub direction: String,
    pub size_pct: f64,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub confidence: f64,
    pub reason: String,
}

/// Trade fill result after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeFillMsg {
    pub trade_id: String,
    pub strategy_id: String,
    pub direction: String,
    pub size_pct: f64,
    pub entry_price: f64,
    pub fill_status: String,
    pub reason: String,
}

/// Portfolio update after a trade fill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioUpdateMsg {
    pub trade_id: String,
    pub strategy_id: String,
    pub direction: String,
    pub size_pct: f64,
    pub entry_price: f64,
    pub portfolio_value: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serialization_roundtrip() {
        let candle = PriceCandle {
            open: 66000.0,
            high: 66500.0,
            low: 65800.0,
            close: 66200.0,
            volume: 1234.5,
            trades: 42,
        };
        let env = NatsEnvelope::new("BTCUSDT", "binance", candle);
        let bytes = env.to_bytes().unwrap();
        let decoded: NatsEnvelope<PriceCandle> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.symbol, "BTCUSDT");
        assert_eq!(decoded.source, "binance");
        assert_eq!(decoded.version, "v1");
        assert!((decoded.data.close - 66200.0).abs() < 0.01);
    }

    #[test]
    fn prediction_msg_serialization() {
        let msg = ModelPredictionMsg {
            model_name: "catboost".to_string(),
            model_version: "v1".to_string(),
            prediction_type: "return".to_string(),
            direction: Some("long".to_string()),
            predicted_return: Some(0.02),
            volatility: None,
            var_95: None,
            confidence: 0.85,
            latency_ms: 45,
            forecasts: None,
        };
        let env = NatsEnvelope::new("BTCUSDT", "catboost", msg);
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("catboost"));
        assert!(json.contains("0.85"));
    }
}
