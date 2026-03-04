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

        let weighted_score = (reddit.sentiment_score * reddit.mention_count as f64
            + twitter.sentiment_score * twitter.mention_count as f64)
            / total_mentions as f64;

        Ok(SocialSentiment {
            platform: Platform::Combined,
            ticker: ticker.to_string(),
            sentiment_score: weighted_score,
            mention_count: total_mentions,
            engagement: reddit.engagement + twitter.engagement,
            trending: reddit.trending || twitter.trending,
        })
    }

    /// Get trending tickers from Reddit.
    /// Scans r/wallstreetbets and r/stocks for frequently mentioned tickers.
    /// Returns empty vec with warning if Reddit is unreachable.
    pub async fn trending_tickers(&self) -> Result<Vec<String>, SocialError> {
        let posts = self.reddit.fetch_trending_posts().await?;

        // Extract ticker symbols ($AAPL, $TSLA, etc.) from post titles
        let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let ticker_pattern = regex::Regex::new(r"\$([A-Z]{1,5})\b").ok();

        for post in &posts {
            if let Some(ref re) = ticker_pattern {
                for cap in re.captures_iter(&post.title) {
                    if let Some(m) = cap.get(1) {
                        *counts.entry(m.as_str().to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Sort by mention count descending, take top 20
        let mut sorted: Vec<(String, u32)> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(sorted.into_iter().take(20).map(|(t, _)| t).collect())
    }
}

impl Default for SocialCollector {
    fn default() -> Self {
        Self::new()
    }
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

    /// Fetch posts mentioning a ticker from Reddit JSON API.
    /// Uses public `.json` endpoints (no OAuth required for read).
    pub async fn fetch_posts(&self, ticker: &str) -> Result<Vec<RedditPost>, SocialError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| SocialError::ApiError(e.to_string()))?;

        let subreddits = ["wallstreetbets", "stocks", "investing"];
        let mut posts = Vec::new();

        for sub in &subreddits {
            let url = format!(
                "https://www.reddit.com/r/{}/search.json?q={}&restrict_sr=1&sort=new&limit=25",
                sub, ticker
            );
            match client
                .get(&url)
                .header("User-Agent", &self.user_agent)
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(children) = json["data"]["children"].as_array() {
                            for child in children {
                                let d = &child["data"];
                                let title = d["title"].as_str().unwrap_or_default().to_string();
                                let ups = d["ups"].as_u64().unwrap_or(0) as u32;
                                let comments = d["num_comments"].as_u64().unwrap_or(0) as u32;
                                let upvote_ratio = d["upvote_ratio"].as_f64().unwrap_or(0.5);
                                // Simple sentiment: upvote_ratio > 0.7 = positive, < 0.4 = negative
                                let sentiment = (upvote_ratio - 0.5) * 2.0;
                                posts.push(RedditPost {
                                    title,
                                    sentiment_score: sentiment,
                                    upvotes: ups,
                                    comment_count: comments,
                                    subreddit: sub.to_string(),
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Reddit fetch failed for r/{}: {}", sub, e);
                }
            }
        }

        if posts.is_empty() {
            tracing::warn!("No Reddit posts found for ticker {}", ticker);
        }

        Ok(posts)
    }

    /// Fetch trending posts from popular finance subreddits (for trending_tickers)
    pub async fn fetch_trending_posts(&self) -> Result<Vec<RedditPost>, SocialError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| SocialError::ApiError(e.to_string()))?;

        let mut posts = Vec::new();
        let url = "https://www.reddit.com/r/wallstreetbets/hot.json?limit=50";
        match client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(children) = json["data"]["children"].as_array() {
                        for child in children {
                            let d = &child["data"];
                            posts.push(RedditPost {
                                title: d["title"].as_str().unwrap_or_default().to_string(),
                                sentiment_score: 0.0,
                                upvotes: d["ups"].as_u64().unwrap_or(0) as u32,
                                comment_count: d["num_comments"].as_u64().unwrap_or(0) as u32,
                                subreddit: "wallstreetbets".to_string(),
                            });
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Reddit trending fetch failed: {}", e);
            }
        }

        Ok(posts)
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

    /// Fetch tweets for a ticker. Requires `TWITTER_BEARER_TOKEN` env var.
    /// Returns empty vec without the token (no fake data).
    pub async fn fetch_tweets(&self, ticker: &str) -> Result<Vec<Tweet>, SocialError> {
        let bearer = match &self.bearer_token {
            Some(token) if !token.is_empty() => token.clone(),
            _ => {
                tracing::debug!("Twitter API disabled — TWITTER_BEARER_TOKEN not set");
                return Ok(vec![]);
            }
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| SocialError::ApiError(e.to_string()))?;

        let url = format!(
            "https://api.twitter.com/2/tweets/search/recent?query=%24{}&max_results=25&tweet.fields=public_metrics",
            ticker
        );
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", bearer))
            .send()
            .await
            .map_err(|e| SocialError::ApiError(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(SocialError::AuthError);
        }
        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(SocialError::RateLimit);
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SocialError::ApiError(e.to_string()))?;

        let mut tweets = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for tweet in data {
                let text = tweet["text"].as_str().unwrap_or_default().to_string();
                let metrics = &tweet["public_metrics"];
                let likes = metrics["like_count"].as_u64().unwrap_or(0) as u32;
                let retweets = metrics["retweet_count"].as_u64().unwrap_or(0) as u32;
                // Simple sentiment from engagement ratio
                let total = (likes + retweets).max(1) as f64;
                let sentiment = ((likes as f64 / total) - 0.5).clamp(-1.0, 1.0);
                tweets.push(Tweet {
                    text,
                    sentiment,
                    likes,
                    retweets,
                    author_followers: 0, // Requires user expansion
                });
            }
        }

        Ok(tweets)
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
    pub sentiment_score: f64, // -1 to 1
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
