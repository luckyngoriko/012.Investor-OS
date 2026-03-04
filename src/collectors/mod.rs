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
pub use news::{NewsArticle, NewsCollector, SentimentScore};
pub use options::{FlowSentiment, FlowSignal, OptionsFlow, OptionsFlowCollector};
pub use scraper::{CorporateIntelligence, InsiderSentiment, SecFiling, WebScraper};
pub use social::{SocialCollector, SocialSentiment};
pub use trends::{AttentionTrend, SearchSentiment, TrendData, TrendsCollector};
