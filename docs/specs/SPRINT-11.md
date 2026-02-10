# Sprint 11: Multi-Asset Expansion (Crypto & Forex)

> **Status:** IMPLEMENTED  
> **Duration:** 2 weeks  
> **Goal:** Add Crypto and Forex trading capabilities  
> **Depends on:** Sprint 6 (IB Integration), Sprint 10 (AI Foundation)

---

## Overview

Expand beyond equities to Crypto and Forex markets. Integration with Binance, Coinbase Pro, and OANDA.

---

## Implementation Summary

### ✅ Completed Features

#### 1. Binance Integration (`src/broker/binance/`)
```rust
pub struct BinanceClient {
    api_key: String,
    api_secret: String,
    client: reqwest::Client,
    base_url: String,
}
```

**Features:**
- Get crypto prices (BTC, ETH, SOL, etc.)
- Account balance retrieval
- Order placement (MARKET, LIMIT)
- HMAC-SHA256 authentication

**Supported Cryptos:**
| Symbol | Name | Precision |
|--------|------|-----------|
| BTCUSDT | Bitcoin | 0.000001 |
| ETHUSDT | Ethereum | 0.0001 |
| SOLUSDT | Solana | 0.01 |
| ADAUSDT | Cardano | 0.1 |
| DOTUSDT | Polkadot | 0.01 |

#### 2. OANDA Forex Integration (`src/broker/oanda/`)
```rust
pub struct OandaClient {
    api_key: String,
    account_id: String,
    client: reqwest::Client,
    base_url: String,
}
```

**Features:**
- Get forex prices (50+ pairs)
- Account summary
- Practice/Live environment support

**Supported Pairs:**
| Type | Pairs |
|------|-------|
| Majors | EUR_USD, GBP_USD, USD_JPY, USD_CHF, AUD_USD, USD_CAD, NZD_USD |
| Crosses | EUR_GBP, EUR_JPY, GBP_JPY, etc. |
| Total | 28+ pairs |

#### 3. Multi-Asset Portfolio (`src/broker/multi_asset.rs`)
```rust
pub struct MultiAssetPortfolio {
    positions: Vec<MultiAssetPosition>,
    cash_balances: HashMap<String, Decimal>,
    total_value_usd: Decimal,
}
```

**Features:**
- Unified position tracking across asset classes
- Real-time allocation calculation
- Risk concentration analysis
- Portfolio rebalancing

**Asset Classes:**
```rust
pub enum AssetClass {
    Equity,     // Existing
    Crypto,     // NEW
    Forex,      // NEW
    ETF,
    Commodity,
}
```

#### 4. Unified Order Management
```rust
pub struct UnifiedOrder {
    symbol: String,
    asset_class: AssetClass,
    side: OrderSide,      // Buy/Sell
    quantity: Decimal,
    order_type: OrderType, // Market/Limit/Stop
    price: Option<Decimal>,
}
```

**Order Router:**
- Automatically routes to correct exchange
- Binance for Crypto
- OANDA for Forex
- IB for Equities

---

## API Endpoints

### Binance
| Endpoint | Purpose |
|----------|---------|
| GET /api/v3/ticker/price | Get symbol price |
| GET /api/v3/account | Account info |
| POST /api/v3/order | Place order |

### OANDA
| Endpoint | Purpose |
|----------|---------|
| GET /v3/accounts/{id}/pricing | Get prices |
| GET /v3/accounts/{id}/summary | Account summary |

---

## Usage Examples

### Crypto Trading
```rust
use investor_os::broker::binance::BinanceClient;

let binance = BinanceClient::new(
    "api_key".to_string(),
    "api_secret".to_string()
);

// Get BTC price
let btc_price = binance.get_price("BTCUSDT").await?;
println!("BTC: ${}", btc_price);

// Get account balances
let account = binance.get_account().await?;
for balance in account.balances {
    println!("{}: {}", balance.asset, balance.free);
}
```

### Forex Trading
```rust
use investor_os::broker::oanda::OandaClient;

let oanda = OandaClient::new(
    "api_key".to_string(),
    "account_id".to_string(),
    true // practice mode
);

// Get EUR/USD price
let eurusd = oanda.get_price("EUR_USD").await?;
println!("EUR/USD: {}", eurusd);
```

### Multi-Asset Portfolio
```rust
use investor_os::broker::multi_asset::MultiAssetPortfolio;

let mut portfolio = MultiAssetPortfolio::new();

// Add crypto positions
portfolio.add_crypto_positions(&binance_balances, &prices);

// Add forex balance
portfolio.add_forex_balance(&oanda_account);

// Calculate total value
portfolio.calculate_total_value();

// Get allocation
let allocation = portfolio.get_allocation();
println!("Crypto: {}%", allocation.get(&AssetClass::Crypto).unwrap_or(&Decimal::ZERO));
```

---

## Configuration

```yaml
# config/brokers.yaml
binance:
  api_key: "${BINANCE_API_KEY}"
  api_secret: "${BINANCE_SECRET}"
  testnet: false

oanda:
  api_key: "${OANDA_API_KEY}"
  account_id: "${OANDA_ACCOUNT}"
  practice: true  # Use practice environment
```

---

## Risk Management

### Position Limits
| Asset Class | Max Position | Max Leverage |
|-------------|--------------|--------------|
| Crypto | 10% portfolio | 1x (spot) |
| Forex | 5% portfolio | 10x |
| Equity | 5% portfolio | 2x |

### Circuit Breakers
- Auto-liquidation if margin < 20%
- Daily loss limit: 3% of portfolio
- Volatility-adjusted position sizing

---

## Testing

```bash
# Run multi-asset tests
cargo test --test multi_asset_integration_test

# Run broker tests
cargo test --lib broker::
```

---

## Next Steps (Sprint 12)

- Real-time streaming (WebSocket)
- Cross-asset arbitrage detection
- Advanced forex strategies (carry trade)
- Crypto staking/lending

---

## Dependencies Added

| Crate | Purpose |
|-------|---------|
| reqwest | HTTP client |
| rust_decimal | Financial precision |
| serde | Serialization |
| chrono | Timestamps |

---

**Completed:** 2026-02-08  
**Next:** Sprint 12 (Real-Time Streaming)
