# Sprint 27: Global Exchange Integration

## Goal
Enable trading across 50+ global exchanges with unified API, regional market coverage, trading hours management, and cross-market arbitrage capabilities.

## User Stories

### Story 1: Multi-Exchange Connectivity
> As a trader, I want to connect to exchanges in different regions so I can access global liquidity

**Acceptance Criteria:**
- Support 50+ exchanges (US, EU, APAC, LATAM)
- Unified API across all exchanges
- Automatic exchange selection based on liquidity
- Regional market data aggregation

### Story 2: Trading Hours Management
> As a system, I need to track trading hours across time zones so I don't submit orders when markets are closed

**Acceptance Criteria:**
- Trading hours for each exchange
- Holiday calendar management
- Pre-market and after-hours support
- Time zone handling

### Story 3: Cross-Market Arbitrage
> As an AI, I want to identify arbitrage opportunities across markets for the same asset

**Acceptance Criteria:**
- Real-time price comparison across exchanges
- Currency conversion for cross-border arbitrage
- Latency-aware opportunity detection
- Risk-adjusted arbitrage sizing

## Technical Requirements

### Exchange Registry
```rust
pub struct Exchange {
    pub id: ExchangeId,
    pub name: String,
    pub region: Region,
    pub country: String,
    pub time_zone: Tz,
    pub trading_hours: TradingHours,
    pub supported_assets: Vec<AssetClass>,
    pub api_endpoint: String,
    pub rate_limits: RateLimits,
    pub fees: FeeStructure,
}
```

### Trading Hours
```rust
pub struct TradingHours {
    pub regular: Vec<TimeRange>,
    pub pre_market: Option<TimeRange>,
    pub after_hours: Option<TimeRange>,
    pub holidays: Vec<NaiveDate>,
    pub timezone: Tz,
}

impl TradingHours {
    pub fn is_open(&self, datetime: DateTime<Utc>) -> bool;
    pub fn time_until_open(&self, datetime: DateTime<Utc>) -> Duration;
    pub fn time_until_close(&self, datetime: DateTime<Utc>) -> Duration;
}
```

### Global Order Router
```rust
pub struct GlobalOrderRouter {
    exchanges: HashMap<ExchangeId, ExchangeConnection>,
    selector: ExchangeSelector,
    arbitrage_detector: ArbitrageDetector,
}

impl GlobalOrderRouter {
    pub async fn route_order(&self, order: Order) -> Result<ExchangeId>;
    pub async fn get_best_price(&self, symbol: &str) -> Result<GlobalQuote>;
    pub async fn find_arbitrage(&self) -> Vec<ArbitrageOpportunity>;
}
```

### Arbitrage Detection
```rust
pub struct ArbitrageOpportunity {
    pub symbol: String,
    pub buy_exchange: ExchangeId,
    pub sell_exchange: ExchangeId,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub size: Decimal,
    pub profit_pct: f64,
    pub estimated_latency_ms: u64,
    pub confidence: f64,
}
```

## Exchange Coverage

### North America (15 exchanges)
- US: NYSE, NASDAQ, CBOE, BATS, IEX
- Canada: TSX, TSX Venture, CSE
- Mexico: BMV
- Other: ICE, CME (futures)

### Europe (20 exchanges)
- UK: LSE, AIX
- Germany: Xetra, Tradegate
- France: Euronext Paris
- Netherlands: Euronext Amsterdam
- Switzerland: SIX Swiss
- Nordics: OMX Nordic (Stockholm, Copenhagen, Helsinki)
- Italy: Borsa Italiana
- Spain: BME
- Portugal: Euronext Lisbon
- Poland: GPW
- Russia: MOEX
- Turkey: BIST

### Asia-Pacific (12 exchanges)
- Japan: TSE, JPX
- Hong Kong: HKEX
- China: SSE, SZSE, BSE
- Singapore: SGX
- Australia: ASX
- South Korea: KRX
- India: NSE, BSE
- Taiwan: TWSE

### Latin America (5 exchanges)
- Brazil: B3
- Argentina: BYMA
- Chile: BCS
- Colombia: BVC
- Peru: BVL

## Definition of Done
- [ ] 50+ exchanges configured
- [ ] Trading hours accurate for all exchanges
- [ ] Holiday calendars for major markets
- [ ] Cross-market arbitrage detection < 500ms
- [ ] 8-10 Golden Path tests passing

## Test Scenarios

### Test 1: Exchange Selection
```rust
#[tokio::test]
async fn test_best_exchange_selection() {
    // Given order for AAPL
    // Should route to exchange with best liquidity/lowest fees
}
```

### Test 2: Trading Hours
```rust
#[tokio::test]
async fn test_market_hours_check() {
    // Should reject orders when market is closed
    // Should allow pre-market orders when configured
}
```

### Test 3: Cross-Market Arbitrage
```rust
#[tokio::test]
async fn test_arbitrage_detection() {
    // Given price discrepancy between NYSE and LSE
    // Should detect and report opportunity
}
```
