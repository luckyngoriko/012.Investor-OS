//! Social Media Sentiment - Sprint 14
//!
//! Reddit, Twitter/X sentiment analysis


/// Social media collector
#[derive(Debug, Clone)]
pub struct SocialCollector {
    reddit: RedditClient,
    twitter: TwitterClient,
}

impl SocialCollector {
    pub fn new() -> Self {
        Self {
            reddit: RedditClient::new(),
            twitter: TwitterClient::new(),
        }
    }
    
    /// Get Reddit sentiment for a ticker
    pub async fn reddit_sentiment(&self, ticker: &str) -> Result<SocialSentiment, SocialError> {
        let posts = self.reddit.fetch_posts(ticker).await?;
        
        let mut total_score = 0.0;
        let mut total_comments = 0;
        
        for post in &posts {
            total_score += post.sentiment_score;
            total_comments += post.comment_count;
        }
        
        let avg_sentiment = if posts.is_empty() {
            0.0
        } else {
            total_score / posts.len() as f64
        };
        
        Ok(SocialSentiment {
            platform: Platform::Reddit,
            ticker: ticker.to_string(),
            sentiment_score: avg_sentiment,
            mention_count: posts.len() as u32,
            engagement: total_comments,
            trending: posts.len() > 10,
        })
    }
    
    /// Get Twitter sentiment for a ticker
    pub async fn twitter_sentiment(&self, ticker: &str) -> Result<SocialSentiment, SocialError> {
        let tweets = self.twitter.fetch_tweets(ticker).await?;
        
        let total_score: f64 = tweets.iter().map(|t| t.sentiment).sum();
        let avg_sentiment = if tweets.is_empty() {
            0.0
        } else {
            total_score / tweets.len() as f64
        };
        
        Ok(SocialSentiment {
            platform: Platform::Twitter,
            ticker: ticker.to_string(),
            sentiment_score: avg_sentiment,
            mention_count: tweets.len() as u32,
            engagement: tweets.iter().map(|t| t.likes + t.retweets).sum(),
            trending: tweets.len() > 100,
        })
    }
    
    /// Get composite social sentiment
    pub async fn composite_sentiment(&self, ticker: &str) -> Result<SocialSentiment, SocialError> {
        let reddit = self.reddit_sentiment(ticker).await?;
        let twitter = self.twitter_sentiment(ticker).await?;
        
        // Weighted average
        let total_mentions = reddit.mention_count + twitter.mention_count;
        if total_mentions == 0 {
            return Ok(SocialSentiment {
                platform: Platform::Combined,
                ticker: ticker.to_string(),
                sentiment_score: 0.0,
                mention_count: 0,
                engagement: 0,
                trending: false,
            });
        }
        
        let weighted_score = (
            reddit.sentiment_score * reddit.mention_count as f64 +
            twitter.sentiment_score * twitter.mention_count as f64
        ) / total_mentions as f64;
        
        Ok(SocialSentiment {
            platform: Platform::Combined,
            ticker: ticker.to_string(),
            sentiment_score: weighted_score,
            mention_count: total_mentions,
            engagement: reddit.engagement + twitter.engagement,
            trending: reddit.trending || twitter.trending,
        })
    }
    
    /// Get trending tickers on social media
    pub async fn trending_tickers(&self) -> Result<Vec<String>, SocialError> {
        // Would fetch from r/wallstreetbets, StockTwits, etc.
        Ok(vec![
            "GME".to_string(),
            "AMC".to_string(),
            "TSLA".to_string(),
            "AAPL".to_string(),
        ])
    }
}

impl Default for SocialCollector {
    fn default() -> Self { Self::new() }
}

/// Reddit client
#[derive(Debug, Clone)]
pub struct RedditClient {
    user_agent: String,
}

impl Default for RedditClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RedditClient {
    pub fn new() -> Self {
        Self {
            user_agent: "InvestorOS/1.0".to_string(),
        }
    }
    
    pub async fn fetch_posts(&self, ticker: &str) -> Result<Vec<RedditPost>, SocialError> {
        // Would use Reddit API
        // Placeholder
        Ok(vec![
            RedditPost {
                title: format!("${} to the moon!", ticker),
                sentiment_score: 0.8,
                upvotes: 1000,
                comment_count: 200,
                subreddit: "wallstreetbets".to_string(),
            },
            RedditPost {
                title: format!("${} earnings discussion", ticker),
                sentiment_score: 0.3,
                upvotes: 500,
                comment_count: 100,
                subreddit: "investing".to_string(),
            },
        ])
    }
}

/// Twitter/X client
#[derive(Debug, Clone)]
pub struct TwitterClient {
    bearer_token: Option<String>,
}

impl Default for TwitterClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitterClient {
    pub fn new() -> Self {
        Self {
            bearer_token: std::env::var("TWITTER_BEARER_TOKEN").ok(),
        }
    }
    
    pub async fn fetch_tweets(&self, ticker: &str) -> Result<Vec<Tweet>, SocialError> {
        // Would use Twitter API v2
        // Placeholder
        Ok(vec![
            Tweet {
                text: format!("Bullish on ${}", ticker),
                sentiment: 0.7,
                likes: 50,
                retweets: 20,
                author_followers: 10000,
            },
            Tweet {
                text: format!("${} looking weak today", ticker),
                sentiment: -0.4,
                likes: 10,
                retweets: 2,
                author_followers: 1000,
            },
        ])
    }
}

/// Reddit post
#[derive(Debug, Clone)]
pub struct RedditPost {
    pub title: String,
    pub sentiment_score: f64,
    pub upvotes: u32,
    pub comment_count: u32,
    pub subreddit: String,
}

/// Tweet
#[derive(Debug, Clone)]
pub struct Tweet {
    pub text: String,
    pub sentiment: f64,
    pub likes: u32,
    pub retweets: u32,
    pub author_followers: u32,
}

/// Social sentiment result
#[derive(Debug, Clone)]
pub struct SocialSentiment {
    pub platform: Platform,
    pub ticker: String,
    pub sentiment_score: f64,  // -1 to 1
    pub mention_count: u32,
    pub engagement: u32,
    pub trending: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Platform {
    Reddit,
    Twitter,
    StockTwits,
    Combined,
}

/// Social media error
#[derive(Debug, thiserror::Error)]
pub enum SocialError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limit")]
    RateLimit,
    #[error("Auth error")]
    AuthError,
}

/// Trending mentions
#[derive(Debug, Clone)]
pub struct TrendingMentions {
    pub ticker: String,
    pub mention_count_24h: u32,
    pub sentiment_change: f64,
    pub volume_surge: bool,
}
