# Sprint 29: 24/7 Trading Scheduler

## Goal
Enable continuous 24/7 trading across global markets with automatic session management, futures/options roll handling, and holiday calendar coordination.

## User Stories

### Story 1: Always-On Trading
> As a trader, I want the system to trade continuously across global markets so I never miss opportunities

**Acceptance Criteria:**
- Trade across multiple time zones seamlessly
- Automatic market session switching
- 24/7 position monitoring
- Continuous risk management

### Story 2: Futures Roll Management
> As a system, I need to automatically roll futures contracts before expiration to maintain exposure

**Acceptance Criteria:**
- Detect expiring contracts
- Calculate optimal roll timing
- Execute roll trades
- Track roll costs

### Story 3: Holiday Calendar Coordination
> As a system, I need to track holidays across all markets to avoid submitting orders when markets are closed

**Acceptance Criteria:**
- Holiday calendars for 50+ markets
- Early closure detection
- Multi-day holiday handling
- Automatic trading resumption

## Technical Requirements

### 24/7 Trading Engine
```rust
pub struct TradingScheduler {
    markets: Vec<MarketSession>,
    current_session: MarketSession,
    futures_tracker: FuturesTracker,
    holiday_calendar: GlobalHolidayCalendar,
    risk_monitor: ContinuousRiskMonitor,
}

impl TradingScheduler {
    pub async fn run_continuous(&self) -> Result<()>;
    pub fn get_current_opportunities(&self) -> Vec<TradingOpportunity>;
    pub fn next_market_open(&self) -> DateTime<Utc>;
}
```

### Futures Roll Manager
```rust
pub struct FuturesRollManager {
    expiring_contracts: Vec<FuturesContract>,
    roll_strategy: RollStrategy,
}

impl FuturesRollManager {
    pub fn detect_expiring_contracts(&self) -> Vec<RollRequired>;
    pub fn calculate_optimal_roll(&self, contract: &FuturesContract) -> RollPlan;
    pub async fn execute_roll(&self, plan: RollPlan) -> Result<RollResult>;
}
```

### Global Market Clock
```rustnpub struct GlobalMarketClock {
    current_time: DateTime<Utc>,
    active_markets: Vec<Market>,
    upcoming_sessions: Vec<MarketSession>,
}

impl GlobalMarketClock {
    pub fn new() -> Self;
    pub fn tick(&mut self);
    pub fn get_active_markets(&self) -> Vec<&Market>;
    pub fn time_until_next_session(&self) -> Duration;
}
```

## Market Coverage

### Asia-Pacific (Night Session for US)
- Tokyo (TSE): 00:00 - 06:00 UTC
- Hong Kong (HKEX): 01:30 - 08:00 UTC
- Singapore (SGX): 01:00 - 09:00 UTC
- Sydney (ASX): 22:00 - 05:00 UTC (prev day)

### Europe (Morning Session for US)
- London (LSE): 08:00 - 16:30 UTC
- Frankfurt (Xetra): 08:00 - 16:30 UTC
- Paris (Euronext): 08:00 - 16:30 UTC

### Americas (Day Session)
- New York (NYSE): 14:30 - 21:00 UTC
- Chicago (CME): Various by product
- Toronto (TSX): 14:30 - 21:00 UTC
- São Paulo (B3): 14:00 - 20:00 UTC

### 24/7 Crypto Markets
- Binance, Coinbase, Kraken (always open)

## Roll Schedule

| Asset Class | Roll Frequency | Days Before Expiry |
|-------------|---------------|-------------------|
| Equity Index Futures | Quarterly | 5 days |
| Treasury Futures | Quarterly | 7 days |
| Commodity Futures | Monthly | 3 days |
| Currency Futures | Quarterly | 5 days |
| Crypto Futures | Perpetual | N/A (funding rate) |

## Definition of Done
- [ ] 24/7 trading loop operational
- [ ] Automatic futures roll detection and execution
- [ ] Holiday calendar for 50+ markets
- [ ] Market session transitions smooth
- [ ] 8-10 Golden Path tests passing

## Test Scenarios

### Test 1: Market Session Transition
```rust
#[tokio::test]
async fn test_market_session_transition() {
    // When Tokyo closes, should automatically switch to London
}
```

### Test 2: Futures Roll Detection
```rust
#[test]
fn test_futures_roll_detection() {
    // Given: ES futures expiring in 3 days
    // Should: Detect and plan roll to next contract
}
```

### Test 3: Holiday Handling
```rust
#[test]
fn test_holiday_trading_pause() {
    // Given: US market holiday
    // Should: Pause US trading, continue other markets
}
```
