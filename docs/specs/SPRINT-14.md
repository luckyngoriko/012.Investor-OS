# Sprint 14: Alternative Data

> **Status:** IMPLEMENTED  
> **Duration:** 2 weeks  
> **Goal:** Integrate non-traditional data sources  
> **Depends on:** Sprint 10 (AI APIs), Sprint 12 (Streaming)  
> **Completed:** 2026-02-08

---

## Overview

Add alternative data: satellite imagery, web scraping, social media, credit card data for alpha generation.

---

## Goals

- [ ] News NLP pipeline
- [ ] Reddit/Twitter sentiment
- [ ] Google Trends integration
- [ ] Web scraping (job postings)
- [ ] Satellite data integration
- [ ] Options flow analysis
- [ ] Insider sentiment NLP

---

## Technical Tasks

### 1. News NLP Pipeline
```rust
src/collectors/news/
├── mod.rs
├── rss_feeds.rs
├── scraper.rs
├── nlp_processor.rs
└── sentiment.rs
```
- [ ] RSS aggregation (100+ sources)
- [ ] Article deduplication
- [ ] Named entity recognition (NER)
- [ ] Ticker extraction
- [ ] Sentiment scoring
- [ ] Impact assessment

### 2. Social Media Sentiment
```rust
src/collectors/social/
├── mod.rs
├── reddit.rs           // WallStreetBets, investing
├── twitter.rs          // X API
├── stocktwits.rs       // Existing from Sprint 2
└── aggregator.rs
```

### 3. Google Trends
```rust
src/collectors/trends/
├── mod.rs
└── google_trends.rs    // pytrends integration
```
- [ ] Search volume by ticker
- [ ] Rising queries
- [ ] Regional interest
- [ ] Related topics

### 4. Web Scraping
```rust
src/collectors/scraper/
├── mod.rs
├── jobs.rs             // LinkedIn, Indeed
├── patents.rs          // USPTO
├── earnings_dates.rs   // Earnings calendars
└── sec_filings.rs      // EDGAR enhancement
```

### 5. Options Flow
```rust
src/collectors/options_flow/
├── mod.rs
├── unusual_volume.rs
├── whale_tracking.rs
└── sweep_detection.rs
```
- [ ] Unusual volume alerts
- [ ] Block trades
- [ ] Sweep detection
- [ ] Put/Call ratios

### 6. Alternative Data API
```rust
pub struct AlternativeDataEngine {
    news: NewsCollector,
    social: SocialCollector,
    trends: TrendsCollector,
    scraper: WebScraper,
    options: OptionsFlowCollector,
}

impl AlternativeDataEngine {
    pub async fn get_composite_sentiment(&self, ticker: &str) -> SentimentScore {
        let news_score = self.news.sentiment(ticker).await;
        let social_score = self.social.sentiment(ticker).await;
        let trends_score = self.trends.interest(ticker).await;
        
        // Weighted composite
        news_score * 0.4 + social_score * 0.4 + trends_score * 0.2
    }
}
```

---

## Data Sources

| Source | Type | Frequency | Cost |
|--------|------|-----------|------|
| NewsAPI | News | Real-time | $50/mo |
| Reddit API | Social | Real-time | Free |
| Twitter/X API | Social | Real-time | $100/mo |
| Google Trends | Search | Daily | Free |
| Quiver Quant | Options | Real-time | $300/mo |
| Orbital Insight | Satellite | Weekly | $$$ |

---

## NLP Pipeline

```
Raw Text → Preprocessing → NER → Ticker Extraction → Sentiment → Signal
                ↓
        Tokenization
                ↓
        Stopword Removal
                ↓
        Lemmatization
```

---

## Success Criteria

- [ ] 100+ news sources aggregated
- [ ] < 5 min latency news-to-signal
- [ ] Social sentiment real-time
- [ ] 80%+ ticker extraction accuracy
- [ ] Options flow alerts working

---

## Dependencies

- Sprint 2: StockTwits (sentiment pattern)
- Sprint 5: RAG (for news context)
- Sprint 10: AI APIs (for NLP)
- Sprint 12: Streaming (real-time processing)

---

## Golden Path Tests

```rust
#[test]
fn test_news_sentiment_extraction() { ... }

#[test]
fn test_reddit_wsb_sentiment() { ... }

#[test]
fn test_twitter_ticker_extraction() { ... }

#[test]
fn test_google_trends_correlation() { ... }

#[test]
fn test_options_unusual_volume() { ... }

#[test]
fn test_composite_sentiment_calculation() { ... }

#[test]
fn test_news_latency() { ... }

#[test]
fn test_scraper_job_postings() { ... }
```

---

**Next:** Sprint 15 (Social Trading & Mobile)
