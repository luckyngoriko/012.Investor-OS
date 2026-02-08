# Sprint 6: Broker Integration - Trade Execution

> **Duration:** Week 11-12
> **Goal:** Connect to Interactive Brokers for paper and live trading
> **Ref:** [SPEC-v1.0](./SPEC-v1.0.md) | [Golden Path](./GOLDEN-PATH.md) | [ROADMAP](../ROADMAP.md)

---

## Scope

### ✅ In Scope
- Interactive Brokers API integration
- Paper trading mode
- Order management system
- Position reconciliation
- Order status tracking
- Kill switch integration with broker

### ❌ Out of Scope
- Multi-broker support (Phase 2)
- Options/futures trading
- Algorithmic order types (TWAP, VWAP)
- Direct market access (DMA)

---

## Deliverables

| ID | Deliverable | Acceptance Criteria |
|----|-------------|---------------------|
| S6-D1 | IBKR API client | TWS/IB Gateway connection |
| S6-D2 | Broker trait abstraction | Generic interface for orders |
| S6-D3 | Paper trading mode | Simulated execution, same CQ logic |
| S6-D4 | Order management | Place, cancel, modify orders |
| S6-D5 | Position reconciliation | Sync positions with broker |
| S6-D6 | Order status tracking | WebSocket updates for fills |
| S6-D7 | Kill switch integration | Emergency cancel all orders |
| S6-D8 | Trade confirmation | Post-execution journal entry |

---

## Technical Implementation

### S6-D1: Interactive Brokers Client

```rust
// crates/investor-broker/src/ibkr.rs
use ibapi::Client;

pub struct IBKRClient {
    client: Client,
    account_id: String,
}

impl IBKRClient {
    pub fn connect(host: &str, port: u16, client_id: i32) -> Result<Self> {
        let client = Client::connect(format!("{}:{}", host, port), client_id)?;
        
        Ok(Self {
            client,
            account_id: Self::get_account_id(&client)?,
        })
    }
    
    pub async fn get_positions(&self) -> Result<Vec<BrokerPosition>> {
        let positions = self.client.positions()?;
        
        positions.iter().map(|p| {
            Ok(BrokerPosition {
                ticker: p.contract.symbol.clone(),
                shares: p.position,
                avg_cost: p.average_cost,
                market_price: p.market_price,
            })
        }).collect()
    }
}
```

### S6-D2: Broker Trait Abstraction

```rust
// crates/investor-broker/src/lib.rs
#[async_trait]
pub trait Broker: Send + Sync {
    /// Get account information
    async fn get_account(&self) -> Result<Account>;
    
    /// Get current positions
    async fn get_positions(&self) -> Result<Vec<Position>>;
    
    /// Place a new order
    async fn place_order(&self, order: Order) -> Result<OrderId>;
    
    /// Cancel an existing order
    async fn cancel_order(&self, order_id: OrderId) -> Result<()>;
    
    /// Modify an existing order
    async fn modify_order(&self, order_id: OrderId, order: Order) -> Result<()>;
    
    /// Get order status
    async fn get_order_status(&self, order_id: OrderId) -> Result<OrderStatus>;
    
    /// Cancel all orders (kill switch)
    async fn cancel_all_orders(&self) -> Result<()>;
    
    /// Close all positions (emergency)
    async fn close_all_positions(&self) -> Result<()>;
}

/// Broker factory
pub struct BrokerFactory;

impl BrokerFactory {
    pub fn create_ibkr(config: IBKRConfig) -> Result<Box<dyn Broker>> {
        let client = IBKRClient::connect(&config.host, config.port, config.client_id)?;
        Ok(Box::new(client))
    }
    
    pub fn create_paper(initial_capital: Money) -> Result<Box<dyn Broker>> {
        Ok(Box::new(PaperBroker::new(initial_capital)))
    }
}
```

### S6-D3: Paper Trading Mode

```rust
// crates/investor-broker/src/paper.rs
pub struct PaperBroker {
    initial_capital: Money,
    cash: Money,
    positions: HashMap<String, PaperPosition>,
    orders: HashMap<OrderId, PaperOrder>,
    order_counter: AtomicU64,
    price_feed: Arc<dyn PriceFeed>,
}

impl PaperBroker {
    pub fn new(initial_capital: Money) -> Self {
        Self {
            cash: initial_capital,
            initial_capital,
            positions: HashMap::new(),
            orders: HashMap::new(),
            order_counter: AtomicU64::new(1),
            price_feed: Arc::new(YahooPriceFeed::new()),
        }
    }
    
    async fn execute_order(&mut self, order: &Order) -> Result<Fill> {
        // Get current market price
        let market_price = self.price_feed.get_price(&order.ticker).await?;
        
        // Calculate fill (assume full fill for market orders)
        let fill_price = market_price;
        let commission = self.calculate_commission(order.shares, fill_price);
        
        // Update cash and positions
        let total_cost = fill_price * order.shares as f64 + commission;
        self.cash -= total_cost;
        
        let position = self.positions.entry(order.ticker.clone()).or_default();
        position.shares += order.shares;
        position.avg_price = (position.avg_price * position.shares as f64 + total_cost) 
            / (position.shares + order.shares) as f64;
        
        Ok(Fill {
            order_id: order.id.clone(),
            shares: order.shares,
            price: fill_price,
            commission,
            timestamp: Utc::now(),
        })
    }
}

#[async_trait]
impl Broker for PaperBroker {
    async fn place_order(&self, order: Order) -> Result<OrderId> {
        let order_id = OrderId::new();
        
        // Validate order
        self.validate_order(&order)?;
        
        // Execute immediately (paper trading)
        let fill = self.execute_order(&order).await?;
        
        // Record in journal
        self.record_trade(&order, &fill).await?;
        
        Ok(order_id)
    }
    
    async fn get_positions(&self) -> Result<Vec<Position>> {
        Ok(self.positions.values().map(|p| p.to_position()).collect())
    }
    
    async fn cancel_all_orders(&self) -> Result<()> {
        // In paper mode, orders execute immediately
        // But we track pending orders for limit orders
        self.orders.clear();
        Ok(())
    }
}
```

### S6-D4: Order Management

```rust
// crates/investor-core/src/order.rs
pub struct Order {
    pub id: OrderId,
    pub ticker: Ticker,
    pub action: OrderAction, // Buy, Sell
    pub order_type: OrderType,
    pub shares: i32,
    pub limit_price: Option<Money>,
    pub stop_price: Option<Money>,
    pub time_in_force: TimeInForce,
    pub created_at: DateTime<Utc>,
}

pub enum OrderType {
    Market,
    Limit(Money),
    Stop(Money),
    StopLimit(Money, Money),
}

pub enum TimeInForce {
    Day,
    GTC, // Good Till Canceled
    IOC, // Immediate or Cancel
}
```

### S6-D5: Position Reconciliation

```rust
// crates/investor-broker/src/reconciliation.rs
pub struct PositionReconciler {
    broker: Arc<dyn Broker>,
    db: Arc<Database>,
}

impl PositionReconciler {
    pub async fn reconcile(&self) -> Result<ReconciliationReport> {
        let broker_positions = self.broker.get_positions().await?;
        let db_positions = self.db.get_positions().await?;
        
        let mut discrepancies = Vec::new();
        
        for broker_pos in &broker_positions {
            match db_positions.iter().find(|p| p.ticker == broker_pos.ticker) {
                Some(db_pos) => {
                    if (broker_pos.shares - db_pos.shares).abs() > 0.001 {
                        discrepancies.push(Discrepancy {
                            ticker: broker_pos.ticker.clone(),
                            broker_shares: broker_pos.shares,
                            db_shares: db_pos.shares,
                        });
                    }
                }
                None => {
                    // Position in broker but not in DB
                    discrepancies.push(Discrepancy {
                        ticker: broker_pos.ticker.clone(),
                        broker_shares: broker_pos.shares,
                        db_shares: 0.0,
                    });
                }
            }
        }
        
        Ok(ReconciliationReport { discrepancies })
    }
}
```

### S6-D6: Order Status WebSocket

```rust
// crates/investor-api/src/websocket.rs
use axum::extract::ws::{WebSocket, WebSocketUpgrade};

pub async fn order_status_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state, user))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: Arc<AppState>,
    user: User,
) {
    let mut rx = state.order_updates.subscribe();
    
    while let Ok(update) = rx.recv().await {
        // Filter updates for this user's orders
        if update.user_id == user.id {
            let msg = serde_json::to_string(&update).unwrap();
            if socket.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    }
}
```

### S6-D7: Kill Switch Integration

```rust
// crates/investor-decision/src/killswitch.rs
pub struct KillSwitch {
    enabled: Arc<AtomicBool>,
    broker: Arc<dyn Broker>,
}

impl KillSwitch {
    pub async fn trigger(&self, reason: &str) -> Result<KillSwitchResult> {
        // 1. Set flag to prevent new orders
        self.enabled.store(true, Ordering::SeqCst);
        
        // 2. Cancel all pending orders
        let cancelled = self.broker.cancel_all_orders().await?;
        
        // 3. Close all positions (optional, configurable)
        // self.broker.close_all_positions().await?;
        
        // 4. Log the action
        self.log_trigger(reason, cancelled).await?;
        
        Ok(KillSwitchResult { cancelled_orders: cancelled.len() })
    }
}
```

### S6-D8: Trade Confirmation

```rust
pub async fn on_order_filled(
    &self,
    fill: &Fill,
    proposal: &TradeProposal,
) -> Result<()> {
    // Create journal entry
    let entry = DecisionJournal::new()
        .ticker(&fill.ticker)
        .decision_type(DecisionType::Buy)
        .entry_price(fill.price)
        .shares(fill.shares)
        .commission(fill.commission)
        .rationale(&proposal.rationale)
        .cq_score(proposal.cq_score)
        .build();
    
    self.journal.add_entry(entry).await?;
    
    // Update position
    self.positions.update_or_create(fill).await?;
    
    // Send notification
    self.notifications.send_trade_confirmation(fill).await?;
    
    Ok(())
}
```

---

## Golden Path Tests

### S6-GP-01: IBKR Connection
```rust
#[tokio::test]
async fn test_ibkr_connection() {
    let client = IBKRClient::connect("127.0.0.1", 7497, 1)
        .expect("Failed to connect to TWS");
    
    let account = client.get_account().await.unwrap();
    assert!(!account.account_id.is_empty());
}
```

### S6-GP-02: Paper Trading Order
```rust
#[tokio::test]
async fn test_paper_trading_order() {
    let broker = PaperBroker::new(Money::new(dec!(100000)));
    
    let order = Order::market_buy("AAPL", 100);
    let order_id = broker.place_order(order).await.unwrap();
    
    let status = broker.get_order_status(order_id).await.unwrap();
    assert_eq!(status.state, OrderState::Filled);
    
    let positions = broker.get_positions().await.unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].ticker, "AAPL");
}
```

### S6-GP-03: Position Reconciliation
```rust
#[tokio::test]
async fn test_position_reconciliation() {
    let reconciler = setup_test_reconciler().await;
    
    // Create discrepancy
    reconciler.broker.update_position("AAPL", 100).await;
    reconciler.db.update_position("AAPL", 90).await;
    
    let report = reconciler.reconcile().await.unwrap();
    
    assert_eq!(report.discrepancies.len(), 1);
    assert_eq!(report.discrepancies[0].ticker, "AAPL");
}
```

### S6-GP-04: Kill Switch Cancels Orders
```rust
#[tokio::test]
async fn test_killswitch_cancels_orders() {
    let broker = PaperBroker::new(Money::new(dec!(100000)));
    let killswitch = KillSwitch::new(broker.clone());
    
    // Place some orders
    for _ in 0..5 {
        broker.place_order(Order::market_buy("AAPL", 10)).await.unwrap();
    }
    
    // Trigger kill switch
    let result = killswitch.trigger("Test emergency").await.unwrap();
    
    assert_eq!(result.cancelled_orders, 5);
    
    // Verify no new orders allowed
    let new_order = broker.place_order(Order::market_buy("MSFT", 10)).await;
    assert!(new_order.is_err());
}
```

### S6-GP-05: Order Creates Journal Entry
```rust
#[tokio::test]
async fn test_order_creates_journal_entry() {
    let (broker, journal) = setup_test_broker().await;
    
    let proposal = TradeProposal::new("AAPL", TradeAction::Buy, 0.05, Score::new(0.8));
    let order = Order::from_proposal(&proposal);
    
    broker.place_order(order).await.unwrap();
    
    let entries = journal.get_entries().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].ticker, "AAPL");
}
```

### S6-GP-06: WebSocket Order Updates
```rust
#[tokio::test]
async fn test_websocket_order_updates() {
    let (client, mut rx) = connect_websocket().await;
    
    // Place order
    client.place_order(Order::market_buy("AAPL", 100)).await;
    
    // Should receive fill notification
    let msg = rx.recv().await.unwrap();
    let update: OrderUpdate = serde_json::from_str(&msg).unwrap();
    
    assert_eq!(update.status, OrderStatus::Filled);
}
```

---

## Schedule

| Day | Focus |
|-----|-------|
| Day 1 | IBKR API client, connection handling |
| Day 2 | Broker trait abstraction |
| Day 3 | Paper trading implementation |
| Day 4 | Order management (place, cancel, modify) |
| Day 5 | Position reconciliation |
| Day 6 | WebSocket order updates |
| Day 7 | Kill switch integration |
| Day 8 | Trade confirmation, tests, docs |

---

## Exit Criteria

Sprint 6 is **COMPLETE** when:
- ✅ All 6 Golden Path tests pass
- ✅ Can connect to IB TWS/IB Gateway
- ✅ Paper trading executes orders end-to-end
- ✅ Kill switch cancels all open orders
- ✅ Order fills create journal entries
- ✅ Position reconciliation detects discrepancies

---

## Configuration

```yaml
# config/broker.yaml
broker:
  mode: paper  # or "live"
  
  ibkr:
    host: 127.0.0.1
    port: 7497  # 7496 for TWS, 4001 for IB Gateway
    client_id: 1
    
  paper:
    initial_capital: 100000.00
    commission_rate: 0.005  # $0.005 per share
    
  killswitch:
    close_positions_on_trigger: false
```

---

## Security Notes

- IBKR credentials stored in environment variables
- Paper trading mode is default
- Live mode requires explicit `--live` flag
- All orders logged for audit
