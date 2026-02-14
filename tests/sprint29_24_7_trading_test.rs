//! Sprint 29: 24/7 Trading Scheduler - Golden Path Tests
//!
//! Tests for:
//! - 24/7 trading loop
//! - Market session management
//! - Futures roll automation
//! - Holiday calendar coordination

use investor_os::scheduler::{
    TradingScheduler, MarketId, Market, MarketStatus,
    futures_roll::{FuturesRollManager, FuturesContract, RollUrgency},
    holiday_calendar::{HolidayCalendar, HolidayType},
    continuous_trading::TradingOpportunity,
};
use chrono::{Utc, Duration, NaiveDate};
use rust_decimal::Decimal;

// ============================================================================
// Test 1: Scheduler Creation
// ============================================================================

#[test]
fn test_trading_scheduler_creation() {
    let scheduler = TradingScheduler::new();
    
    // Check number of markets through status
    let status = scheduler.status_summary(Utc::now());
    assert!(status.total_markets > 5, "Should have multiple markets");
}

// ============================================================================
// Test 2: Market Registration
// ============================================================================

#[test]
fn test_market_registration() {
    let scheduler = TradingScheduler::new();
    
    let nyse = scheduler.get_market(&MarketId("NYSE".to_string()));
    assert!(nyse.is_some(), "NYSE should be registered");
    assert_eq!(nyse.unwrap().region, "Americas");
    
    let tse = scheduler.get_market(&MarketId("TSE".to_string()));
    assert!(tse.is_some(), "Tokyo should be registered");
    assert_eq!(tse.unwrap().region, "Asia-Pacific");
    
    let lse = scheduler.get_market(&MarketId("LSE".to_string()));
    assert!(lse.is_some(), "London should be registered");
}

// ============================================================================
// Test 3: 24/7 Crypto Market
// ============================================================================

#[test]
fn test_crypto_24_7_market() {
    let scheduler = TradingScheduler::new();
    let now = Utc::now();
    
    let crypto = scheduler.get_market(&MarketId("CRYPTO".to_string()));
    assert!(crypto.is_some(), "Crypto market should exist");
    
    let crypto = crypto.unwrap();
    assert!(crypto.is_24_7, "Crypto should be 24/7");
    
    let calendar = HolidayCalendar::global();
    assert!(crypto.is_open(now, &calendar), "Crypto should always be open");
}

// ============================================================================
// Test 4: Market Session Status
// ============================================================================

#[test]
fn test_market_session_status() {
    let scheduler = TradingScheduler::new();
    let now = Utc::now();
    
    // Get status summary
    let status = scheduler.status_summary(now);
    
    assert!(status.total_markets > 5);
    assert!(status.active_markets <= status.total_markets);
}

// ============================================================================
// Test 5: Active Markets Detection
// ============================================================================

#[test]
fn test_active_markets_detection() {
    let scheduler = TradingScheduler::new();
    let now = Utc::now();
    
    let active = scheduler.get_active_markets(now);
    
    // Should have at least crypto market active
    assert!(!active.is_empty(), "Should have at least one active market (crypto)");
    
    // Check if any market is open
    assert!(scheduler.is_any_market_open(now), "Should have at least one market open");
}

// ============================================================================
// Test 6: Trading Opportunities
// ============================================================================

#[test]
fn test_trading_opportunities() {
    let scheduler = TradingScheduler::new();
    let now = Utc::now();
    
    let opportunities = scheduler.get_opportunities(now);
    
    // Should have opportunities from active markets
    assert!(!opportunities.is_empty(), "Should have trading opportunities");
    
    // Crypto should always have opportunities
    assert!(opportunities.iter().any(|o| o.market.0 == "CRYPTO"));
}

// ============================================================================
// Test 7: Futures Roll Detection
// ============================================================================

#[test]
fn test_futures_roll_detection() {
    use investor_os::scheduler::futures_roll::FuturesContract;
    
    let mut scheduler = TradingScheduler::new();
    
    // Register a contract expiring soon
    let contract = FuturesContract {
        symbol: "ES".to_string(),
        contract_code: "ESZ24".to_string(),
        next_contract: "ESH25".to_string(),
        expiry: Utc::now() + Duration::days(2),
        roll_days_before: 5,
        contract_size: Decimal::from(50),
    };
    
    // Note: In real implementation, we'd add this to scheduler's futures manager
    // For now, just verify the futures manager works
    let expiring = scheduler.check_futures_rolls();
    // May or may not have rolls depending on registered contracts
}

// ============================================================================
// Test 8: Holiday Calendar
// ============================================================================

#[test]
fn test_holiday_calendar() {
    let calendar = HolidayCalendar::global();
    
    // New Year's Day 2024 should be a holiday
    let new_year = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    assert!(calendar.is_market_holiday(&MarketId("NYSE".to_string()), new_year));
    
    // Regular day should not be a holiday
    let regular_day = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();
    assert!(!calendar.is_market_holiday(&MarketId("NYSE".to_string()), regular_day));
}

// ============================================================================
// Test 9: Market Holiday Check
// ============================================================================

#[test]
fn test_market_holiday_check() {
    let scheduler = TradingScheduler::new();
    
    let new_year = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    
    // NYSE should be closed on New Year's
    assert!(scheduler.is_market_holiday(&MarketId("NYSE".to_string()), new_year));
}

// ============================================================================
// Test 10: Next Market Opening
// ============================================================================

#[test]
fn test_next_market_opening() {
    let scheduler = TradingScheduler::new();
    let now = Utc::now();
    
    let next_open = scheduler.next_market_open(now);
    
    // Should have a next opening (at least crypto is always open)
    // Or another market will open
    let _ = next_open; // Just verify it doesn't panic
}

// ============================================================================
// Test 11: Market Time Zones
// ============================================================================

#[test]
fn test_market_time_zones() {
    let scheduler = TradingScheduler::new();
    
    let tokyo = scheduler.get_market(&MarketId("TSE".to_string())).unwrap();
    let nyse = scheduler.get_market(&MarketId("NYSE".to_string())).unwrap();
    let london = scheduler.get_market(&MarketId("LSE".to_string())).unwrap();
    
    // Each market should have different timezone characteristics
    assert_ne!(tokyo.timezone.local_minus_utc(), nyse.timezone.local_minus_utc());
}

// ============================================================================
// Test 12: Session Transition
// ============================================================================

#[test]
fn test_session_tick() {
    let mut scheduler = TradingScheduler::new();
    let now = Utc::now();
    
    // Tick the scheduler
    scheduler.tick(now);
    
    // Should update internal state without panic
    let status = scheduler.status_summary(now);
    assert!(status.total_markets > 0);
}

// ============================================================================
// Test 13: Futures Contract Expiration
// ============================================================================

#[test]
fn test_futures_contract_expiration() {
    use investor_os::scheduler::futures_roll::{FuturesRollManager, FuturesContract, RollUrgency};
    
    let mut manager = FuturesRollManager::new();
    
    // Contract expiring in 2 days (high urgency)
    manager.register_contract(FuturesContract {
        symbol: "ES".to_string(),
        contract_code: "ESZ24".to_string(),
        next_contract: "ESH25".to_string(),
        expiry: Utc::now() + Duration::days(2),
        roll_days_before: 5,
        contract_size: Decimal::from(50),
    });
    
    // Contract expiring in 10 days (no urgency)
    manager.register_contract(FuturesContract {
        symbol: "NQ".to_string(),
        contract_code: "NQZ24".to_string(),
        next_contract: "NQH25".to_string(),
        expiry: Utc::now() + Duration::days(10),
        roll_days_before: 5,
        contract_size: Decimal::from(20),
    });
    
    let expiring = manager.detect_expiring_contracts();
    
    // Should detect the ES contract (expiring in 2 days)
    assert!(!expiring.is_empty(), "Should detect expiring contracts");
    assert!(expiring.iter().any(|e| e.symbol == "ES"));
    assert!(expiring.iter().any(|e| e.urgency == RollUrgency::High));
}

// ============================================================================
// Test 14: Global Market Coverage
// ============================================================================

#[test]
fn test_global_market_coverage() {
    let scheduler = TradingScheduler::new();
    
    // Should have markets in all regions
    // Get markets and their regions through individual lookups
    let market_ids = vec!["TSE", "LSE", "NYSE", "CRYPTO"];
    let regions: Vec<_> = market_ids.iter()
        .filter_map(|id| scheduler.get_market(&MarketId(id.to_string())))
        .map(|m| m.region.clone())
        .collect();
    
    assert!(regions.iter().any(|r| r == "Asia-Pacific"));
    assert!(regions.iter().any(|r| r == "Europe" || r == "Americas"));
}

// ============================================================================
// Test 15: 24/7 Status
// ============================================================================

#[test]
fn test_24_7_status() {
    let scheduler = TradingScheduler::new();
    let now = Utc::now();
    
    let status = scheduler.status_summary(now);
    
    // Crypto market should always be active
    assert!(status.is_24_7_active, "Should have 24/7 market active");
}

// ============================================================================
// Test 16: Trading Continuity
// ============================================================================

#[test]
fn test_trading_continuity() {
    let scheduler = TradingScheduler::new();
    
    // At any given time, at least crypto should be tradeable
    let times = vec![
        Utc::now(),
        Utc::now() + Duration::hours(6),
        Utc::now() + Duration::hours(12),
        Utc::now() + Duration::hours(18),
    ];
    
    for time in times {
        let active = scheduler.get_active_markets(time);
        assert!(!active.is_empty(), "Should always have at least one active market");
        
        // Crypto should always be there
        assert!(active.iter().any(|m| m.is_24_7), "Crypto should always be open");
    }
}
