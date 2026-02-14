//! Trading Calendar and Market Hours
//!
//! Manages trading hours and holidays for global exchanges

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use std::collections::HashSet;

/// Market hours for an exchange
#[derive(Debug, Clone)]
pub struct MarketHours {
    pub timezone: String,
    pub open_time: NaiveTime,
    pub close_time: NaiveTime,
    pub days: Vec<Weekday>,
    pub lunch_break: Option<(NaiveTime, NaiveTime)>,
    pub extends_to_next_day: bool, // For markets that open evening and close morning
}

impl MarketHours {
    /// Create new market hours
    pub fn new(
        timezone: impl Into<String>,
        open: NaiveTime,
        close: NaiveTime,
        days: Vec<Weekday>,
    ) -> Self {
        Self {
            timezone: timezone.into(),
            open_time: open,
            close_time: close,
            days,
            lunch_break: None,
            extends_to_next_day: false,
        }
    }

    /// With lunch break (e.g., Asian markets)
    pub fn with_lunch_break(mut self, start: NaiveTime, end: NaiveTime) -> Self {
        self.lunch_break = Some((start, end));
        self
    }

    /// Check if market is currently open
    pub fn is_open_now(&self) -> bool {
        let now = Local::now();
        let local_time = now.time();
        let weekday = now.weekday();

        // Check if today is a trading day
        if !self.days.contains(&weekday) {
            return false;
        }

        // Check if within trading hours
        let in_hours = if self.extends_to_next_day {
            // Market spans midnight (e.g., futures)
            local_time >= self.open_time || local_time <= self.close_time
        } else {
            local_time >= self.open_time && local_time <= self.close_time
        };

        if !in_hours {
            return false;
        }

        // Check lunch break
        if let Some((lunch_start, lunch_end)) = self.lunch_break {
            if local_time >= lunch_start && local_time <= lunch_end {
                return false;
            }
        }

        true
    }

    /// Get time until market opens
    pub fn time_until_open(&self) -> Option<Duration> {
        if self.is_open_now() {
            return None;
        }

        let now = Local::now();
        let today = now.date_naive();
        let current_time = now.time();

        // Check remaining days this week
        for day_offset in 0..7 {
            let check_date = today + Duration::days(day_offset);
            let weekday = check_date.weekday();

            if !self.days.contains(&weekday) {
                continue;
            }

            let open_datetime = today.and_time(self.open_time) + Duration::days(day_offset);
            let open_utc: DateTime<Utc> = DateTime::from_utc(open_datetime, Utc);
            let now_utc: DateTime<Utc> = now.into();

            if open_utc > now_utc {
                return Some(open_utc - now_utc);
            }
        }

        None
    }

    /// Get time until market closes
    pub fn time_until_close(&self) -> Option<Duration> {
        if !self.is_open_now() {
            return None;
        }

        let now = Local::now();
        let today = now.date_naive();
        let current_time = now.time();

        let close_datetime = if self.extends_to_next_day && current_time > self.close_time {
            // Close is tomorrow
            today.and_time(self.close_time) + Duration::days(1)
        } else {
            today.and_time(self.close_time)
        };

        let close_utc: DateTime<Utc> = DateTime::from_utc(close_datetime, Utc);
        let now_utc: DateTime<Utc> = now.into();
        Some(close_utc - now_utc)
    }

    /// US Equity market hours (9:30 - 16:00 ET)
    pub fn us_equity() -> Self {
        Self::new(
            "America/New_York",
            NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
        )
    }

    /// European market hours (9:00 - 17:30 local)
    pub fn european() -> Self {
        Self::new(
            "Europe/London",
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(17, 30, 0).unwrap(),
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
        )
    }

    /// Tokyo market hours (9:00 - 15:00 JST, lunch 11:30-12:30)
    pub fn tokyo() -> Self {
        Self::new(
            "Asia/Tokyo",
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
        )
        .with_lunch_break(
            NaiveTime::from_hms_opt(11, 30, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 30, 0).unwrap(),
        )
    }

    /// Hong Kong hours (9:30 - 16:00 HKT, lunch 12:00-13:00)
    pub fn hong_kong() -> Self {
        Self::new(
            "Asia/Hong_Kong",
            NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
        )
        .with_lunch_break(
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
    }

    /// China A-shares (9:30 - 15:00 CST, lunch 11:30-13:00)
    pub fn china() -> Self {
        Self::new(
            "Asia/Shanghai",
            NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
            NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
            vec![Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri],
        )
        .with_lunch_break(
            NaiveTime::from_hms_opt(11, 30, 0).unwrap(),
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
        )
    }

    /// Crypto 24/7
    pub fn crypto() -> Self {
        Self::new(
            "UTC",
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            vec![
                Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri,
                Weekday::Sat, Weekday::Sun,
            ],
        )
    }
}

impl Default for MarketHours {
    fn default() -> Self {
        Self::us_equity()
    }
}

/// Holiday calendar for an exchange
#[derive(Debug, Clone)]
pub struct HolidayCalendar {
    pub exchange: String,
    pub holidays: HashSet<NaiveDate>,
    pub half_days: HashSet<NaiveDate>,
}

impl HolidayCalendar {
    /// Create new calendar
    pub fn new(exchange: impl Into<String>) -> Self {
        Self {
            exchange: exchange.into(),
            holidays: HashSet::new(),
            half_days: HashSet::new(),
        }
    }

    /// Add holiday
    pub fn add_holiday(&mut self, date: NaiveDate) {
        self.holidays.insert(date);
    }

    /// Add half-day
    pub fn add_half_day(&mut self, date: NaiveDate) {
        self.half_days.insert(date);
    }

    /// Check if date is a holiday
    pub fn is_holiday(&self, date: NaiveDate) -> bool {
        self.holidays.contains(&date)
    }

    /// Check if date is a half-day
    pub fn is_half_day(&self, date: NaiveDate) -> bool {
        self.half_days.contains(&date)
    }

    /// Check if date is a trading day
    pub fn is_trading_day(&self, date: NaiveDate) -> bool {
        // Check if weekend
        let weekday = date.weekday();
        if weekday == Weekday::Sat || weekday == Weekday::Sun {
            return false;
        }

        // Check if holiday
        !self.is_holiday(date)
    }

    /// Get next trading day
    pub fn next_trading_day(&self, from: NaiveDate) -> NaiveDate {
        let mut date = from + Duration::days(1);
        while !self.is_trading_day(date) {
            date = date + Duration::days(1);
        }
        date
    }

    /// US Equity holidays (simplified)
    pub fn us_equity() -> Self {
        let mut cal = Self::new("NYSE");
        
        // 2024 holidays (simplified - real impl would be dynamic)
        let holidays = vec![
            "2024-01-01", // New Year's
            "2024-01-15", // MLK Day
            "2024-02-19", // Presidents Day
            "2024-03-29", // Good Friday
            "2024-05-27", // Memorial Day
            "2024-06-19", // Juneteenth
            "2024-07-04", // Independence Day
            "2024-09-02", // Labor Day
            "2024-11-28", // Thanksgiving
            "2024-12-25", // Christmas
        ];

        for h in holidays {
            if let Ok(date) = NaiveDate::parse_from_str(h, "%Y-%m-%d") {
                cal.add_holiday(date);
            }
        }

        // Half days
        let half_days = vec![
            "2024-07-03", // Day before Independence
            "2024-11-29", // Day after Thanksgiving
            "2024-12-24", // Christmas Eve
        ];

        for h in half_days {
            if let Ok(date) = NaiveDate::parse_from_str(h, "%Y-%m-%d") {
                cal.add_half_day(date);
            }
        }

        cal
    }
}

/// Trading calendar combining hours and holidays
#[derive(Debug, Clone)]
pub struct TradingCalendar {
    pub hours: MarketHours,
    pub holidays: HolidayCalendar,
}

impl TradingCalendar {
    /// Create new calendar
    pub fn new(hours: MarketHours, holidays: HolidayCalendar) -> Self {
        Self { hours, holidays }
    }

    /// Check if market is open now
    pub fn is_open_now(&self) -> bool {
        let now = Local::now();
        
        // Check holiday
        if !self.holidays.is_trading_day(now.date_naive()) {
            return false;
        }

        // Check hours
        self.hours.is_open_now()
    }

    /// Check if market is open on specific date/time
    pub fn is_open_at(&self, datetime: DateTime<Utc>) -> bool {
        let local = datetime.with_timezone(&Local);
        
        if !self.holidays.is_trading_day(local.date_naive()) {
            return false;
        }

        let time = local.time();
        let weekday = local.weekday();

        if !self.hours.days.contains(&weekday) {
            return false;
        }

        time >= self.hours.open_time && time <= self.hours.close_time
    }

    /// Get trading days in range
    pub fn trading_days(&self, start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
        let mut days = Vec::new();
        let mut current = start;

        while current <= end {
            if self.holidays.is_trading_day(current) {
                days.push(current);
            }
            current = current + Duration::days(1);
        }

        days
    }
}

impl Default for TradingCalendar {
    fn default() -> Self {
        Self::new(MarketHours::us_equity(), HolidayCalendar::us_equity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_hours_creation() {
        let hours = MarketHours::us_equity();
        assert_eq!(hours.open_time, NaiveTime::from_hms_opt(9, 30, 0).unwrap());
        assert_eq!(hours.close_time, NaiveTime::from_hms_opt(16, 0, 0).unwrap());
    }

    #[test]
    fn test_holiday_calendar() {
        let cal = HolidayCalendar::us_equity();
        
        // Check a known holiday
        let new_year = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(cal.is_holiday(new_year));
        assert!(!cal.is_trading_day(new_year));
    }

    #[test]
    fn test_next_trading_day() {
        let cal = HolidayCalendar::us_equity();
        
        let friday = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(); // Friday
        let next = cal.next_trading_day(friday);
        
        // Should skip weekend and return Monday
        assert_eq!(next, NaiveDate::from_ymd_opt(2024, 1, 8).unwrap());
    }

    #[test]
    fn test_crypto_hours() {
        let crypto = MarketHours::crypto();
        
        // Crypto is always open
        assert!(crypto.days.contains(&Weekday::Sat));
        assert!(crypto.days.contains(&Weekday::Sun));
    }

    #[test]
    fn test_lunch_break() {
        let hk = MarketHours::hong_kong();
        assert!(hk.lunch_break.is_some());
        
        let (start, end) = hk.lunch_break.unwrap();
        assert_eq!(start, NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        assert_eq!(end, NaiveTime::from_hms_opt(13, 0, 0).unwrap());
    }
}
