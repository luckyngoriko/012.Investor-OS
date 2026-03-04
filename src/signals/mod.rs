//! Trading Signals Module
//!
//! CQ (Composite Quality) calculation and signal generation

/// Quality score (1-100)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QualityScore(pub u8);

impl QualityScore {
    pub fn inner(&self) -> f64 {
        self.0 as f64 / 100.0
    }
}

/// Ticker signals collection
#[derive(Debug, Clone)]
pub struct TickerSignals {
    pub quality_score: QualityScore,
    pub value_score: QualityScore,
    pub momentum_score: QualityScore,
    pub insider_score: QualityScore,
    pub sentiment_score: QualityScore,
    pub regime_fit: QualityScore,
    pub composite_quality: QualityScore,

    // Insider features
    pub insider_flow_ratio: f64,
    pub insider_cluster_signal: bool,

    // Sentiment features
    pub news_sentiment: f64,
    pub social_sentiment: f64,

    // Regime features
    pub vix_level: f64,
    pub market_breadth: f64,

    // Technical features
    pub breakout_score: f64,
    pub atr_trend: f64,
    pub rsi_14: f64,
    pub macd_signal: f64,
}

impl Default for TickerSignals {
    fn default() -> Self {
        Self {
            quality_score: QualityScore(50),
            value_score: QualityScore(50),
            momentum_score: QualityScore(50),
            insider_score: QualityScore(50),
            sentiment_score: QualityScore(50),
            regime_fit: QualityScore(50),
            composite_quality: QualityScore(50),
            insider_flow_ratio: 0.0,
            insider_cluster_signal: false,
            news_sentiment: 0.0,
            social_sentiment: 0.0,
            vix_level: 20.0,
            market_breadth: 0.5,
            breakout_score: 0.0,
            atr_trend: 0.0,
            rsi_14: 50.0,
            macd_signal: 0.0,
        }
    }
}
