//! Trading Hours Management

use chrono::{DateTime, Utc, NaiveTime, Datelike, Weekday, Duration};
use serde::{Deserialize, Serialize};

/// Trading hours configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingHours {
    pub regular: Vec<TimeRange>,
    pub pre_market: Option<TimeRange>,
    pub after_hours: Option<TimeRange>,
    pub timezone: String,
}

/// Time range for trading sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: NaiveTime,
    pub end: NaiveTime,
}

/// Market session type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketSession {
    Closed,
    PreMarket,
    Regular,
    AfterHours,
}

impl TradingHours {
    /// Check if market is open at given datetime
    pub fn is_open(&self, datetime: DateTime<Utc>, holidays: Option<&super::HolidayCalendar>) -> bool {
        // Check holidays
        if let Some(holiday_cal) = holidays {
            if holiday_cal.is_holiday(datetime) {
                return false;
            }
        }
        
        // Check weekend
        let weekday = datetime.weekday();
        if matches!(weekday, Weekday::Sat | Weekday::Sun) {
            return false;
        }
        
        // Get local time
        let local_time = datetime.time();
        
        // Check regular hours
        for range in &self.regular {
            if local_time >= range.start && local_time <= range.end {
                return true;
            }
        }
        
        false
    }
    
    /// Get current market session
    pub fn get_session(&self, datetime: DateTime<Utc>) -> MarketSession {
        let local_time = datetime.time();
        
        // Check regular hours
        for range in &self.regular {
            if local_time >= range.start && local_time <= range.end {
                return MarketSession::Regular;
            }
        }
        
        // Check pre-market
        if let Some(ref pre) = self.pre_market {
            if local_time >= pre.start && local_time <= pre.end {
                return MarketSession::PreMarket;
            }
        }
        
        // Check after-hours
        if let Some(ref after) = self.after_hours {
            if local_time >= after.start && local_time <= after.end {
                return MarketSession::AfterHours;
            }
        }
        
        MarketSession::Closed
    }
    
    /// Time until market opens
    pub fn time_until_open(&self, datetime: DateTime<Utc>) -> Duration {
        if self.is_open(datetime, None) {
            return Duration::zero();
        }
        
        let local_time = datetime.time();
        
        // Find next opening
        for range in &self.regular {
            if local_time < range.start {
                let diff = range.start.signed_duration_since(local_time);
                return Duration::seconds(diff.num_seconds());
            }
        }
        
        // Market opens next day
        Duration::hours(24) - Duration::seconds(
            local_time.signed_duration_since(NaiveTime::from_hms_opt(0, 0, 0).unwrap()).num_seconds()
        ) + Duration::seconds(self.regular[0].start.signed_duration_since(NaiveTime::from_hms_opt(0, 0, 0).unwrap()).num_seconds())
    }
    
    /// Time until market closes
    pub fn time_until_close(&self, datetime: DateTime<Utc>) -> Duration {
        if !self.is_open(datetime, None) {
            return Duration::zero();
        }
        
        let local_time = datetime.time();
        
        for range in &self.regular {
            if local_time >= range.start && local_time <= range.end {
                let diff = range.end.signed_duration_since(local_time);
                return Duration::seconds(diff.num_seconds());
            }
        }
        
        Duration::zero()
    }
    
    // Factory methods for common markets
    
    /// US Equity markets (NYSE, NASDAQ)
    pub fn us_equity() -> Self {
        Self {
            regular: vec![TimeRange {
                start: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
                end: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            }],
            pre_market: Some(TimeRange {
                start: NaiveTime::from_hms_opt(4, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            }),
            after_hours: Some(TimeRange {
                start: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            }),
            timezone: "America/New_York".to_string(),
        }
    }
    
    /// London Stock Exchange
    pub fn lse() -> Self {
        Self {
            regular: vec![TimeRange {
                start: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(16, 30, 0).unwrap(),
            }],
            pre_market: None,
            after_hours: None,
            timezone: "Europe/London".to_string(),
        }
    }
    
    /// European equity markets (Xetra, Euronext)
    pub fn european_equity() -> Self {
        Self {
            regular: vec![TimeRange {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(17, 30, 0).unwrap(),
            }],
            pre_market: Some(TimeRange {
                start: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            }),
            after_hours: None,
            timezone: "Europe/Berlin".to_string(),
        }
    }
    
    /// Tokyo Stock Exchange
    pub fn tse() -> Self {
        Self {
            regular: vec![
                TimeRange {
                    start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(11, 30, 0).unwrap(),
                },
                TimeRange {
                    start: NaiveTime::from_hms_opt(12, 30, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                },
            ],
            pre_market: Some(TimeRange {
                start: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            }),
            after_hours: None,
            timezone: "Asia/Tokyo".to_string(),
        }
    }
    
    /// Hong Kong Stock Exchange
    pub fn hkex() -> Self {
        Self {
            regular: vec![
                TimeRange {
                    start: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                },
                TimeRange {
                    start: NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
                    end: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
                },
            ],
            pre_market: Some(TimeRange {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            }),
            after_hours: None,
            timezone: "Asia/Hong_Kong".to_string(),
        }
    }
    
    /// Asian equity markets (SGX, etc.)
    pub fn asian_equity() -> Self {
        Self {
            regular: vec![TimeRange {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            }],
            pre_market: Some(TimeRange {
                start: NaiveTime::from_hms_opt(8, 30, 0).unwrap(),
                end: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            }),
            after_hours: None,
            timezone: "Asia/Singapore".to_string(),
        }
    }
    
    /// Australian Securities Exchange
    pub fn asx() -> Self {
        Self {
            regular: vec![TimeRange {
                start: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            }],
            pre_market: Some(TimeRange {
                start: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            }),
            after_hours: None,
            timezone: "Australia/Sydney".to_string(),
        }
    }
    
    /// National Stock Exchange of India
    pub fn nse() -> Self {
        Self {
            regular: vec![TimeRange {
                start: NaiveTime::from_hms_opt(9, 15, 0).unwrap(),
                end: NaiveTime::from_hms_opt(15, 30, 0).unwrap(),
            }],
            pre_market: Some(TimeRange {
                start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(9, 15, 0).unwrap(),
            }),
            after_hours: None,
            timezone: "Asia/Kolkata".to_string(),
        }
    }
    
    /// B3 Brazil
    pub fn b3() -> Self {
        Self {
            regular: vec![TimeRange {
                start: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                end: NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            }],
            pre_market: None,
            after_hours: None,
            timezone: "America/Sao_Paulo".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_us_equity_hours() {
        let hours = TradingHours::us_equity();
        
        // Monday at 10:00 AM NY
        let dt = DateTime::parse_from_rfc3339("2024-01-15T15:00:00Z").unwrap().with_timezone(&Utc);
        assert!(hours.is_open(dt, None));
        
        // Monday at 5:00 PM NY
        let dt = DateTime::parse_from_rfc3339("2024-01-15T22:00:00Z").unwrap().with_timezone(&Utc);
        assert!(!hours.is_open(dt, None));
    }

    #[test]
    fn test_tse_hours() {
        let hours = TradingHours::tse();
        
        assert_eq!(hours.regular.len(), 2); // Two sessions
        assert!(hours.pre_market.is_some());
    }
}
