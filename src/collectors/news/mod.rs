//! News NLP Pipeline - Sprint 14
//!
//! RSS aggregation, article analysis, sentiment extraction

use serde::Deserialize;

/// News collector
#[derive(Debug, Clone)]
pub struct NewsCollector {
    sources: Vec<NewsSource>,
}

impl NewsCollector {
    pub fn new() -> Self {
        Self {
            sources: vec![
                NewsSource::bloomberg(),
                NewsSource::reuters(),
                NewsSource::cnbc(),
                NewsSource::wsj(),
                NewsSource::ft(),
            ],
        }
    }
    
    /// Fetch news for a ticker
    pub async fn fetch_news(&self, ticker: &str) -> Result<Vec<NewsArticle>, NewsError> {
        let mut articles = vec![];
        
        // Would fetch from RSS feeds
        for _source in &self.sources {
            // Placeholder - would parse RSS
            articles.push(NewsArticle {
                title: format!("{} reports earnings beat", ticker),
                content: format!("{} announced strong Q4 results...", ticker),
                source: "Bloomberg".to_string(),
                url: format!("https://bloomberg.com/news/{}", ticker),
                published_at: chrono::Utc::now(),
                ticker: ticker.to_string(),
            });
        }
        
        Ok(articles)
    }
    
    /// Extract sentiment from news
    pub async fn analyze_sentiment(&self, article: &NewsArticle) -> SentimentScore {
        // Would use NLP (FinBERT from Sprint 10)
        // Placeholder: simple keyword matching
        let text = format!("{} {}", article.title, article.content).to_lowercase();
        
        let positive_words = ["beat", "growth", "strong", "profit", "gain"];
        let negative_words = ["miss", "loss", "decline", "weak", "drop"];
        
        let pos_count = positive_words.iter().filter(|w| text.contains(*w)).count();
        let neg_count = negative_words.iter().filter(|w| text.contains(*w)).count();
        
        let total = pos_count + neg_count;
        if total == 0 {
            return SentimentScore::neutral();
        }
        
        let score = (pos_count as f64 - neg_count as f64) / total as f64;
        
        SentimentScore {
            positive: pos_count as f64 / total as f64,
            negative: neg_count as f64 / total as f64,
            neutral: 0.0,
            overall: score,
            label: if score > 0.1 {
                SentimentLabel::Positive
            } else if score < -0.1 {
                SentimentLabel::Negative
            } else {
                SentimentLabel::Neutral
            },
        }
    }
}

impl Default for NewsCollector {
    fn default() -> Self { Self::new() }
}

/// News source
#[derive(Debug, Clone)]
pub struct NewsSource {
    pub name: String,
    pub rss_url: String,
    pub category: NewsCategory,
}

impl NewsSource {
    pub fn bloomberg() -> Self {
        Self {
            name: "Bloomberg".to_string(),
            rss_url: "https://feeds.bloomberg.com/business/news.rss".to_string(),
            category: NewsCategory::Business,
        }
    }
    
    pub fn reuters() -> Self {
        Self {
            name: "Reuters".to_string(),
            rss_url: "https://reuters.com/rss/news.rss".to_string(),
            category: NewsCategory::Business,
        }
    }
    
    pub fn cnbc() -> Self {
        Self {
            name: "CNBC".to_string(),
            rss_url: "https://cnbc.com/rss/news.rss".to_string(),
            category: NewsCategory::Finance,
        }
    }
    
    pub fn wsj() -> Self {
        Self {
            name: "Wall Street Journal".to_string(),
            rss_url: "https://wsj.com/rss/news.rss".to_string(),
            category: NewsCategory::Finance,
        }
    }
    
    pub fn ft() -> Self {
        Self {
            name: "Financial Times".to_string(),
            rss_url: "https://ft.com/rss/news.rss".to_string(),
            category: NewsCategory::Finance,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NewsCategory {
    Business,
    Finance,
    Technology,
    Politics,
}

/// News article
#[derive(Debug, Clone, Deserialize)]
pub struct NewsArticle {
    pub title: String,
    pub content: String,
    pub source: String,
    pub url: String,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub ticker: String,
}

/// Sentiment analysis result
#[derive(Debug, Clone)]
pub struct SentimentScore {
    pub positive: f64,
    pub negative: f64,
    pub neutral: f64,
    pub overall: f64,  // -1 to 1
    pub label: SentimentLabel,
}

impl SentimentScore {
    pub fn neutral() -> Self {
        Self {
            positive: 0.33,
            negative: 0.33,
            neutral: 0.34,
            overall: 0.0,
            label: SentimentLabel::Neutral,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SentimentLabel {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

/// Composite sentiment from multiple sources
#[derive(Debug, Clone)]
pub struct CompositeSentiment {
    pub ticker: String,
    pub news_sentiment: f64,
    pub social_sentiment: f64,
    pub overall_sentiment: f64,
    pub news_volume: u32,
    pub social_volume: u32,
}

/// News error
#[derive(Debug, thiserror::Error)]
pub enum NewsError {
    #[error("Fetch error: {0}")]
    Fetch(String),
    #[error("Parse error: {0}")]
    Parse(String),
}
