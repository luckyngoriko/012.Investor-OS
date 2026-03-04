//! Alternative Data Aggregator - Sprint 14
//!
//! Combines all alternative data sources into unified signals:
//! - News sentiment (RSS feeds)
//! - Social media (Reddit, Twitter)
//! - Options flow (unusual activity)
//! - Search trends (Google Trends)
//! - Web scraping (SEC filings, insider data)

use super::{news, options, scraper, social, trends};

/// Alternative data engine
#[derive(Debug, Clone)]
pub struct AlternativeDataEngine {
    news: news::NewsCollector,
    social: social::SocialCollector,
    options: options::OptionsFlowCollector,
    trends: trends::TrendsCollector,
    scraper: scraper::WebScraper,
}

impl AlternativeDataEngine {
    pub fn new() -> Self {
        Self {
            news: news::NewsCollector::new(),
            social: social::SocialCollector::new(),
            options: options::OptionsFlowCollector::new(),
            trends: trends::TrendsCollector::new(),
            scraper: scraper::WebScraper::new(),
        }
    }

    /// Get comprehensive alternative data signal
    pub async fn get_signal(&self, ticker: &str) -> Result<AlternativeSignal, AltDataError> {
        // Fetch news sentiment
        let news_articles = self.news.fetch_news(ticker).await?;
        let news_sentiment = self.aggregate_news_sentiment(&news_articles).await;

        // Fetch social sentiment
        let social_sentiment = self.social.composite_sentiment(ticker).await?;

        // Fetch options flow sentiment
        let options_sentiment = self.options.get_sentiment(ticker).await?;

        // Fetch search trends sentiment
        let search_sentiment = self.trends.get_search_sentiment(ticker).await?;

        // Fetch corporate intelligence
        let corp_intel = self.scraper.get_corporate_intelligence(ticker).await?;

        // Get trending status
        let trending = self.social.trending_tickers().await?;
        let is_trending = trending.contains(&ticker.to_string());

        // Get trend momentum
        let trend_momentum = self.trends.get_momentum(ticker).await?;

        // Calculate weighted composite score
        // News: 25%, Social: 25%, Options: 20%, Trends: 15%, Corporate: 15%
        let composite_score = news_sentiment.overall * 0.25
            + social_sentiment.sentiment_score * 0.25
            + options_sentiment.sentiment_score * 0.20
            + search_sentiment.sentiment_score * 0.15
            + corp_intel.composite_score * 0.15;

        // Calculate overall confidence
        let confidence = self.calculate_confidence(
            &news_articles,
            social_sentiment.mention_count,
            options_sentiment.confidence,
            search_sentiment.attention_trend.to_score().abs(),
        );

        // Determine signal strength
        let strength = self.classify_strength(composite_score, confidence);

        Ok(AlternativeSignal {
            ticker: ticker.to_string(),
            timestamp: chrono::Utc::now(),
            composite_score,
            news_sentiment: news_sentiment.overall,
            social_sentiment: social_sentiment.sentiment_score,
            options_sentiment: options_sentiment.sentiment_score,
            trends_sentiment: search_sentiment.sentiment_score,
            corporate_sentiment: corp_intel.composite_score,
            news_volume: news_articles.len() as u32,
            social_volume: social_sentiment.mention_count,
            options_flow_signal: options_sentiment.unusual_activity,
            trending: is_trending,
            trend_momentum: trend_momentum.strength,
            has_material_news: corp_intel.has_material_developments(),
            confidence,
            strength,
        })
    }

    /// Get signals for multiple tickers
    pub async fn get_signals(
        &self,
        tickers: &[String],
    ) -> Vec<Result<AlternativeSignal, AltDataError>> {
        let mut results = vec![];

        for ticker in tickers {
            results.push(self.get_signal(ticker).await);
        }

        results
    }

    /// Get top trending tickers with signals
    pub async fn get_trending_signals(&self) -> Result<Vec<AlternativeSignal>, AltDataError> {
        let trending = self.social.trending_tickers().await?;

        let mut signals = vec![];
        for ticker in trending {
            if let Ok(signal) = self.get_signal(&ticker).await {
                signals.push(signal);
            }
        }

        // Sort by composite score
        signals.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap());

        Ok(signals)
    }

    /// Get signals with strong options flow
    pub async fn get_options_flow_signals(&self) -> Result<Vec<AlternativeSignal>, AltDataError> {
        let unusual = self.options.get_unusual_activity().await?;
        let tickers: Vec<String> = unusual.into_iter().map(|f| f.ticker).collect();

        let mut signals = vec![];
        for ticker in tickers {
            if let Ok(signal) = self.get_signal(&ticker).await {
                if signal.options_flow_signal {
                    signals.push(signal);
                }
            }
        }

        Ok(signals)
    }

    /// Get signals for tickers with material news
    pub async fn get_material_news_signals(
        &self,
        tickers: &[String],
    ) -> Result<Vec<AlternativeSignal>, AltDataError> {
        let mut signals = vec![];

        for ticker in tickers {
            if let Ok(signal) = self.get_signal(ticker).await {
                if signal.has_material_news() {
                    signals.push(signal);
                }
            }
        }

        Ok(signals)
    }

    /// Get comprehensive dashboard data
    pub async fn get_dashboard(&self) -> Result<AlternativeDataDashboard, AltDataError> {
        let trending = self.get_trending_signals().await?;
        let options_signals = self.get_options_flow_signals().await?;
        let search_trending = self.trends.get_trending_tickers().await?;

        let (top_positive, top_negative): (Vec<_>, Vec<_>) = trending
            .clone()
            .into_iter()
            .partition(|s| s.composite_score > 0.0);

        Ok(AlternativeDataDashboard {
            top_positive,
            top_negative,
            trending,
            options_flow_leaders: options_signals,
            surging_interest: search_trending,
            last_updated: chrono::Utc::now(),
        })
    }

    /// Aggregate news sentiment from multiple articles
    async fn aggregate_news_sentiment(
        &self,
        articles: &[news::NewsArticle],
    ) -> news::SentimentScore {
        let mut total_positive = 0.0;
        let mut total_negative = 0.0;
        let mut total_neutral = 0.0;

        for article in articles {
            let sentiment = self.news.analyze_sentiment(article).await;
            total_positive += sentiment.positive;
            total_negative += sentiment.negative;
            total_neutral += sentiment.neutral;
        }

        let total = total_positive + total_negative + total_neutral;
        if total == 0.0 {
            return news::SentimentScore::neutral();
        }

        news::SentimentScore {
            positive: total_positive / total,
            negative: total_negative / total,
            neutral: total_neutral / total,
            overall: (total_positive - total_negative) / total,
            label: news::SentimentLabel::Neutral,
        }
    }

    /// Calculate confidence based on data volume and quality
    fn calculate_confidence(
        &self,
        articles: &[news::NewsArticle],
        social_mentions: u32,
        options_confidence: f64,
        trends_score: f64,
    ) -> f64 {
        let news_weight = articles.len().min(10) as f64 / 10.0;
        let social_weight = social_mentions.min(1000) as f64 / 1000.0;

        (news_weight * 0.3 + social_weight * 0.3 + options_confidence * 0.25 + trends_score * 0.15)
            .min(1.0)
    }

    /// Classify signal strength
    fn classify_strength(&self, score: f64, confidence: f64) -> SignalStrength {
        if score > 0.5 && confidence > 0.6 {
            SignalStrength::StrongBuy
        } else if score > 0.2 && confidence > 0.5 {
            SignalStrength::Buy
        } else if score < -0.5 && confidence > 0.6 {
            SignalStrength::StrongSell
        } else if score < -0.2 && confidence > 0.5 {
            SignalStrength::Sell
        } else {
            SignalStrength::Neutral
        }
    }
}

impl Default for AlternativeDataEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified alternative data signal
#[derive(Debug, Clone)]
pub struct AlternativeSignal {
    pub ticker: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub composite_score: f64,     // -1 to 1
    pub news_sentiment: f64,      // -1 to 1
    pub social_sentiment: f64,    // -1 to 1
    pub options_sentiment: f64,   // -1 to 1
    pub trends_sentiment: f64,    // -1 to 1
    pub corporate_sentiment: f64, // -1 to 1
    pub news_volume: u32,
    pub social_volume: u32,
    pub options_flow_signal: bool,
    pub trending: bool,
    pub trend_momentum: f64,
    pub has_material_news: bool,
    pub confidence: f64, // 0 to 1
    pub strength: SignalStrength,
}

impl AlternativeSignal {
    /// Get the signal strength enum
    pub fn signal_strength(&self) -> SignalStrength {
        self.strength
    }

    /// Check if signal is actionable
    pub fn is_actionable(&self) -> bool {
        self.confidence > 0.5 && self.composite_score.abs() > 0.3
    }

    /// Check if there is material news
    pub fn has_material_news(&self) -> bool {
        self.has_material_news
    }

    /// Get primary signal source
    pub fn primary_source(&self) -> SignalSource {
        let scores = [
            (SignalSource::News, self.news_sentiment.abs()),
            (SignalSource::SocialMedia, self.social_sentiment.abs()),
            (SignalSource::OptionsFlow, self.options_sentiment.abs()),
            (SignalSource::Trends, self.trends_sentiment.abs()),
            (SignalSource::Corporate, self.corporate_sentiment.abs()),
        ];

        scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(source, _)| *source)
            .unwrap_or(SignalSource::News)
    }

    /// Get signal summary
    pub fn summary(&self) -> String {
        format!(
            "{}: {} (score: {:.2}, confidence: {:.0}%, source: {:?})",
            self.ticker,
            match self.strength {
                SignalStrength::StrongBuy => "Strong Buy",
                SignalStrength::Buy => "Buy",
                SignalStrength::Neutral => "Neutral",
                SignalStrength::Sell => "Sell",
                SignalStrength::StrongSell => "Strong Sell",
            },
            self.composite_score,
            self.confidence * 100.0,
            self.primary_source()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalStrength {
    StrongBuy,
    Buy,
    Neutral,
    Sell,
    StrongSell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSource {
    News,
    SocialMedia,
    OptionsFlow,
    Trends,
    Corporate,
}

/// Alternative data error
#[derive(Debug, thiserror::Error)]
pub enum AltDataError {
    #[error("News error: {0}")]
    News(String),
    #[error("Social error: {0}")]
    Social(String),
    #[error("Options error: {0}")]
    Options(String),
    #[error("Trends error: {0}")]
    Trends(String),
    #[error("Scraper error: {0}")]
    Scraper(String),
    #[error("No data available")]
    NoData,
}

impl From<news::NewsError> for AltDataError {
    fn from(e: news::NewsError) -> Self {
        AltDataError::News(e.to_string())
    }
}

impl From<social::SocialError> for AltDataError {
    fn from(e: social::SocialError) -> Self {
        AltDataError::Social(e.to_string())
    }
}

impl From<options::OptionsError> for AltDataError {
    fn from(e: options::OptionsError) -> Self {
        AltDataError::Options(e.to_string())
    }
}

impl From<trends::TrendsError> for AltDataError {
    fn from(e: trends::TrendsError) -> Self {
        AltDataError::Trends(e.to_string())
    }
}

impl From<scraper::ScraperError> for AltDataError {
    fn from(e: scraper::ScraperError) -> Self {
        AltDataError::Scraper(e.to_string())
    }
}

/// Alternative data dashboard
#[derive(Debug, Clone, Default)]
pub struct AlternativeDataDashboard {
    pub top_positive: Vec<AlternativeSignal>,
    pub top_negative: Vec<AlternativeSignal>,
    pub trending: Vec<AlternativeSignal>,
    pub options_flow_leaders: Vec<AlternativeSignal>,
    pub surging_interest: Vec<trends::TrendingTicker>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Signal filter configuration
#[derive(Debug, Clone)]
pub struct SignalFilter {
    pub min_confidence: f64,
    pub require_material_news: bool,
    pub require_options_flow: bool,
    pub min_social_volume: u32,
    pub sentiment_threshold: f64,
}

impl Default for SignalFilter {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            require_material_news: false,
            require_options_flow: false,
            min_social_volume: 10,
            sentiment_threshold: 0.2,
        }
    }
}

impl SignalFilter {
    /// Check if a signal passes the filter
    pub fn passes(&self, signal: &AlternativeSignal) -> bool {
        if signal.confidence < self.min_confidence {
            return false;
        }
        if self.require_material_news && !signal.has_material_news() {
            return false;
        }
        if self.require_options_flow && !signal.options_flow_signal {
            return false;
        }
        if signal.social_volume < self.min_social_volume {
            return false;
        }
        if signal.composite_score.abs() < self.sentiment_threshold {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_strength_classification() {
        let signal = AlternativeSignal {
            ticker: "AAPL".to_string(),
            timestamp: chrono::Utc::now(),
            composite_score: 0.8,
            news_sentiment: 0.5,
            social_sentiment: 0.6,
            options_sentiment: 0.4,
            trends_sentiment: 0.3,
            corporate_sentiment: 0.5,
            news_volume: 10,
            social_volume: 100,
            options_flow_signal: true,
            trending: true,
            trend_momentum: 0.7,
            has_material_news: true,
            confidence: 0.8,
            strength: SignalStrength::StrongBuy,
        };

        assert!(signal.is_actionable());
        assert!(signal.has_material_news());
        assert_eq!(signal.signal_strength(), SignalStrength::StrongBuy);
    }

    #[test]
    fn test_signal_filter() {
        let filter = SignalFilter::default();

        let passing_signal = AlternativeSignal {
            ticker: "AAPL".to_string(),
            timestamp: chrono::Utc::now(),
            composite_score: 0.5,
            news_sentiment: 0.4,
            social_sentiment: 0.4,
            options_sentiment: 0.3,
            trends_sentiment: 0.2,
            corporate_sentiment: 0.3,
            news_volume: 5,
            social_volume: 50,
            options_flow_signal: false,
            trending: false,
            trend_momentum: 0.5,
            has_material_news: false,
            confidence: 0.6,
            strength: SignalStrength::Buy,
        };

        assert!(filter.passes(&passing_signal));

        let failing_signal = AlternativeSignal {
            ticker: "TSLA".to_string(),
            timestamp: chrono::Utc::now(),
            composite_score: 0.1,
            news_sentiment: 0.1,
            social_sentiment: 0.1,
            options_sentiment: 0.1,
            trends_sentiment: 0.1,
            corporate_sentiment: 0.1,
            news_volume: 1,
            social_volume: 5,
            options_flow_signal: false,
            trending: false,
            trend_momentum: 0.1,
            has_material_news: false,
            confidence: 0.3,
            strength: SignalStrength::Neutral,
        };

        assert!(!filter.passes(&failing_signal));
    }
}
