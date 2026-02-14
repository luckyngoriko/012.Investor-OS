//! 24/7 Trading Scheduler Module - Sprint 29
//!
//! Enables continuous trading across global markets:
//! - Market session management
//! - Futures/options roll automation
//! - Holiday calendar coordination
//! - 24/7 risk monitoring

pub mod market_clock;
pub mod session_manager;
pub mod futures_roll;
pub mod holiday_calendar;
pub mod continuous_trading;

pub use market_clock::{GlobalMarketClock, MarketTime, TradingSession};
pub use session_manager::{SessionManager, SessionTransition, MarketSession};
pub use futures_roll::{FuturesRollManager, RollPlan, RollResult, ContractExpiration};
pub use holiday_calendar::{HolidayCalendar, MarketHoliday, HolidayType};
pub use continuous_trading::{ContinuousTradingEngine, TradingOpportunity, MarketStatus};

use chrono::{DateTime, Utc, Duration, NaiveDate, FixedOffset};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;

/// Scheduler error
#[derive(Error, Debug, Clone)]
pub enum SchedulerError {
    #[error("No active markets")]
    NoActiveMarkets,
    
    #[error("Market closed: {market}")]
    MarketClosed { market: String },
    
    #[error("Roll failed: {reason}")]
    RollFailed { reason: String },
    
    #[error("Invalid session transition: {from} to {to}")]
    InvalidTransition { from: String, to: String },
    
    #[error("Holiday conflict: {market} on {date}")]
    HolidayConflict { market: String, date: NaiveDate },
}

/// Global trading scheduler
pub struct TradingScheduler {
    clock: GlobalMarketClock,
    session_manager: SessionManager,
    futures_roll_manager: FuturesRollManager,
    holiday_calendar: HolidayCalendar,
    markets: HashMap<MarketId, Market>,
    current_session: Option<MarketSession>,
}

impl TradingScheduler {
    pub fn new() -> Self {
        let mut scheduler = Self {
            clock: GlobalMarketClock::new(),
            session_manager: SessionManager::new(),
            futures_roll_manager: FuturesRollManager::new(),
            holiday_calendar: HolidayCalendar::global(),
            markets: HashMap::new(),
            current_session: None,
        };
        scheduler.initialize_markets();
        scheduler
    }
    
    /// Initialize default markets
    fn initialize_markets(&mut self) {
        // Asia-Pacific
        self.register_market(Market::tokyo());
        self.register_market(Market::hong_kong());
        self.register_market(Market::singapore());
        self.register_market(Market::sydney());
        
        // Europe
        self.register_market(Market::london());
        self.register_market(Market::frankfurt());
        self.register_market(Market::paris());
        
        // Americas
        self.register_market(Market::new_york());
        self.register_market(Market::chicago());
        self.register_market(Market::toronto());
        self.register_market(Market::sao_paulo());
        
        // Crypto (24/7)
        self.register_market(Market::crypto_global());
    }
    
    /// Register a market
    pub fn register_market(&mut self, market: Market) {
        let id = market.id.clone();
        self.markets.insert(id, market);
    }
    
    /// Get currently active markets
    pub fn get_active_markets(&self, now: DateTime<Utc>) -> Vec<&Market> {
        self.markets
            .values()
            .filter(|m| m.is_open(now, &self.holiday_calendar))
            .collect()
    }
    
    /// Get next market opening
    pub fn next_market_open(&self, now: DateTime<Utc>) -> Option<(MarketId, DateTime<Utc>)> {
        let mut next: Option<(MarketId, DateTime<Utc>)> = None;
        
        for market in self.markets.values() {
            if let Some(open_time) = market.next_opening(now, &self.holiday_calendar) {
                if next.is_none() || open_time < next.as_ref().map(|n: &(MarketId, DateTime<Utc>)| n.1).unwrap() {
                    next = Some((market.id.clone(), open_time));
                }
            }
        }
        
        next
    }
    
    /// Check if any market is open
    pub fn is_any_market_open(&self, now: DateTime<Utc>) -> bool {
        !self.get_active_markets(now).is_empty()
    }
    
    /// Get trading opportunities across active markets
    pub fn get_opportunities(&self, now: DateTime<Utc>) -> Vec<TradingOpportunity> {
        let active = self.get_active_markets(now);
        
        active.iter()
            .map(|m| TradingOpportunity {
                market: m.id.clone(),
                market_name: m.name.clone(),
                time_until_close: m.time_until_close(now),
                session_type: m.current_session(now),
                priority: if m.is_liquid_session(now) { 1 } else { 2 },
            })
            .collect()
    }
    
    /// Detect futures rolls needed
    pub fn check_futures_rolls(&self) -> Vec<ContractExpiration> {
        self.futures_roll_manager.detect_expiring_contracts()
    }
    
    /// Execute futures roll
    pub async fn execute_roll(&self, plan: RollPlan) -> Result<RollResult, SchedulerError> {
        self.futures_roll_manager.execute_roll(plan).await
    }
    
    /// Get current session info
    pub fn current_session(&self) -> Option<&MarketSession> {
        self.current_session.as_ref()
    }
    
    /// Update session (call periodically)
    pub fn tick(&mut self, now: DateTime<Utc>) {
        self.clock.tick(now);
        
        // Check for session transitions
        if let Some(transition) = self.session_manager.check_transitions(now) {
            info!("Market session transition: {:?}", transition);
            self.handle_transition(transition);
        }
        
        // Update current session
        let active = self.get_active_markets(now);
        if let Some(primary) = active.first() {
            self.current_session = Some(MarketSession {
                market: primary.id.clone(),
                start: now,
                session_type: primary.current_session(now),
            });
        }
    }
    
    fn handle_transition(&self, transition: SessionTransition) {
        // Handle session transition logic
        info!("Handling transition from {} to {}", 
            transition.from_market, 
            transition.to_market
        );
    }
    
    /// Get 24/7 status summary
    pub fn status_summary(&self, now: DateTime<Utc>) -> SchedulerStatus {
        let active = self.get_active_markets(now);
        let next_open = self.next_market_open(now);
        
        SchedulerStatus {
            timestamp: now,
            active_markets: active.len(),
            total_markets: self.markets.len(),
            active_market_names: active.iter().map(|m| m.name.clone()).collect(),
            next_market_open_info: next_open,
            is_24_7_active: active.iter().any(|m| m.is_24_7),
        }
    }
    
    /// Check if specific market is holiday
    pub fn is_market_holiday(&self, market_id: &MarketId, date: NaiveDate) -> bool {
        self.holiday_calendar.is_market_holiday(market_id, date)
    }
    
    /// Get market by ID
    pub fn get_market(&self, id: &MarketId) -> Option<&Market> {
        self.markets.get(id)
    }
}

impl Default for TradingScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Market identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(pub String);

/// Market definition
#[derive(Debug, Clone)]
pub struct Market {
    pub id: MarketId,
    pub name: String,
    pub region: String,
    pub timezone: FixedOffset,
    pub trading_hours: TradingHours,
    pub is_24_7: bool,
    pub supported_assets: Vec<AssetType>,
}

/// Trading hours
#[derive(Debug, Clone)]
pub struct TradingHours {
    pub regular: Vec<TimeRange>,
    pub pre_market: Option<TimeRange>,
    pub after_hours: Option<TimeRange>,
}

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: chrono::NaiveTime,
    pub end: chrono::NaiveTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    Stocks,
    Futures,
    Options,
    Crypto,
    Forex,
}

impl Market {
    /// Check if market is open
    pub fn is_open(&self, now: DateTime<Utc>, calendar: &HolidayCalendar) -> bool {
        if self.is_24_7 {
            return true;
        }
        
        // Check holiday
        let local = now.with_timezone(&self.timezone);
        let local_date = local.date_naive();
        if calendar.is_market_holiday(&self.id, local_date) {
            return false;
        }
        
        // Check trading hours
        let local = now.with_timezone(&self.timezone);
        let local_time = local.time();
        
        for range in &self.trading_hours.regular {
            if local_time >= range.start && local_time <= range.end {
                return true;
            }
        }
        
        false
    }
    
    /// Get current session type
    pub fn current_session(&self, now: DateTime<Utc>) -> SessionType {
        if self.is_24_7 {
            return SessionType::Continuous;
        }
        
        let local_time = now.with_timezone(&self.timezone).time();
        
        // Check pre-market
        if let Some(ref pre) = self.trading_hours.pre_market {
            if local_time >= pre.start && local_time <= pre.end {
                return SessionType::PreMarket;
            }
        }
        
        // Check regular
        for range in &self.trading_hours.regular {
            if local_time >= range.start && local_time <= range.end {
                return SessionType::Regular;
            }
        }
        
        // Check after-hours
        if let Some(ref after) = self.trading_hours.after_hours {
            if local_time >= after.start && local_time <= after.end {
                return SessionType::AfterHours;
            }
        }
        
        SessionType::Closed
    }
    
    /// Check if currently in liquid session
    pub fn is_liquid_session(&self, now: DateTime<Utc>) -> bool {
        matches!(self.current_session(now), SessionType::Regular)
    }
    
    /// Get time until market closes
    pub fn time_until_close(&self, now: DateTime<Utc>) -> Duration {
        if self.is_24_7 {
            return Duration::MAX;
        }
        
        let local_time = now.with_timezone(&self.timezone).time();
        
        // Find next close time
        for range in &self.trading_hours.regular {
            if local_time < range.end {
                let duration = range.end.signed_duration_since(local_time);
                return Duration::seconds(duration.num_seconds());
            }
        }
        
        Duration::zero()
    }
    
    /// Get next opening time
    pub fn next_opening(&self, now: DateTime<Utc>, calendar: &HolidayCalendar) -> Option<DateTime<Utc>> {
        if self.is_24_7 {
            return Some(now);
        }
        
        // Simplified - would calculate actual next opening
        let local = now.with_timezone(&self.timezone);
        let local_date = local.date_naive();
        
        // Check if opening later today
        if let Some(first_range) = self.trading_hours.regular.first() {
            if local.time() < first_range.start {
                // Opens later today
                let open_datetime = local_date.and_time(first_range.start);
                let dt: chrono::DateTime<chrono::FixedOffset> = chrono::DateTime::from_naive_utc_and_offset(open_datetime, self.timezone);
                return Some(dt.with_timezone(&Utc));
            }
        }
        
        // Opens tomorrow (or next business day)
        let next_day = local_date.succ_opt()?;
        
        // Skip holidays
        if calendar.is_market_holiday(&self.id, next_day) {
            // Would check subsequent days
            return None;
        }
        
        let first_range = self.trading_hours.regular.first()?;
        let open_datetime = next_day.and_time(first_range.start);
        let dt: chrono::DateTime<chrono::FixedOffset> = chrono::DateTime::from_naive_utc_and_offset(open_datetime, self.timezone);
        Some(dt.with_timezone(&Utc))
    }
    
    // Factory methods
    pub fn tokyo() -> Self {
        use chrono::NaiveTime;
        Self {
            id: MarketId("TSE".to_string()),
            name: "Tokyo Stock Exchange".to_string(),
            region: "Asia-Pacific".to_string(),
            timezone: FixedOffset::east_opt(9 * 3600).unwrap(), // JST = UTC+9
            trading_hours: TradingHours {
                regular: vec![
                    TimeRange { start: NaiveTime::from_hms_opt(9, 0, 0).unwrap(), end: NaiveTime::from_hms_opt(11, 30, 0).unwrap() },
                    TimeRange { start: NaiveTime::from_hms_opt(12, 30, 0).unwrap(), end: NaiveTime::from_hms_opt(15, 0, 0).unwrap() },
                ],
                pre_market: None,
                after_hours: None,
            },
            is_24_7: false,
            supported_assets: vec![AssetType::Stocks, AssetType::Futures],
        }
    }
    
    pub fn new_york() -> Self {
        use chrono::NaiveTime;
        Self {
            id: MarketId("NYSE".to_string()),
            name: "New York Stock Exchange".to_string(),
            region: "Americas".to_string(),
            timezone: FixedOffset::west_opt(5 * 3600).unwrap(), // EST = UTC-5 (simplified)
            trading_hours: TradingHours {
                regular: vec![TimeRange { start: NaiveTime::from_hms_opt(9, 30, 0).unwrap(), end: NaiveTime::from_hms_opt(16, 0, 0).unwrap() }],
                pre_market: Some(TimeRange { start: NaiveTime::from_hms_opt(4, 0, 0).unwrap(), end: NaiveTime::from_hms_opt(9, 30, 0).unwrap() }),
                after_hours: Some(TimeRange { start: NaiveTime::from_hms_opt(16, 0, 0).unwrap(), end: NaiveTime::from_hms_opt(20, 0, 0).unwrap() }),
            },
            is_24_7: false,
            supported_assets: vec![AssetType::Stocks, AssetType::Options],
        }
    }
    
    pub fn crypto_global() -> Self {
        Self {
            id: MarketId("CRYPTO".to_string()),
            name: "Global Crypto Markets".to_string(),
            region: "Global".to_string(),
            timezone: FixedOffset::east_opt(0).unwrap(), // UTC
            trading_hours: TradingHours {
                regular: vec![TimeRange { start: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(), end: chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap() }],
                pre_market: None,
                after_hours: None,
            },
            is_24_7: true,
            supported_assets: vec![AssetType::Crypto],
        }
    }
    
    // Other markets - simplified implementations with unique IDs
    pub fn hong_kong() -> Self {
        let mut market = Self::tokyo();
        market.id = MarketId("HKEX".to_string());
        market.name = "Hong Kong Stock Exchange".to_string();
        market.timezone = FixedOffset::east_opt(8 * 3600).unwrap(); // HKT = UTC+8
        market
    }
    pub fn singapore() -> Self {
        let mut market = Self::tokyo();
        market.id = MarketId("SGX".to_string());
        market.name = "Singapore Exchange".to_string();
        market.timezone = FixedOffset::east_opt(8 * 3600).unwrap(); // SGT = UTC+8
        market
    }
    pub fn sydney() -> Self {
        let mut market = Self::tokyo();
        market.id = MarketId("ASX".to_string());
        market.name = "Australian Securities Exchange".to_string();
        market.timezone = FixedOffset::east_opt(11 * 3600).unwrap(); // AEDT = UTC+11 (simplified)
        market
    }
    pub fn london() -> Self {
        let mut market = Self::tokyo();
        market.id = MarketId("LSE".to_string());
        market.name = "London Stock Exchange".to_string();
        market.region = "Europe".to_string();
        market.timezone = FixedOffset::east_opt(0).unwrap(); // GMT (simplified)
        market
    }
    pub fn frankfurt() -> Self {
        let mut market = Self::tokyo();
        market.id = MarketId("XETRA".to_string());
        market.name = "Xetra".to_string();
        market.region = "Europe".to_string();
        market.timezone = FixedOffset::east_opt(3600).unwrap(); // CET = UTC+1
        market
    }
    pub fn paris() -> Self {
        let mut market = Self::tokyo();
        market.id = MarketId("EURONEXT".to_string());
        market.name = "Euronext Paris".to_string();
        market.region = "Europe".to_string();
        market.timezone = FixedOffset::east_opt(3600).unwrap(); // CET = UTC+1
        market
    }
    pub fn chicago() -> Self {
        let mut market = Self::new_york();
        market.id = MarketId("CME".to_string());
        market.name = "Chicago Mercantile Exchange".to_string();
        market
    }
    pub fn toronto() -> Self {
        let mut market = Self::new_york();
        market.id = MarketId("TSX".to_string());
        market.name = "Toronto Stock Exchange".to_string();
        market
    }
    pub fn sao_paulo() -> Self {
        let mut market = Self::new_york();
        market.id = MarketId("B3".to_string());
        market.name = "B3 - Brasil Bolsa Balcao".to_string();
        market.timezone = FixedOffset::west_opt(3 * 3600).unwrap(); // BRT = UTC-3
        market
    }
}

#[derive(Debug, Clone)]
pub enum SessionType {
    PreMarket,
    Regular,
    AfterHours,
    Continuous, // 24/7 markets
    Closed,
}

/// Scheduler status
#[derive(Debug, Clone)]
pub struct SchedulerStatus {
    pub timestamp: DateTime<Utc>,
    pub active_markets: usize,
    pub total_markets: usize,
    pub active_market_names: Vec<String>,
    pub next_market_open_info: Option<(MarketId, DateTime<Utc>)>,
    pub is_24_7_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = TradingScheduler::new();
        assert!(scheduler.markets.len() > 5);
    }

    #[test]
    fn test_market_registration() {
        let scheduler = TradingScheduler::new();
        
        let nyse = scheduler.get_market(&MarketId("NYSE".to_string()));
        assert!(nyse.is_some());
        assert_eq!(nyse.unwrap().region, "Americas");
    }

    #[test]
    fn test_crypto_24_7() {
        let crypto = Market::crypto_global();
        let calendar = HolidayCalendar::global();
        
        assert!(crypto.is_open(Utc::now(), &calendar));
        assert!(crypto.is_24_7);
    }

    #[test]
    fn test_scheduler_status() {
        let scheduler = TradingScheduler::new();
        let status = scheduler.status_summary(Utc::now());
        
        assert!(status.total_markets > 0);
        assert!(status.active_markets <= status.total_markets);
    }
}
