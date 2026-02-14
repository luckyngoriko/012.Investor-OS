//! Sentiment Reader Agent
//!
//! Analyzes news, social media, and other textual data for market sentiment.
//! Provides sentiment scores and identifies key topics.

use super::*;
use crate::agent::{AgentMessage, MessagePayload, MessageType};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

/// Sentiment Reader Agent implementation
pub struct SentimentReaderAgent {
    config: AgentConfig,
    status: AgentStatus,
    /// News sources to monitor
    news_sources: Vec<String>,
    /// Social media sources
    social_sources: Vec<String>,
    /// Sentiment cache to avoid recalculating
    sentiment_cache: HashMap<String, SentimentAnalysis>,
    /// Cache TTL in seconds
    cache_ttl_secs: u64,
}

impl SentimentReaderAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            status: AgentStatus::Active,
            news_sources: vec![
                "bloomberg".to_string(),
                "reuters".to_string(),
                "wsj".to_string(),
                "cnbc".to_string(),
            ],
            social_sources: vec![
                "twitter".to_string(),
                "reddit".to_string(),
                "stocktwits".to_string(),
            ],
            sentiment_cache: HashMap::new(),
            cache_ttl_secs: 300, // 5 minutes
        }
    }
    
    pub fn with_sources(mut self, news: Vec<String>, social: Vec<String>) -> Self {
        self.news_sources = news;
        self.social_sources = social;
        self
    }
    
    /// Analyze sentiment for a symbol
    fn analyze_sentiment(&self, symbol: &str) -> SentimentAnalysis {
        // In production: fetch real news/social data and run NLP
        // For now, generate deterministic sentiment based on symbol
        
        let symbol_hash = symbol.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        let sentiment_value = symbol_hash % 5;
        
        let overall_sentiment = match sentiment_value {
            0 => Sentiment::VeryBullish,
            1 => Sentiment::Bullish,
            2 => Sentiment::Neutral,
            3 => Sentiment::Bearish,
            _ => Sentiment::VeryBearish,
        };
        
        // Generate sentiment scores (-1.0 to 1.0)
        let news_sentiment = match overall_sentiment {
            Sentiment::VeryBullish => 0.8,
            Sentiment::Bullish => 0.4,
            Sentiment::Neutral => 0.0,
            Sentiment::Bearish => -0.4,
            Sentiment::VeryBearish => -0.8,
        };
        
        let variation: f64 = ((symbol_hash % 10) as f64 - 5.0) / 20.0;
        let social_sentiment: f64 = (news_sentiment + variation).clamp(-1.0, 1.0);
        
        // Generate key topics
        let topics = vec![
            format!("earnings_{}", symbol.to_lowercase()),
            format!("growth_{}", symbol.to_lowercase()),
            "market_sentiment".to_string(),
            "sector_rotation".to_string(),
        ];
        
        SentimentAnalysis {
            overall_sentiment,
            news_sentiment,
            social_sentiment,
            key_topics: topics,
        }
    }
    
    /// Process news item
    fn process_news(&self, headline: &str, source: &str) -> f64 {
        // Simple keyword-based sentiment
        let headline_lower = headline.to_lowercase();
        
        let positive_words = ["surge", "rally", "gain", "growth", "profit", "beat", "strong"];
        let negative_words = ["fall", "drop", "crash", "loss", "miss", "weak", "decline"];
        
        let mut score = 0.0;
        
        for word in &positive_words {
            if headline_lower.contains(word) {
                score += 0.2;
            }
        }
        
        for word in &negative_words {
            if headline_lower.contains(word) {
                score -= 0.2;
            }
        }
        
        // Adjust by source reliability (simplified)
        let source_weight = match source.to_lowercase().as_str() {
            "bloomberg" | "reuters" => 1.0,
            "wsj" | "ft" => 0.9,
            "cnbc" | "yahoo" => 0.7,
            _ => 0.5,
        };
        
        let result: f64 = score * source_weight;
        result.clamp(-1.0, 1.0)
    }
    
    /// Combine multiple sentiment sources
    fn combine_sentiment(&self, news: f64, social: f64) -> Sentiment {
        let combined = (news * 0.6) + (social * 0.4);
        
        match combined {
            x if x > 0.6 => Sentiment::VeryBullish,
            x if x > 0.2 => Sentiment::Bullish,
            x if x > -0.2 => Sentiment::Neutral,
            x if x > -0.6 => Sentiment::Bearish,
            _ => Sentiment::VeryBearish,
        }
    }
    
    /// Get sentiment signal strength (-1.0 to 1.0)
    fn get_signal_strength(&self, sentiment: &SentimentAnalysis) -> f64 {
        let base = match sentiment.overall_sentiment {
            Sentiment::VeryBullish => 1.0,
            Sentiment::Bullish => 0.5,
            Sentiment::Neutral => 0.0,
            Sentiment::Bearish => -0.5,
            Sentiment::VeryBearish => -1.0,
        };
        
        // Adjust by consistency between news and social
        let consistency = 1.0 - (sentiment.news_sentiment - sentiment.social_sentiment).abs();
        
        base * (0.5 + 0.5 * consistency)
    }
}

#[async_trait]
impl Agent for SentimentReaderAgent {
    fn id(&self) -> &AgentId {
        &self.config.id
    }
    
    fn role(&self) -> AgentRole {
        AgentRole::SentimentReader
    }
    
    fn status(&self) -> AgentStatus {
        self.status
    }
    
    async fn process(&mut self, task: Task) -> Result<TaskResult, AgentError> {
        let start_time = std::time::Instant::now();
        
        let output = match &task.task_type {
            super::super::TaskType::AnalyzeSentiment { symbol, sources } => {
                debug!("SentimentReader analyzing sentiment for {} from {:?}", symbol, sources);
                
                let analysis = self.analyze_sentiment(symbol);
                
                let signal = self.get_signal_strength(&analysis);
                
                info!(
                    "Sentiment for {}: {:?} (news: {:.2}, social: {:.2}, signal: {:.2})",
                    symbol,
                    analysis.overall_sentiment,
                    analysis.news_sentiment,
                    analysis.social_sentiment,
                    signal
                );
                
                // Cache result
                self.sentiment_cache.insert(symbol.clone(), analysis.clone());
                
                TaskOutput::SentimentAnalysis(analysis)
            }
            _ => {
                TaskOutput::Error(format!("Unsupported task type for SentimentReader: {:?}", task.task_type))
            }
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(TaskResult {
            task_id: task.id,
            agent_id: self.config.id.clone(),
            status: TaskStatus::Success,
            output,
            execution_time_ms: execution_time,
        })
    }
    
    async fn on_message(&mut self, msg: AgentMessage) -> Result<(), AgentError> {
        match msg.msg_type {
            MessageType::Broadcast => {
                // Could process broadcast news
                if let MessagePayload::Broadcast(text) = &msg.payload {
                    debug!("SentimentReader received broadcast: {}", text);
                }
            }
            MessageType::Observation => {
                // Process market observations for sentiment context
                if let MessagePayload::Observation(obs) = &msg.payload {
                    debug!("SentimentReader received observation for {}", obs.symbol);
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn pause(&mut self) {
        self.status = AgentStatus::Paused;
        info!("SentimentReader {} paused", self.config.id);
    }
    
    async fn resume(&mut self) {
        self.status = AgentStatus::Active;
        info!("SentimentReader {} resumed", self.config.id);
    }
    
    async fn shutdown(&mut self) {
        self.status = AgentStatus::Shutdown;
        info!("SentimentReader {} shutdown", self.config.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::AgentConfig;

    #[tokio::test]
    async fn test_sentiment_reader_creation() {
        let config = AgentConfig::new(AgentRole::SentimentReader, "Sentiment Agent");
        let agent = SentimentReaderAgent::new(config);
        
        assert_eq!(agent.role(), AgentRole::SentimentReader);
    }

    #[test]
    fn test_sentiment_analysis() {
        let config = AgentConfig::new(AgentRole::SentimentReader, "Sentiment Agent");
        let agent = SentimentReaderAgent::new(config);
        
        let analysis = agent.analyze_sentiment("AAPL");
        
        assert!(!analysis.key_topics.is_empty());
        assert!(analysis.news_sentiment >= -1.0 && analysis.news_sentiment <= 1.0);
        assert!(analysis.social_sentiment >= -1.0 && analysis.social_sentiment <= 1.0);
    }

    #[test]
    fn test_news_processing() {
        let config = AgentConfig::new(AgentRole::SentimentReader, "Sentiment Agent");
        let agent = SentimentReaderAgent::new(config);
        
        let positive_news = "Apple shares surge on strong earnings beat";
        let negative_news = "Tesla stock crashes after production miss";
        
        let positive_score = agent.process_news(positive_news, "bloomberg");
        let negative_score = agent.process_news(negative_news, "reuters");
        
        assert!(positive_score > 0.0);
        assert!(negative_score < 0.0);
    }

    #[test]
    fn test_signal_strength() {
        let config = AgentConfig::new(AgentRole::SentimentReader, "Sentiment Agent");
        let agent = SentimentReaderAgent::new(config);
        
        let very_bullish = SentimentAnalysis {
            overall_sentiment: Sentiment::VeryBullish,
            news_sentiment: 0.8,
            social_sentiment: 0.75,
            key_topics: vec![],
        };
        
        let neutral = SentimentAnalysis {
            overall_sentiment: Sentiment::Neutral,
            news_sentiment: 0.1,
            social_sentiment: -0.05,
            key_topics: vec![],
        };
        
        let strong_signal = agent.get_signal_strength(&very_bullish);
        let weak_signal = agent.get_signal_strength(&neutral);
        
        assert!(strong_signal.abs() > weak_signal.abs());
    }

    #[test]
    fn test_sentiment_combination() {
        let config = AgentConfig::new(AgentRole::SentimentReader, "Sentiment Agent");
        let agent = SentimentReaderAgent::new(config);
        
        // Strong positive news, neutral social
        let sentiment = agent.combine_sentiment(0.8, 0.1);
        assert!(matches!(sentiment, Sentiment::Bullish | Sentiment::VeryBullish));
        
        // Negative news, neutral social
        let sentiment = agent.combine_sentiment(-0.5, -0.1);
        assert!(matches!(sentiment, Sentiment::Bearish));
    }
}
