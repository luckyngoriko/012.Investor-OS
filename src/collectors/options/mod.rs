//! Options Flow Collector - Sprint 14
//!
//! Tracks unusual options activity, block trades, and flow sentiment

use serde::{Deserialize, Serialize};

/// Options flow collector
#[derive(Debug, Clone)]
pub struct OptionsFlowCollector {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl OptionsFlowCollector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: std::env::var("OPTIONS_FLOW_API_KEY").ok(),
        }
    }

    /// Get options flow for a ticker
    pub async fn get_flow(&self, ticker: &str) -> Result<OptionsFlow, OptionsError> {
        // Would fetch from Unusual Whales, Cheddar Flow, or similar API
        // Placeholder with realistic synthetic data
        let flow = OptionsFlow {
            ticker: ticker.to_string(),
            timestamp: chrono::Utc::now(),
            call_volume: 15000,
            put_volume: 8000,
            call_put_ratio: 1.875,
            unusual_call_volume: true,
            unusual_put_volume: false,
            net_premium: 2_500_000.0,
            bullish_flow: true,
            implied_move_pct: 5.5,
            block_trades: vec![
                BlockTrade {
                    option_type: OptionType::Call,
                    strike: 150.0,
                    expiration: "2024-12-20".to_string(),
                    volume: 5000,
                    premium: 1_250_000.0,
                    sentiment: TradeSentiment::Bullish,
                },
                BlockTrade {
                    option_type: OptionType::Put,
                    strike: 140.0,
                    expiration: "2024-12-20".to_string(),
                    volume: 2000,
                    premium: 400_000.0,
                    sentiment: TradeSentiment::Bearish,
                },
            ],
            sweep_count: 15,
            institutional_activity: true,
        };

        Ok(flow)
    }

    /// Get unusual options activity across market
    pub async fn get_unusual_activity(&self) -> Result<Vec<OptionsFlow>, OptionsError> {
        // Would fetch top unusual options activity
        let tickers = vec!["AAPL", "TSLA", "NVDA", "AMD", "META"];
        let mut flows = vec![];

        for ticker in tickers {
            if let Ok(flow) = self.get_flow(ticker).await {
                if flow.has_unusual_activity() {
                    flows.push(flow);
                }
            }
        }

        // Sort by net premium
        flows.sort_by(|a, b| b.net_premium.partial_cmp(&a.net_premium).unwrap());

        Ok(flows.into_iter().take(10).collect())
    }

    /// Get options sentiment for a ticker
    pub async fn get_sentiment(&self, ticker: &str) -> Result<FlowSentiment, OptionsError> {
        let flow = self.get_flow(ticker).await?;

        let sentiment_score = if flow.call_put_ratio > 2.0 {
            0.8
        } else if flow.call_put_ratio > 1.5 {
            0.6
        } else if flow.call_put_ratio > 1.0 {
            0.3
        } else if flow.call_put_ratio > 0.75 {
            -0.3
        } else if flow.call_put_ratio > 0.5 {
            -0.6
        } else {
            -0.8
        };

        Ok(FlowSentiment {
            ticker: ticker.to_string(),
            sentiment_score,
            call_put_ratio: flow.call_put_ratio,
            unusual_activity: flow.has_unusual_activity(),
            institutional_bias: flow.institutional_activity,
            confidence: flow.calculate_confidence(),
        })
    }

    /// Get options flow for multiple tickers
    pub async fn get_flows(&self, tickers: &[String]) -> Vec<Result<OptionsFlow, OptionsError>> {
        let mut results = vec![];
        for ticker in tickers {
            results.push(self.get_flow(ticker).await);
        }
        results
    }
}

impl Default for OptionsFlowCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Options flow data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionsFlow {
    pub ticker: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub call_volume: u32,
    pub put_volume: u32,
    pub call_put_ratio: f64,
    pub unusual_call_volume: bool,
    pub unusual_put_volume: bool,
    pub net_premium: f64,
    pub bullish_flow: bool,
    pub implied_move_pct: f64,
    pub block_trades: Vec<BlockTrade>,
    pub sweep_count: u32,
    pub institutional_activity: bool,
}

impl OptionsFlow {
    /// Check if there's unusual options activity
    pub fn has_unusual_activity(&self) -> bool {
        self.unusual_call_volume
            || self.unusual_put_volume
            || self.sweep_count > 10
            || self.institutional_activity
    }

    /// Calculate flow signal strength
    pub fn signal_strength(&self) -> FlowSignal {
        let score = (self.call_put_ratio - 1.0).abs();
        let has_unusual = self.has_unusual_activity();

        if score > 2.0 && has_unusual {
            if self.bullish_flow {
                FlowSignal::StrongBullish
            } else {
                FlowSignal::StrongBearish
            }
        } else if score > 1.0 && has_unusual {
            if self.bullish_flow {
                FlowSignal::Bullish
            } else {
                FlowSignal::Bearish
            }
        } else {
            FlowSignal::Neutral
        }
    }

    /// Calculate confidence based on data quality
    pub fn calculate_confidence(&self) -> f64 {
        let volume_confidence = (self.call_volume + self.put_volume).min(10000) as f64 / 10000.0;
        let premium_confidence = self.net_premium.min(1_000_000.0) / 1_000_000.0;
        let block_confidence = (self.block_trades.len() as f64 * 0.2).min(1.0);

        (volume_confidence * 0.4 + premium_confidence * 0.4 + block_confidence * 0.2).min(1.0)
    }

    /// Get total block trade premium
    pub fn total_block_premium(&self) -> f64 {
        self.block_trades.iter().map(|t| t.premium).sum()
    }
}

/// Block trade details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTrade {
    pub option_type: OptionType,
    pub strike: f64,
    pub expiration: String,
    pub volume: u32,
    pub premium: f64,
    pub sentiment: TradeSentiment,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OptionType {
    Call,
    Put,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TradeSentiment {
    Bullish,
    Bearish,
    Neutral,
}

/// Flow signal classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowSignal {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

impl FlowSignal {
    /// Convert to numeric score for aggregation
    pub fn to_score(&self) -> f64 {
        match self {
            FlowSignal::StrongBullish => 1.0,
            FlowSignal::Bullish => 0.5,
            FlowSignal::Neutral => 0.0,
            FlowSignal::Bearish => -0.5,
            FlowSignal::StrongBearish => -1.0,
        }
    }
}

/// Flow sentiment summary
#[derive(Debug, Clone)]
pub struct FlowSentiment {
    pub ticker: String,
    pub sentiment_score: f64, // -1 to 1
    pub call_put_ratio: f64,
    pub unusual_activity: bool,
    pub institutional_bias: bool,
    pub confidence: f64,
}

/// Options flow error
#[derive(Debug, thiserror::Error)]
pub enum OptionsError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limit")]
    RateLimit,
    #[error("Invalid ticker: {0}")]
    InvalidTicker(String),
    #[error("No data available")]
    NoData,
}

/// Options flow dashboard
#[derive(Debug, Clone, Default)]
pub struct OptionsFlowDashboard {
    pub top_bullish_flow: Vec<OptionsFlow>,
    pub top_bearish_flow: Vec<OptionsFlow>,
    pub unusual_activity: Vec<OptionsFlow>,
    pub institutional_flow: Vec<OptionsFlow>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Flow alert configuration
#[derive(Debug, Clone)]
pub struct FlowAlertConfig {
    pub min_premium: f64,
    pub min_call_put_ratio: f64,
    pub sweep_threshold: u32,
    pub alert_on_institutional: bool,
}

impl Default for FlowAlertConfig {
    fn default() -> Self {
        Self {
            min_premium: 500_000.0,
            min_call_put_ratio: 2.0,
            sweep_threshold: 10,
            alert_on_institutional: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_strength() {
        let flow = OptionsFlow {
            ticker: "AAPL".to_string(),
            timestamp: chrono::Utc::now(),
            call_volume: 10000,
            put_volume: 1000,
            call_put_ratio: 10.0,
            unusual_call_volume: true,
            unusual_put_volume: false,
            net_premium: 5_000_000.0,
            bullish_flow: true,
            implied_move_pct: 5.0,
            block_trades: vec![],
            sweep_count: 20,
            institutional_activity: true,
        };

        assert_eq!(flow.signal_strength(), FlowSignal::StrongBullish);
        assert!(flow.has_unusual_activity());
    }

    #[test]
    fn test_flow_signal_scores() {
        assert_eq!(FlowSignal::StrongBullish.to_score(), 1.0);
        assert_eq!(FlowSignal::Bullish.to_score(), 0.5);
        assert_eq!(FlowSignal::Neutral.to_score(), 0.0);
        assert_eq!(FlowSignal::Bearish.to_score(), -0.5);
        assert_eq!(FlowSignal::StrongBearish.to_score(), -1.0);
    }
}
