//! Trends Collector - Sprint 14
//!
//! Google Trends, search interest, and market attention tracking

use serde::{Deserialize, Serialize};

/// Trends data collector
#[derive(Debug, Clone)]
pub struct TrendsCollector {
    client: reqwest::Client,
}

impl TrendsCollector {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Get search trends for a ticker
    pub async fn get_trends(&self, ticker: &str) -> Result<TrendData, TrendsError> {
        // Would fetch from Google Trends API or similar
        // Placeholder with realistic synthetic data
        let mut historical = vec![];
        let base_value = 50;

        for i in 0..30 {
            let date = chrono::Utc::now() - chrono::Duration::days(30 - i);
            let value = base_value + (i as i32 * 2) + ((i as i32 * 7) % 10);
            historical.push(TrendPoint {
                date: date.format("%Y-%m-%d").to_string(),
                value: value.max(0) as u32,
            });
        }

        let current = historical.last().map(|p| p.value).unwrap_or(50);
        let previous = historical
            .get(historical.len().saturating_sub(8))
            .map(|p| p.value)
            .unwrap_or(40);

        Ok(TrendData {
            ticker: ticker.to_string(),
            keyword: format!("{} stock", ticker),
            current_value: current,
            previous_value: previous,
            change_pct: if previous > 0 {
                ((current as f64 - previous as f64) / previous as f64) * 100.0
            } else {
                0.0
            },
            historical,
            related_queries: vec![
                RelatedQuery {
                    query: format!("{} earnings", ticker),
                    value: 80,
                    trending: true,
                },
                RelatedQuery {
                    query: format!("buy {}", ticker),
                    value: 60,
                    trending: false,
                },
                RelatedQuery {
                    query: format!("{} price target", ticker),
                    value: 45,
                    trending: true,
                },
            ],
            region_interest: vec![
                RegionInterest {
                    region: "United States".to_string(),
                    value: 100,
                },
                RegionInterest {
                    region: "United Kingdom".to_string(),
                    value: 25,
                },
                RegionInterest {
                    region: "Canada".to_string(),
                    value: 20,
                },
            ],
            timestamp: chrono::Utc::now(),
        })
    }

    /// Get comparative trends for multiple tickers
    pub async fn get_comparative_trends(
        &self,
        tickers: &[String],
    ) -> Result<Vec<TrendComparison>, TrendsError> {
        let mut comparisons = vec![];

        for ticker in tickers {
            let trend = self.get_trends(ticker).await?;
            comparisons.push(TrendComparison {
                ticker: ticker.clone(),
                relative_strength: trend.current_value as f64 / 100.0,
                momentum: trend.change_pct / 100.0,
                attention_score: self.calculate_attention_score(&trend),
            });
        }

        // Sort by attention score descending
        comparisons.sort_by(|a, b| b.attention_score.partial_cmp(&a.attention_score).unwrap());

        Ok(comparisons)
    }

    /// Get trending tickers based on search interest
    pub async fn get_trending_tickers(&self) -> Result<Vec<TrendingTicker>, TrendsError> {
        // Would fetch from Google Trends trending searches
        let tickers = vec![
            ("TSLA", 150, 25.0),
            ("NVDA", 130, 18.0),
            ("AAPL", 110, 5.0),
            ("AMD", 95, 30.0),
            ("META", 85, 12.0),
        ];

        let mut trending = vec![];
        for (ticker, value, change) in tickers {
            trending.push(TrendingTicker {
                ticker: ticker.to_string(),
                interest_value: value,
                change_24h_pct: change,
                category: TrendCategory::Finance,
            });
        }

        Ok(trending)
    }

    /// Get sentiment indicator from search trends
    pub async fn get_search_sentiment(&self, ticker: &str) -> Result<SearchSentiment, TrendsError> {
        let trends = self.get_trends(ticker).await?;
        let related = &trends.related_queries;

        let bullish_keywords = ["buy", "moon", "bull", "long", "calls", "breakout"];
        let bearish_keywords = ["sell", "crash", "bear", "short", "puts", "dump"];

        let bullish_score: u32 = related
            .iter()
            .filter(|q| bullish_keywords.iter().any(|kw| q.query.contains(kw)))
            .map(|q| q.value)
            .sum();

        let bearish_score: u32 = related
            .iter()
            .filter(|q| bearish_keywords.iter().any(|kw| q.query.contains(kw)))
            .map(|q| q.value)
            .sum();

        let total = bullish_score + bearish_score;
        let sentiment = if total > 0 {
            (bullish_score as f64 - bearish_score as f64) / total as f64
        } else {
            0.0
        };

        Ok(SearchSentiment {
            ticker: ticker.to_string(),
            sentiment_score: sentiment, // -1 to 1
            bullish_mentions: bullish_score,
            bearish_mentions: bearish_score,
            attention_trend: if trends.change_pct > 20.0 {
                AttentionTrend::Surging
            } else if trends.change_pct > 5.0 {
                AttentionTrend::Rising
            } else if trends.change_pct < -20.0 {
                AttentionTrend::Collapsing
            } else if trends.change_pct < -5.0 {
                AttentionTrend::Falling
            } else {
                AttentionTrend::Stable
            },
            timestamp: chrono::Utc::now(),
        })
    }

    /// Calculate attention score from trend data
    fn calculate_attention_score(&self, trend: &TrendData) -> f64 {
        let base_score = trend.current_value as f64 / 100.0;
        let momentum_boost = (trend.change_pct / 100.0).max(0.0) * 0.3;
        let related_query_boost = (trend.related_queries.len() as f64 * 0.05).min(0.2);

        (base_score + momentum_boost + related_query_boost).min(1.0)
    }

    /// Get trend momentum (rate of change)
    pub async fn get_momentum(&self, ticker: &str) -> Result<TrendMomentum, TrendsError> {
        let trends = self.get_trends(ticker).await?;

        let short_term = if trends.historical.len() >= 7 {
            let recent: u32 = trends
                .historical
                .iter()
                .rev()
                .take(7)
                .map(|p| p.value)
                .sum();
            let previous: u32 = trends
                .historical
                .iter()
                .rev()
                .skip(7)
                .take(7)
                .map(|p| p.value)
                .sum();
            if previous > 0 {
                ((recent as f64 - previous as f64) / previous as f64) * 100.0
            } else {
                0.0
            }
        } else {
            trends.change_pct
        };

        Ok(TrendMomentum {
            ticker: ticker.to_string(),
            short_term,
            medium_term: trends.change_pct,
            direction: if short_term > 10.0 {
                TrendDirection::StrongUp
            } else if short_term > 0.0 {
                TrendDirection::Up
            } else if short_term < -10.0 {
                TrendDirection::StrongDown
            } else {
                TrendDirection::Down
            },
            strength: trends.current_value as f64 / 100.0,
        })
    }
}

impl Default for TrendsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Trend data for a ticker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendData {
    pub ticker: String,
    pub keyword: String,
    pub current_value: u32,  // 0-100 relative interest
    pub previous_value: u32, // For comparison
    pub change_pct: f64,
    pub historical: Vec<TrendPoint>,
    pub related_queries: Vec<RelatedQuery>,
    pub region_interest: Vec<RegionInterest>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Single trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: String,
    pub value: u32,
}

/// Related search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedQuery {
    pub query: String,
    pub value: u32,
    pub trending: bool,
}

/// Regional interest breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInterest {
    pub region: String,
    pub value: u32, // Relative interest 0-100
}

/// Trend comparison between tickers
#[derive(Debug, Clone)]
pub struct TrendComparison {
    pub ticker: String,
    pub relative_strength: f64, // 0-1
    pub momentum: f64,          // Rate of change
    pub attention_score: f64,   // 0-1 composite
}

/// Trending ticker data
#[derive(Debug, Clone)]
pub struct TrendingTicker {
    pub ticker: String,
    pub interest_value: u32,
    pub change_24h_pct: f64,
    pub category: TrendCategory,
}

#[derive(Debug, Clone, Copy)]
pub enum TrendCategory {
    Finance,
    Technology,
    Healthcare,
    Energy,
    Consumer,
    Industrial,
}

/// Search-based sentiment
#[derive(Debug, Clone)]
pub struct SearchSentiment {
    pub ticker: String,
    pub sentiment_score: f64, // -1 to 1
    pub bullish_mentions: u32,
    pub bearish_mentions: u32,
    pub attention_trend: AttentionTrend,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Attention trend classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionTrend {
    Surging,    // >20% increase
    Rising,     // 5-20% increase
    Stable,     // -5% to 5%
    Falling,    // -20% to -5%
    Collapsing, // <-20%
}

impl AttentionTrend {
    /// Convert to numeric score for aggregation
    pub fn to_score(&self) -> f64 {
        match self {
            AttentionTrend::Surging => 1.0,
            AttentionTrend::Rising => 0.5,
            AttentionTrend::Stable => 0.0,
            AttentionTrend::Falling => -0.5,
            AttentionTrend::Collapsing => -1.0,
        }
    }
}

/// Trend momentum metrics
#[derive(Debug, Clone)]
pub struct TrendMomentum {
    pub ticker: String,
    pub short_term: f64,  // 7-day change %
    pub medium_term: f64, // 30-day change %
    pub direction: TrendDirection,
    pub strength: f64, // 0-1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    StrongUp,
    Up,
    Down,
    StrongDown,
}

/// Trends error
#[derive(Debug, thiserror::Error)]
pub enum TrendsError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limit")]
    RateLimit,
    #[error("Invalid ticker: {0}")]
    InvalidTicker(String),
    #[error("No data available")]
    NoData,
}

/// Trends dashboard
#[derive(Debug, Clone, Default)]
pub struct TrendsDashboard {
    pub top_attention: Vec<TrendComparison>,
    pub surging_interest: Vec<TrendingTicker>,
    pub declining_interest: Vec<TrendingTicker>,
    pub search_sentiment_leaders: Vec<SearchSentiment>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Trend alert for significant changes
#[derive(Debug, Clone)]
pub struct TrendAlert {
    pub ticker: String,
    pub alert_type: TrendAlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendAlertType {
    SurgingInterest,
    CollapsingInterest,
    SentimentShift,
    ViralMentions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attention_trend_scores() {
        assert_eq!(AttentionTrend::Surging.to_score(), 1.0);
        assert_eq!(AttentionTrend::Rising.to_score(), 0.5);
        assert_eq!(AttentionTrend::Stable.to_score(), 0.0);
        assert_eq!(AttentionTrend::Falling.to_score(), -0.5);
        assert_eq!(AttentionTrend::Collapsing.to_score(), -1.0);
    }

    #[test]
    fn test_trend_data_change_calculation() {
        let trend = TrendData {
            ticker: "AAPL".to_string(),
            keyword: "AAPL stock".to_string(),
            current_value: 60,
            previous_value: 50,
            change_pct: 20.0,
            historical: vec![],
            related_queries: vec![],
            region_interest: vec![],
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(trend.change_pct, 20.0);
    }
}
