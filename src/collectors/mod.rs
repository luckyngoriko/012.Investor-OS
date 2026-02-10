//! Alternative Data Collectors - Sprint 14
//!
//! Social media sentiment, news analysis, options flow, trends, and web scraping

pub mod alt;
pub mod news;
pub mod options;
pub mod scraper;
pub mod social;
pub mod trends;

pub use alt::{AlternativeDataEngine, AlternativeSignal, SignalStrength};
pub use news::{NewsCollector, NewsArticle, SentimentScore};
pub use options::{OptionsFlowCollector, OptionsFlow, FlowSignal, FlowSentiment};
pub use scraper::{WebScraper, CorporateIntelligence, SecFiling, InsiderSentiment};
pub use social::{SocialCollector, SocialSentiment};
pub use trends::{TrendsCollector, TrendData, SearchSentiment, AttentionTrend};
