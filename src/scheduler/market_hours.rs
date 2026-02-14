//! Market Hours and Trading Sessions
//!
//! Manages trading hours for global exchanges with 24/7 coverage

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Timelike, Utc, Weekday};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Trading schedule for multiple exchanges
#[derive(Debug, Clone)]
pub struct TradingSchedule {
    exchanges: HashMap<String, ExchangeSchedule>,
}

/// Exchange trading schedule
#[derive(Debug, Clone)]
pub struct ExchangeSchedule {
    pub name: String,
    pub timezone: String,
    pub regular_hours: TradingHours,
    pub pre_market: Option<TradingHours>,
    pub after_hours: Option<TradingHours>,
    pub trading_days: Vec<Weekday>,
    pub half_days: Vec<NaiveDate>,
}

/// Trading hours within a day
#[derive(Debug, Clone)]
pub struct TradingHours {
    pub open: NaiveTime,
    pub close: NaiveTime,
}

/// Market session info
#[derive(Debug, Clone)]
pub struct MarketSession {
    exchange: String,
    session_type: String,
    open_time: DateTime<Utc>,
    close_time: DateTime<Utc>,
    liquidity_score: u32,
}

impl MarketSession {
    /// Get exchange name
    pub fn exchange_name(&self) -> String {
        self.exchange.clone()
    }

    /// Get session type
    pub fn session_type(&self) -> String {
        self.session_type.clone()
    }

    /// Get liquidity score
    pub fn liquidity_score(&self) -> u32 {
        self.liquidity_score
    }

    /// Get time until close
    pub fn time_until_close(&self) -> Duration {
        let now = Utc::now();
        if self.close_time > now {
            self.close_time - now
        } else {
            Duration::zero()
        }
    }

    /// Check if supports symbol
    pub fn supports_symbol(&self, _symbol: &str) -> bool {
        // Simplified - would check exchange capabilities
        true
    }
}

impl TradingSchedule {
    /// Create new schedule
    pub fn new() -> Self {
        Self {
            exchanges: HashMap::new(),
        }
    }

    /// Add exchange schedule
    pub fn add_exchange(&mut self, schedule: ExchangeSchedule) {
        self.exchanges.insert(schedule.name.clone(), schedule);
    }

    /// Check if any market is open
    pub fn is_any_market_open(&self) -> bool {
        let now = Utc::now();
        
        for schedule in self.exchanges.values() {
            if schedule.is_open_at(now) {
                return true;
            }
        }
        
        false
    }

    /// Get currently active sessions
    pub fn get_active_sessions(&self) -> Vec<MarketSession> {
        let now = Utc::now();
        let mut sessions = Vec::new();

        for schedule in self.exchanges.values() {
            if let Some(session) = schedule.get_session_at(now) {
                sessions.push(session);
            }
        }

        // Sort by liquidity score descending
        sessions.sort_by(|a, b| b.liquidity_score.cmp(&a.liquidity_score));
        sessions
    }

    /// Get next market open time
    pub fn next_market_open(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        let mut next_open: Option<DateTime<Utc>> = None;

        for schedule in self.exchanges.values() {
            if let Some(open_time) = schedule.next_open_after(now) {
                if next_open.map_or(true, |current| open_time < current) {
                    next_open = Some(open_time);
                }
            }
        }

        next_open
    }

    /// Get next market close time
    pub fn next_market_close(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        let mut next_close: Option<DateTime<Utc>> = None;

        for schedule in self.exchanges.values() {
            if let Some(close_time) = schedule.next_close_after(now) {
                if next_close.map_or(true, |current| close_time < current) {
                    next_close = Some(close_time);
                }
            }
        }

        next_close
    }

    /// Calculate 24h coverage percentage
    pub fn calculate_coverage_percentage(&self) -> Decimal {
        // Count hours covered by open markets in a 24h period
        let mut covered_minutes = 0;
        
        // Check each 15-minute block in a day
        for block in 0..96 { // 24 * 4 = 96 fifteen-minute blocks
            let minutes = block * 15;
            let hour = minutes / 60;
            let minute = minutes % 60;
            
            let check_time = Utc::now()
                .with_hour(hour).unwrap()
                .with_minute(minute).unwrap();
            
            for schedule in self.exchanges.values() {
                if schedule.is_open_at(check_time) {
                    covered_minutes += 15;
                    break; // Count each block only once
                }
            }
        }

        Decimal::from(covered_minutes) / Decimal::from(1440) * Decimal::from(100)
    }

    /// Get exchange count
    pub fn exchange_count(&self) -> usize {
        self.exchanges.len()
    }

    /// Create default schedule with major exchanges
    pub fn default_with_exchanges() -> Self {
        let mut schedule = Self::new();

        // US Markets
        schedule.add_exchange(ExchangeSchedule::us_equity());
        
        // European Markets
        schedule.add_exchange(ExchangeSchedule::european("LSE", "Europe/London", 8, 0, 16, 30));
        schedule.add_exchange(ExchangeSchedule::european("Xetra", "Europe/Berlin", 9, 0, 17, 30));
        
        // Asian Markets
        schedule.add_exchange(ExchangeSchedule::asian("TSE", "Asia/Tokyo", 9, 0, 15, 0));
        schedule.add_exchange(ExchangeSchedule::asian("HKEX", "Asia/Hong_Kong", 9, 30, 16, 0));
        schedule.add_exchange(ExchangeSchedule::asian("ASX", "Australia/Sydney", 10, 0, 16, 0));
        
        // Crypto (24/7)
        schedule.add_exchange(ExchangeSchedule::crypto("Binance"));
        schedule.add_exchange(ExchangeSchedule::crypto("Coinbase"));

        schedule
    }
}

impl Default for TradingSchedule {
    fn default() -> Self {
        Self::default_with_exchanges()
    }
}

impl ExchangeSchedule {
    /// Check if exchange is open at given time
    pub fn is_open_at(&self, datetime: DateTime<Utc>) -> bool {
        // Check if trading day
        let weekday = datetime.weekday();
        if !self.trading_days.contains(&weekday) {
            return false;
        }

        // Check half day
        let date = datetime.date_naive();
        if self.half_days.contains(&date) {
            // On half days, close early (e.g., 13:00)
            let time = datetime.time();
            return time >= self.regular_hours.open && time < NaiveTime::from_hms_opt(13, 0, 0).unwrap();
        }

        // Check regular hours
        let time = datetime.time();
        if time >= self.regular_hours.open && time <= self.regular_hours.close {
            return true;
        }

        // Check pre-market
        if let Some(ref pre) = self.pre_market {
            if time >= pre.open && time <= pre.close {
                return true;
            }
        }

        // Check after-hours
        if let Some(ref after) = self.after_hours {
            if time >= after.open && time <= after.close {
                return true;
            }
        }

        false
    }

    /// Get session at given time
    pub fn get_session_at(&self, datetime: DateTime<Utc>) -> Option<MarketSession> {
        if !self.is_open_at(datetime) {
            return None;
        }

        let time = datetime.time();
        let session_type = if let Some(ref pre) = self.pre_market {
            if time >= pre.open && time <= pre.close {
                "Pre-Market"
            } else {
                "Regular"
            }
        } else {
            "Regular"
        };

        let liquidity = match session_type {
            "Pre-Market" => 50,
            "After-Hours" => 60,
            _ => 100,
        };

        Some(MarketSession {
            exchange: self.name.clone(),
            session_type: session_type.to_string(),
            open_time: datetime,
            close_time: datetime, // Would calculate actual close
            liquidity_score: liquidity,
        })
    }

    /// Get next open time
    pub fn next_open_after(&self, datetime: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Simplified - would calculate next open based on trading days
        let tomorrow = datetime + Duration::days(1);
        Some(tomorrow.with_hour(self.regular_hours.open.hour()).unwrap()
            .with_minute(self.regular_hours.open.minute()).unwrap())
    }

    /// Get next close time
    pub fn next_close_after(&self, datetime: DateTime<Utc>) -> Option<DateTime<Utc>> {
        if self.is_open_at(datetime) {
            Some(datetime.with_hour(self.regular_hours.close.hour()).unwrap()
                .with_minute(self.regular_hours.close.minute()).unwrap())
        } else {
            None
        }
    }

    /// US Equity markets
    pub fn us_equity() -> Self {
        Self {
            name: "NYSE".to_string(),
            timezone: "America/New_York".to_string(),
            regular_hours: TradingHours {
                open: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
                close: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            },
            pre_market: Some(TradingHours {
                open: NaiveTime::from_hms_opt(4, 0, 0).unwrap(),
                close: NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            }),
            after_hours: Some(TradingHours {
                open: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
                close: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            }),
            trading_days: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            half_days: vec![], // Would populate with actual half days
        }
    }

    /// European markets
    pub fn european(name: &str, timezone: &str, open_h: u32, open_m: u32, close_h: u32, close_m: u32) -> Self {
        Self {
            name: name.to_string(),
            timezone: timezone.to_string(),
            regular_hours: TradingHours {
                open: NaiveTime::from_hms_opt(open_h, open_m, 0).unwrap(),
                close: NaiveTime::from_hms_opt(close_h, close_m, 0).unwrap(),
            },
            pre_market: None,
            after_hours: None,
            trading_days: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            half_days: vec![],
        }
    }

    /// Asian markets
    pub fn asian(name: &str, timezone: &str, open_h: u32, open_m: u32, close_h: u32, close_m: u32) -> Self {
        Self {
            name: name.to_string(),
            timezone: timezone.to_string(),
            regular_hours: TradingHours {
                open: NaiveTime::from_hms_opt(open_h, open_m, 0).unwrap(),
                close: NaiveTime::from_hms_opt(close_h, close_m, 0).unwrap(),
            },
            pre_market: None,
            after_hours: None,
            trading_days: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
            half_days: vec![],
        }
    }

    /// Crypto markets (24/7)
    pub fn crypto(name: &str) -> Self {
        Self {
            name: name.to_string(),
            timezone: "UTC".to_string(),
            regular_hours: TradingHours {
                open: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                close: NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            },
            pre_market: None,
            after_hours: None,
            trading_days: vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun],
            half_days: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_schedule_creation() {
        let schedule = TradingSchedule::new();
        assert_eq!(schedule.exchange_count(), 0);
    }

    #[test]
    fn test_default_schedule() {
        let schedule = TradingSchedule::default();
        assert!(schedule.exchange_count() > 0);
    }

    #[test]
    fn test_us_equity_schedule() {
        let us = ExchangeSchedule::us_equity();
        assert_eq!(us.name, "NYSE");
        assert!(us.pre_market.is_some());
        assert!(us.after_hours.is_some());
    }

    #[test]
    fn test_crypto_schedule() {
        let crypto = ExchangeSchedule::crypto("Binance");
        assert!(crypto.trading_days.contains(&Weekday::Sat));
        assert!(crypto.trading_days.contains(&Weekday::Sun));
    }

    #[test]
    fn test_coverage_percentage() {
        let schedule = TradingSchedule::default();
        let coverage = schedule.calculate_coverage_percentage();
        // With global exchanges + crypto, should be close to 100%
        assert!(coverage > Decimal::from(50));
    }

    #[test]
    fn test_market_session() {
        let session = MarketSession {
            exchange: "NYSE".to_string(),
            session_type: "Regular".to_string(),
            open_time: Utc::now(),
            close_time: Utc::now() + Duration::hours(6),
            liquidity_score: 100,
        };

        assert_eq!(session.exchange_name(), "NYSE");
        assert_eq!(session.liquidity_score(), 100);
    }
}
