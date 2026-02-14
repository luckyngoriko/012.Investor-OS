//! Holiday Calendar Management

use chrono::{DateTime, Utc, Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Holiday calendar for a market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolidayCalendar {
    pub market: String,
    pub holidays: HashSet<NaiveDate>,
    pub half_days: HashSet<NaiveDate>,
}

/// Market holiday definition
#[derive(Debug, Clone)]
pub struct MarketHoliday {
    pub date: NaiveDate,
    pub name: String,
    pub market_closure: MarketClosure,
}

#[derive(Debug, Clone)]
pub enum MarketClosure {
    FullDay,
    HalfDay,
    EarlyClose(u32, u32), // hour, minute
}

impl HolidayCalendar {
    pub fn new(market: &str) -> Self {
        Self {
            market: market.to_string(),
            holidays: HashSet::new(),
            half_days: HashSet::new(),
        }
    }
    
    /// Check if date is a holiday
    pub fn is_holiday(&self, date: DateTime<Utc>) -> bool {
        let naive = NaiveDate::from_ymd_opt(date.year(), date.month(), date.day()).unwrap();
        self.holidays.contains(&naive)
    }
    
    /// Check if date is a half day
    pub fn is_half_day(&self, date: DateTime<Utc>) -> bool {
        let naive = NaiveDate::from_ymd_opt(date.year(), date.month(), date.day()).unwrap();
        self.half_days.contains(&naive)
    }
    
    /// Add holiday
    pub fn add_holiday(&mut self, date: NaiveDate) {
        self.holidays.insert(date);
    }
    
    /// Add half day
    pub fn add_half_day(&mut self, date: NaiveDate) {
        self.half_days.insert(date);
    }
    
    /// Initialize with US market holidays
    pub fn us_market() -> Self {
        
        
        // Major US holidays (simplified - would load from external source in production)
        // New Year's Day, MLK Day, Presidents Day, Good Friday, Memorial Day,
        // Juneteenth, Independence Day, Labor Day, Thanksgiving, Christmas
        
        Self::new("US")
    }
}

impl Default for HolidayCalendar {
    fn default() -> Self {
        Self::new("DEFAULT")
    }
}
