//! Global Market Clock

use chrono::{DateTime, Utc, Duration};

/// Global market clock
#[derive(Debug)]
pub struct GlobalMarketClock {
    current_time: DateTime<Utc>,
}

impl GlobalMarketClock {
    pub fn new() -> Self {
        Self {
            current_time: Utc::now(),
        }
    }
    
    pub fn tick(&mut self, now: DateTime<Utc>) {
        self.current_time = now;
    }
    
    pub fn now(&self) -> DateTime<Utc> {
        self.current_time
    }
}

impl Default for GlobalMarketClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Market time utilities
pub struct MarketTime;

impl MarketTime {
    /// Convert UTC to market local time
    pub fn to_local(utc: DateTime<Utc>, offset: chrono::FixedOffset) -> DateTime<chrono::FixedOffset> {
        utc.with_timezone(&offset)
    }
    
    /// Convert market local time to UTC
    pub fn to_utc(local: DateTime<chrono::FixedOffset>) -> DateTime<Utc> {
        local.with_timezone(&Utc)
    }
    
    /// Check if business day
    pub fn is_business_day(date: chrono::NaiveDate) -> bool {
        use chrono::{Weekday, Datelike};
        !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }
}

/// Trading session
#[derive(Debug, Clone)]
pub struct TradingSession {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub market: String,
    pub session_type: String,
}

impl TradingSession {
    pub fn duration(&self) -> Duration {
        self.end - self.start
    }
    
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        now >= self.start && now <= self.end
    }
}
