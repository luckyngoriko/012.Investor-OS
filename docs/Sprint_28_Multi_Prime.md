# Sprint 28: Multi-Prime Brokerage

## Goal
Enable trading through multiple prime brokers with intelligent routing, financing optimization, and institutional-grade execution capabilities.

## User Stories

### Story 1: Prime Broker Selection
> As a trader, I want the system to select the best prime broker for each trade based on costs and execution quality

**Acceptance Criteria:**
- Compare financing rates across brokers
- Consider execution fees and commissions
- Factor in margin requirements
- Historical execution quality scoring

### Story 2: Cross-Margining
> As a risk manager, I want to optimize margin across multiple prime brokers to reduce capital requirements

**Acceptance Criteria:**
- Calculate total margin across all brokers
- Optimize positions for cross-margining benefits
- Track excess/deficit margin per broker
- Automatic rebalancing suggestions

### Story 3: Financing Rate Optimization
> As a treasurer, I want to route to the broker with lowest financing rates for overnight positions

**Acceptance Criteria:**
- Real-time financing rate comparison
- Automatic routing to cheapest broker
- Track financing costs over time
- Negotiate rates based on volume

## Technical Requirements

### Prime Broker Configuration
```rust
pub struct PrimeBroker {
    pub id: BrokerId,
    pub name: String,
    pub financing_rates: FinancingRates,
    pub commission_structure: CommissionStructure,
    pub margin_requirements: MarginRequirements,
    pub execution_quality: ExecutionQualityMetrics,
    pub api_latency_ms: u64,
    pub reliability_score: f64,
}
```

### Financing Rates
```rust
pub struct FinancingRates {
    pub base_rate: Decimal,           // E.g., SOFR + spread
    pub long_rate: Decimal,           // For long positions
    pub short_rate: Decimal,          // For short positions
    pub margin_rate: Decimal,         // For margin loans
    pub tiered_rates: Vec<TieredRate>, // Volume-based discounts
}

impl FinancingRates {
    pub fn calculate_overnight_cost(&self, position: &Position) -> Decimal;
    pub fn get_effective_rate(&self, volume_30d: Decimal) -> Decimal;
}
```

### Cross-Margining Engine
```rust
pub struct CrossMarginingEngine {
    brokers: Vec<PrimeBroker>,
    positions: HashMap<BrokerId, Vec<Position>>,
    margin_calculator: MarginCalculator,
}

impl CrossMarginingEngine {
    pub fn calculate_total_margin(&self) -> MarginSummary;
    pub fn find_optimization_opportunities(&self) -> Vec<RebalanceSuggestion>;
    pub fn calculate_net_exposure(&self) -> HashMap<String, Decimal>;
}
```

### Prime Broker Router
```rust
pub struct PrimeBrokerRouter {
    brokers: Vec<PrimeBroker>,
    selector: BrokerSelector,
    historical_performance: PerformanceTracker,
}

impl PrimeBrokerRouter {
    pub async fn route_order(&self, order: Order) -> Result<BrokerId>;
    pub fn best_broker_for_asset(&self, symbol: &str) -> Option<&PrimeBroker>;
    pub fn cheapest_financing(&self, side: Side) -> Option<&PrimeBroker>;
}
```

## Broker Categories

### Tier 1: Global Banks
- Goldman Sachs (GS)
- Morgan Stanley (MS)
- JPMorgan (JPM)
- Bank of America (BofA)
- Citi
- UBS
- Credit Suisse
- Barclays

### Tier 2: Specialized Prime Brokers
- Interactive Brokers (IBKR)
- Schwab
- Fidelity
- E*Trade

### Tier 3: Direct Market Access
- Tradestation
- Lightspeed
- CenterPoint Securities

## Fee Structure Comparison

| Broker | Commission | Financing Rate | Min Account | API Quality |
|--------|-----------|----------------|-------------|-------------|
| IBKR | $0.0035/share | BM + 1.5% | $0 | Excellent |
| Goldman | Custom | BM + 0.5% | $10M | Premium |
| Schwab | $0 | BM + 2.0% | $0 | Good |
| Lightspeed | $0.0010/share | BM + 2.5% | $25K | Good |

## Definition of Done
- [ ] Support 10+ prime brokers
- [ ] Real-time financing rate comparison
- [ ] Cross-margining calculation
- [ ] Broker selection based on cost + quality
- [ ] 8-10 Golden Path tests passing

## Test Scenarios

### Test 1: Broker Selection by Cost
```rust
#[tokio::test]
async fn test_cheapest_broker_selection() {
    // Given: Same asset available at 3 brokers with different rates
    // Should: Route to broker with lowest total cost
}
```

### Test 2: Cross-Margining
```rust
#[test]
fn test_cross_margining_optimization() {
    // Given: Long AAPL at Broker A, Short AAPL at Broker B
    // Should: Calculate net exposure = 0 (perfect hedge)
}
```

### Test 3: Financing Cost Tracking
```rust
#[test]
fn test_overnight_financing_calculation() {
    // Given: $100K overnight position
    // Should: Calculate exact financing cost based on rate
}
```
