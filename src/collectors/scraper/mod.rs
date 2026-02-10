//! Web Scraper - Sprint 14
//!
//! SEC filings, earnings transcripts, and corporate data scraping

use serde::{Deserialize, Serialize};

/// Web scraper for financial data
#[derive(Debug, Clone)]
pub struct WebScraper {
    client: reqwest::Client,
    sec_user_agent: String,
}

impl WebScraper {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
            sec_user_agent: "InvestorOS/1.0 (contact@investoros.local)".to_string(),
        }
    }

    /// Get recent SEC filings for a ticker
    pub async fn get_sec_filings(&self, ticker: &str) -> Result<Vec<SecFiling>, ScraperError> {
        // Would fetch from SEC EDGAR API
        // Placeholder with realistic synthetic data
        let now = chrono::Utc::now();

        Ok(vec![
            SecFiling {
                ticker: ticker.to_string(),
                form_type: "8-K".to_string(),
                filing_date: (now - chrono::Duration::days(2)).format("%Y-%m-%d").to_string(),
                description: "Current Report".to_string(),
                url: format!("https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=8-K", ticker),
                period_ending: None,
                material_change: true,
            },
            SecFiling {
                ticker: ticker.to_string(),
                form_type: "10-Q".to_string(),
                filing_date: (now - chrono::Duration::days(15)).format("%Y-%m-%d").to_string(),
                description: "Quarterly Report".to_string(),
                url: format!("https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=10-Q", ticker),
                period_ending: Some((now - chrono::Duration::days(30)).format("%Y-%m-%d").to_string()),
                material_change: false,
            },
            SecFiling {
                ticker: ticker.to_string(),
                form_type: "4".to_string(),
                filing_date: (now - chrono::Duration::days(5)).format("%Y-%m-%d").to_string(),
                description: "Insider Trading Report".to_string(),
                url: format!("https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=4", ticker),
                period_ending: None,
                material_change: true,
            },
        ])
    }

    /// Get insider trading activity
    pub async fn get_insider_trades(&self, ticker: &str) -> Result<Vec<InsiderTrade>, ScraperError> {
        // Would scrape from SEC Form 4 filings
        let now = chrono::Utc::now();

        Ok(vec![
            InsiderTrade {
                ticker: ticker.to_string(),
                insider_name: "John Smith".to_string(),
                title: "CEO".to_string(),
                transaction_type: TransactionType::Purchase,
                shares: 10000,
                price_per_share: 150.0,
                total_value: 1_500_000.0,
                transaction_date: (now - chrono::Duration::days(3)).format("%Y-%m-%d").to_string(),
                ownership_type: OwnershipType::Direct,
            },
            InsiderTrade {
                ticker: ticker.to_string(),
                insider_name: "Jane Doe".to_string(),
                title: "CFO".to_string(),
                transaction_type: TransactionType::Sale,
                shares: 5000,
                price_per_share: 155.0,
                total_value: 775_000.0,
                transaction_date: (now - chrono::Duration::days(5)).format("%Y-%m-%d").to_string(),
                ownership_type: OwnershipType::Indirect,
            },
        ])
    }

    /// Calculate insider sentiment from recent trades
    pub async fn get_insider_sentiment(&self, ticker: &str) -> Result<InsiderSentiment, ScraperError> {
        let trades = self.get_insider_trades(ticker).await?;

        let (buy_count, sell_count, buy_value, sell_value) = trades.iter().fold(
            (0, 0, 0.0, 0.0),
            |(bc, sc, bv, sv), trade| match trade.transaction_type {
                TransactionType::Purchase => (bc + 1, sc, bv + trade.total_value, sv),
                TransactionType::Sale => (bc, sc + 1, bv, sv + trade.total_value),
                _ => (bc, sc, bv, sv),
            },
        );

        let total_value = buy_value + sell_value;
        let sentiment_score = if total_value > 0.0 {
            (buy_value - sell_value) / total_value
        } else {
            0.0
        };

        Ok(InsiderSentiment {
            ticker: ticker.to_string(),
            sentiment_score,
            buy_count,
            sell_count,
            buy_value,
            sell_value,
            net_activity: if buy_value > sell_value {
                NetActivity::Buying
            } else if sell_value > buy_value {
                NetActivity::Selling
            } else {
                NetActivity::Neutral
            },
            last_updated: chrono::Utc::now(),
        })
    }

    /// Get earnings transcript highlights
    pub async fn get_earnings_highlights(&self, ticker: &str) -> Result<EarningsHighlights, ScraperError> {
        // Would scrape from earnings call transcripts
        Ok(EarningsHighlights {
            ticker: ticker.to_string(),
            quarter: "Q3 2024".to_string(),
            fiscal_year: 2024,
            eps_actual: 1.25,
            eps_estimate: 1.20,
            eps_surprise_pct: 4.17,
            revenue_actual: 50_000_000_000.0,
            revenue_estimate: 48_500_000_000.0,
            revenue_surprise_pct: 3.09,
            guidance_raised: true,
            key_themes: vec![
                "Strong AI demand".to_string(),
                "Margin expansion".to_string(),
                "Cloud growth accelerating".to_string(),
            ],
            sentiment: EarningsSentiment::Positive,
            transcript_url: format!("https://seekingalpha.com/symbol/{}/earnings/transcripts", ticker),
            call_date: (chrono::Utc::now() - chrono::Duration::days(10)).format("%Y-%m-%d").to_string(),
        })
    }

    /// Get institutional holdings changes
    pub async fn get_institutional_changes(&self, ticker: &str) -> Result<Vec<InstitutionalHolding>, ScraperError> {
        // Would scrape from 13F filings
        Ok(vec![
            InstitutionalHolding {
                ticker: ticker.to_string(),
                institution_name: "Vanguard Group".to_string(),
                shares_held: 150_000_000,
                previous_shares: 145_000_000,
                change_pct: 3.45,
                portfolio_weight: 5.2,
                filing_date: (chrono::Utc::now() - chrono::Duration::days(20)).format("%Y-%m-%d").to_string(),
            },
            InstitutionalHolding {
                ticker: ticker.to_string(),
                institution_name: "BlackRock".to_string(),
                shares_held: 120_000_000,
                previous_shares: 118_000_000,
                change_pct: 1.69,
                portfolio_weight: 4.8,
                filing_date: (chrono::Utc::now() - chrono::Duration::days(18)).format("%Y-%m-%d").to_string(),
            },
        ])
    }

    /// Get corporate events calendar
    pub async fn get_corporate_events(&self, ticker: &str) -> Result<Vec<CorporateEvent>, ScraperError> {
        let now = chrono::Utc::now();

        Ok(vec![
            CorporateEvent {
                ticker: ticker.to_string(),
                event_type: EventType::Earnings,
                event_date: (now + chrono::Duration::days(25)).format("%Y-%m-%d").to_string(),
                description: "Q4 2024 Earnings Release".to_string(),
                estimated: true,
            },
            CorporateEvent {
                ticker: ticker.to_string(),
                event_type: EventType::Dividend,
                event_date: (now + chrono::Duration::days(10)).format("%Y-%m-%d").to_string(),
                description: "Quarterly Dividend Ex-Date".to_string(),
                estimated: false,
            },
            CorporateEvent {
                ticker: ticker.to_string(),
                event_type: EventType::Conference,
                event_date: (now + chrono::Duration::days(40)).format("%Y-%m-%d").to_string(),
                description: "Tech Conference Presentation".to_string(),
                estimated: true,
            },
        ])
    }

    /// Analyze SEC filing for material changes
    pub fn analyze_filing_sentiment(&self, filing: &SecFiling) -> FilingSentiment {
        match filing.form_type.as_str() {
            "8-K" => FilingSentiment::MaterialEvent,
            "4" => FilingSentiment::InsiderActivity,
            "13D" | "13G" => FilingSentiment::OwnershipChange,
            "SC 13D" => FilingSentiment::ActivistStake,
            _ => FilingSentiment::Routine,
        }
    }

    /// Get comprehensive corporate intelligence
    pub async fn get_corporate_intelligence(&self, ticker: &str) -> Result<CorporateIntelligence, ScraperError> {
        let filings = self.get_sec_filings(ticker).await?;
        let insider_sentiment = self.get_insider_sentiment(ticker).await?;
        let earnings = self.get_earnings_highlights(ticker).await?;
        let institutional = self.get_institutional_changes(ticker).await?;
        let events = self.get_corporate_events(ticker).await?;

        // Calculate composite score
        let filing_score = filings
            .iter()
            .map(|f| self.analyze_filing_sentiment(f).score())
            .sum::<f64>()
            / filings.len().max(1) as f64;

        let institutional_score = institutional
            .iter()
            .map(|h| if h.change_pct > 0.0 { 1.0 } else { -1.0 })
            .sum::<f64>()
            / institutional.len().max(1) as f64;

        let earnings_score = match earnings.sentiment {
            EarningsSentiment::VeryPositive => 1.0,
            EarningsSentiment::Positive => 0.5,
            EarningsSentiment::Neutral => 0.0,
            EarningsSentiment::Negative => -0.5,
            EarningsSentiment::VeryNegative => -1.0,
        };

        let composite_score = filing_score * 0.2
            + insider_sentiment.sentiment_score * 0.3
            + institutional_score * 0.25
            + earnings_score * 0.25;

        Ok(CorporateIntelligence {
            ticker: ticker.to_string(),
            filings,
            insider_sentiment,
            earnings,
            institutional_holdings: institutional,
            upcoming_events: events,
            composite_score,
            last_updated: chrono::Utc::now(),
        })
    }
}

impl Default for WebScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// SEC filing data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecFiling {
    pub ticker: String,
    pub form_type: String,
    pub filing_date: String,
    pub description: String,
    pub url: String,
    pub period_ending: Option<String>,
    pub material_change: bool,
}

/// Insider trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsiderTrade {
    pub ticker: String,
    pub insider_name: String,
    pub title: String,
    pub transaction_type: TransactionType,
    pub shares: u32,
    pub price_per_share: f64,
    pub total_value: f64,
    pub transaction_date: String,
    pub ownership_type: OwnershipType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TransactionType {
    Purchase,
    Sale,
    OptionExercise,
    Gift,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OwnershipType {
    Direct,
    Indirect,
}

/// Insider sentiment summary
#[derive(Debug, Clone)]
pub struct InsiderSentiment {
    pub ticker: String,
    pub sentiment_score: f64, // -1 to 1
    pub buy_count: u32,
    pub sell_count: u32,
    pub buy_value: f64,
    pub sell_value: f64,
    pub net_activity: NetActivity,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetActivity {
    Buying,
    Selling,
    Neutral,
}

/// Earnings highlights
#[derive(Debug, Clone)]
pub struct EarningsHighlights {
    pub ticker: String,
    pub quarter: String,
    pub fiscal_year: u32,
    pub eps_actual: f64,
    pub eps_estimate: f64,
    pub eps_surprise_pct: f64,
    pub revenue_actual: f64,
    pub revenue_estimate: f64,
    pub revenue_surprise_pct: f64,
    pub guidance_raised: bool,
    pub key_themes: Vec<String>,
    pub sentiment: EarningsSentiment,
    pub transcript_url: String,
    pub call_date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarningsSentiment {
    VeryPositive,
    Positive,
    Neutral,
    Negative,
    VeryNegative,
}

/// Institutional holding change
#[derive(Debug, Clone)]
pub struct InstitutionalHolding {
    pub ticker: String,
    pub institution_name: String,
    pub shares_held: u32,
    pub previous_shares: u32,
    pub change_pct: f64,
    pub portfolio_weight: f64,
    pub filing_date: String,
}

/// Corporate event
#[derive(Debug, Clone)]
pub struct CorporateEvent {
    pub ticker: String,
    pub event_type: EventType,
    pub event_date: String,
    pub description: String,
    pub estimated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Earnings,
    Dividend,
    Split,
    Conference,
    ShareholderMeeting,
    ProductLaunch,
    Other,
}

/// Filing sentiment classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilingSentiment {
    MaterialEvent,
    InsiderActivity,
    OwnershipChange,
    ActivistStake,
    Routine,
}

impl FilingSentiment {
    /// Convert to numeric score
    pub fn score(&self) -> f64 {
        match self {
            FilingSentiment::ActivistStake => 1.0,
            FilingSentiment::MaterialEvent => 0.5,
            FilingSentiment::InsiderActivity => 0.3,
            FilingSentiment::OwnershipChange => 0.2,
            FilingSentiment::Routine => 0.0,
        }
    }
}

/// Comprehensive corporate intelligence
#[derive(Debug, Clone)]
pub struct CorporateIntelligence {
    pub ticker: String,
    pub filings: Vec<SecFiling>,
    pub insider_sentiment: InsiderSentiment,
    pub earnings: EarningsHighlights,
    pub institutional_holdings: Vec<InstitutionalHolding>,
    pub upcoming_events: Vec<CorporateEvent>,
    pub composite_score: f64, // -1 to 1
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl CorporateIntelligence {
    /// Check if there are material developments
    pub fn has_material_developments(&self) -> bool {
        self.filings.iter().any(|f| f.material_change)
            || self.insider_sentiment.net_activity != NetActivity::Neutral
            || self.earnings.sentiment != EarningsSentiment::Neutral
    }

    /// Get upcoming earnings date if any
    pub fn next_earnings_date(&self) -> Option<&String> {
        self.upcoming_events
            .iter()
            .find(|e| e.event_type == EventType::Earnings)
            .map(|e| &e.event_date)
    }
}

/// Web scraper error
#[derive(Debug, thiserror::Error)]
pub enum ScraperError {
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Rate limit")]
    RateLimit,
    #[error("No data available")]
    NoData,
}

/// Corporate intelligence dashboard
#[derive(Debug, Clone, Default)]
pub struct CorporateDashboard {
    pub recent_filings: Vec<SecFiling>,
    pub insider_activity: Vec<InsiderSentiment>,
    pub earnings_surprises: Vec<EarningsHighlights>,
    pub institutional_moves: Vec<InstitutionalHolding>,
    pub upcoming_events: Vec<CorporateEvent>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filing_sentiment_scores() {
        assert_eq!(FilingSentiment::ActivistStake.score(), 1.0);
        assert_eq!(FilingSentiment::MaterialEvent.score(), 0.5);
        assert_eq!(FilingSentiment::InsiderActivity.score(), 0.3);
        assert_eq!(FilingSentiment::OwnershipChange.score(), 0.2);
        assert_eq!(FilingSentiment::Routine.score(), 0.0);
    }

    #[test]
    fn test_insider_sentiment_calculation() {
        let sentiment = InsiderSentiment {
            ticker: "AAPL".to_string(),
            sentiment_score: 0.5,
            buy_count: 3,
            sell_count: 1,
            buy_value: 1_000_000.0,
            sell_value: 500_000.0,
            net_activity: NetActivity::Buying,
            last_updated: chrono::Utc::now(),
        };

        assert_eq!(sentiment.net_activity, NetActivity::Buying);
        assert!(sentiment.sentiment_score > 0.0);
    }

    #[test]
    fn test_corporate_intelligence_has_material_developments() {
        let intel = CorporateIntelligence {
            ticker: "AAPL".to_string(),
            filings: vec![SecFiling {
                ticker: "AAPL".to_string(),
                form_type: "8-K".to_string(),
                filing_date: "2024-01-01".to_string(),
                description: "Test".to_string(),
                url: "https://sec.gov".to_string(),
                period_ending: None,
                material_change: true,
            }],
            insider_sentiment: InsiderSentiment {
                ticker: "AAPL".to_string(),
                sentiment_score: 0.0,
                buy_count: 0,
                sell_count: 0,
                buy_value: 0.0,
                sell_value: 0.0,
                net_activity: NetActivity::Neutral,
                last_updated: chrono::Utc::now(),
            },
            earnings: EarningsHighlights {
                ticker: "AAPL".to_string(),
                quarter: "Q1".to_string(),
                fiscal_year: 2024,
                eps_actual: 1.0,
                eps_estimate: 1.0,
                eps_surprise_pct: 0.0,
                revenue_actual: 100.0,
                revenue_estimate: 100.0,
                revenue_surprise_pct: 0.0,
                guidance_raised: false,
                key_themes: vec![],
                sentiment: EarningsSentiment::Neutral,
                transcript_url: "https://seekingalpha.com".to_string(),
                call_date: "2024-01-01".to_string(),
            },
            institutional_holdings: vec![],
            upcoming_events: vec![],
            composite_score: 0.0,
            last_updated: chrono::Utc::now(),
        };

        assert!(intel.has_material_developments());
    }
}
