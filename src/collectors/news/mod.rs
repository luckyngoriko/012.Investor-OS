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

    /// Fetch news for a ticker from configured RSS feeds.
    ///
    /// Makes real HTTP requests to each feed URL. Articles are filtered by
    /// ticker mention in the title. Returns empty vec with a warning log if
    /// all feeds are unreachable.
    pub async fn fetch_news(&self, ticker: &str) -> Result<Vec<NewsArticle>, NewsError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| NewsError::Fetch(e.to_string()))?;

        let mut articles = Vec::new();
        let ticker_upper = ticker.to_uppercase();

        for source in &self.sources {
            match client.get(&source.rss_url).send().await {
                Ok(response) => {
                    let body = response
                        .text()
                        .await
                        .map_err(|e| NewsError::Fetch(e.to_string()))?;
                    let parsed = Self::parse_rss_xml(&body, &source.name, &ticker_upper);
                    articles.extend(parsed);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch RSS from {}: {}", source.name, e);
                }
            }
        }

        Ok(articles)
    }

    /// Parse RSS/Atom XML into NewsArticle items.
    /// Filters articles whose title contains the ticker symbol.
    fn parse_rss_xml(xml: &str, source_name: &str, ticker: &str) -> Vec<NewsArticle> {
        let mut articles = Vec::new();

        // Simple XML extraction — look for <item> or <entry> blocks
        let item_tag = if xml.contains("<entry") {
            "entry"
        } else {
            "item"
        };
        let title_re = format!("<title>([^<]*)</title>");
        let link_re = format!("<link>([^<]*)</link>");
        let desc_re = format!("<description>([^<]*)</description>");

        for block in xml.split(&format!("<{}", item_tag)) {
            let title = Self::extract_tag(block, "title").unwrap_or_default();
            if title.is_empty() || !title.to_uppercase().contains(ticker) {
                continue;
            }
            let link = Self::extract_tag(block, "link")
                .or_else(|| Self::extract_attr(block, "link", "href"))
                .unwrap_or_default();
            let description = Self::extract_tag(block, "description")
                .or_else(|| Self::extract_tag(block, "summary"))
                .unwrap_or_default();

            articles.push(NewsArticle {
                title,
                content: description,
                source: source_name.to_string(),
                url: link,
                published_at: chrono::Utc::now(),
                ticker: ticker.to_string(),
            });
        }

        articles
    }

    /// Extract text content of an XML tag (simple, no nested tags)
    fn extract_tag(block: &str, tag: &str) -> Option<String> {
        let open = format!("<{}>", tag);
        let alt_open = format!("<{} ", tag); // tag with attributes
        let close = format!("</{}>", tag);

        let start_pos = block.find(&open).map(|i| i + open.len()).or_else(|| {
            block
                .find(&alt_open)
                .and_then(|i| block[i..].find('>').map(|j| i + j + 1))
        })?;
        let end_pos = block[start_pos..].find(&close)?;
        let text = &block[start_pos..start_pos + end_pos];
        let text = text
            .replace("<![CDATA[", "")
            .replace("]]>", "")
            .trim()
            .to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Extract href attribute from a self-closing <link ... /> tag (Atom feeds)
    fn extract_attr(block: &str, tag: &str, attr: &str) -> Option<String> {
        let open = format!("<{} ", tag);
        let start = block.find(&open)?;
        let rest = &block[start..];
        let end = rest.find('>')?;
        let tag_str = &rest[..end];
        let attr_prefix = format!("{}=\"", attr);
        let attr_start = tag_str.find(&attr_prefix)? + attr_prefix.len();
        let attr_end = tag_str[attr_start..].find('"')?;
        Some(tag_str[attr_start..attr_start + attr_end].to_string())
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
    fn default() -> Self {
        Self::new()
    }
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
    pub overall: f64, // -1 to 1
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
