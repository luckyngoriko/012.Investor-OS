//! Holiday Calendar

use super::MarketId;
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};

/// Holiday calendar
#[derive(Debug, Clone)]
pub struct HolidayCalendar {
    market_holidays: HashMap<MarketId, HashSet<NaiveDate>>,
}

impl HolidayCalendar {
    pub fn new() -> Self {
        Self {
            market_holidays: HashMap::new(),
        }
    }
    
    pub fn global() -> Self {
        let mut cal = Self::new();
        cal.initialize_global_holidays();
        cal
    }
    
    fn initialize_global_holidays(&mut self) {
        // US Market Holidays (simplified)
        let us_holidays: HashSet<NaiveDate> = [
            // 2024 holidays
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),   // New Year
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),  // MLK Day
            NaiveDate::from_ymd_opt(2024, 2, 19).unwrap(),  // Presidents Day
            NaiveDate::from_ymd_opt(2024, 3, 29).unwrap(),  // Good Friday
            NaiveDate::from_ymd_opt(2024, 5, 27).unwrap(),  // Memorial Day
            NaiveDate::from_ymd_opt(2024, 6, 19).unwrap(),  // Juneteenth
            NaiveDate::from_ymd_opt(2024, 7, 4).unwrap(),   // Independence Day
            NaiveDate::from_ymd_opt(2024, 9, 2).unwrap(),   // Labor Day
            NaiveDate::from_ymd_opt(2024, 11, 28).unwrap(), // Thanksgiving
            NaiveDate::from_ymd_opt(2024, 12, 25).unwrap(), // Christmas
        ].iter().cloned().collect();
        
        self.market_holidays.insert(MarketId("NYSE".to_string()), us_holidays.clone());
        self.market_holidays.insert(MarketId("NASDAQ".to_string()), us_holidays);
    }
    
    /// Check if market is holiday
    pub fn is_market_holiday(&self, market_id: &MarketId, date: NaiveDate) -> bool {
        self.market_holidays
            .get(market_id)
            .map(|holidays| holidays.contains(&date))
            .unwrap_or(false)
    }
    
    /// Add holiday for market
    pub fn add_holiday(&mut self, market_id: MarketId, date: NaiveDate) {
        self.market_holidays
            .entry(market_id)
            .or_default()
            .insert(date);
    }
}

impl Default for HolidayCalendar {
    fn default() -> Self {
        Self::new()
    }
}

/// Market holiday definition
#[derive(Debug, Clone)]
pub struct MarketHoliday {
    pub market: MarketId,
    pub date: NaiveDate,
    pub name: String,
    pub holiday_type: HolidayType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolidayType {
    FullDay,
    EarlyClose,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_calendar() {
        let calendar = HolidayCalendar::global();
        
        // NYSE should have holidays
        assert!(calendar.market_holidays.contains_key(&MarketId("NYSE".to_string())));
    }

    #[test]
    fn test_holiday_check() {
        let calendar = HolidayCalendar::global();
        
        // Check New Year's Day 2024
        let new_year = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(calendar.is_market_holiday(&MarketId("NYSE".to_string()), new_year));
        
        // Regular day should not be holiday
        let regular = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
        assert!(!calendar.is_market_holiday(&MarketId("NYSE".to_string()), regular));
    }

    #[test]
    fn test_add_holiday() {
        let mut calendar = HolidayCalendar::new();
        let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        
        calendar.add_holiday(MarketId("TEST".to_string()), date);
        
        assert!(calendar.is_market_holiday(&MarketId("TEST".to_string()), date));
    }
}
