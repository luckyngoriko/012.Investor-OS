# Sprint 30: Tax & Compliance Engine

## Goal
Implement comprehensive tax management and compliance tracking for trading operations, including tax loss harvesting, wash sale monitoring, and regulatory reporting.

## User Stories

### Story 1: Tax Loss Harvesting
> As a trader, I want the system to automatically identify and execute tax loss harvesting opportunities to minimize my tax liability

**Acceptance Criteria:**
- Identify losing positions with tax loss potential
- Suggest replacement securities (not substantially identical)
- Track 30-day wash sale window
- Calculate tax savings

### Story 2: Wash Sale Monitoring
> As a compliance officer, I need the system to prevent wash sales that would disallow tax losses

**Acceptance Criteria:**
- Detect potential wash sales before execution
- Track 30-day window around loss sales
- Maintain replacement securities list
- Alert on violations

### Story 3: Tax Reporting
> As a trader, I need accurate tax reports for filing (Schedule D, Form 8949)

**Acceptance Criteria:**
- Generate Schedule D report
- Generate Form 8949 details
- Calculate short/long term gains
- Support FIFO, LIFO, HIFO methods
- Export to tax software formats

## Technical Requirements

### Tax Engine
```rust
pub struct TaxEngine {
    lot_tracker: TaxLotTracker,
    wash_sale_monitor: WashSaleMonitor,
    loss_harvester: LossHarvestingEngine,
    report_generator: TaxReportGenerator,
}

impl TaxEngine {
    pub fn calculate_gains(&self, method: CostBasisMethod) -> GainsReport;
    pub fn find_loss_harvesting_opportunities(&self) -> Vec<HarvestOpportunity>;
    pub fn check_wash_sale(&self, trade: &Trade) -> Option<WashSaleViolation>;
    pub fn generate_schedule_d(&self, year: i32) -> ScheduleD;
}
```

### Tax Lot Tracking
```rust
pub struct TaxLot {
    pub lot_id: LotId,
    pub symbol: String,
    pub quantity: Decimal,
    pub cost_basis: Decimal,
    pub acquisition_date: DateTime<Utc>,
    pub term: HoldingTerm, // Short or Long
}

impl TaxLot {
    pub fn unrealized_gain(&self, current_price: Decimal) -> Decimal;
    pub fn is_long_term(&self) -> bool;
}
```

### Wash Sale Monitor
```rust
pub struct WashSaleMonitor {
    loss_sales: Vec<LossSale>,
    replacement_period: Duration, // 30 days
}

impl WashSaleMonitor {
    pub fn record_loss_sale(&mut self, sale: Trade) -> Option<WashSale>;
    pub fn check_repurchase(&self, symbol: &str, date: DateTime<Utc>) -> bool;
    pub fn calculate_disallowed_loss(&self) -> Decimal;
}
```

## Cost Basis Methods

| Method | Description | Use Case |
|--------|-------------|----------|
| FIFO | First In, First Out | Default IRS method |
| LIFO | Last In, Last Out | Recent losses first |
| HIFO | Highest In, First Out | Minimize gains |
| Specific ID | Choose specific lots | Maximum control |

## Tax Report Types

### Schedule D
- Summary of capital gains/losses
- Short-term vs long-term totals
- Carryover losses

### Form 8949
- Detail for each transaction
- Proceeds, cost basis, gain/loss
- Wash sale adjustments
- Code explanations

## Definition of Done
- [ ] FIFO/LIFO/HIFO cost basis calculation
- [ ] Tax loss harvesting detection
- [ ] Wash sale prevention
- [ ] Schedule D generation
- [ ] Form 8949 generation
- [ ] 8-10 Golden Path tests passing

## Test Scenarios

### Test 1: FIFO Cost Basis
```rust
#[test]
fn test_fifo_cost_basis() {
    // Buy 100 @ $50, Buy 100 @ $60, Sell 100 @ $70
    // FIFO: Sold the first lot, gain = $2000
}
```

### Test 2: Wash Sale Detection
```rust
#[test]
fn test_wash_sale_detection() {
    // Sell at loss, buy within 30 days
    // Should flag as wash sale
}
```

### Test 3: Tax Loss Harvesting
```rust
#[test]
fn test_loss_harvesting_opportunity() {
    // Position with $5000 unrealized loss
    // Should suggest harvest before year-end
}
```
