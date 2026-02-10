# Sprint 17: Global Markets Expansion

> **Status:** PLANNED  
> **Duration:** 2 weeks  
> **Goal:** Add international exchanges and 24/7 trading  
> **Depends on:** Sprint 11 (Multi-Asset), Sprint 16 (DeFi)

---

## Overview

Expand to global markets: EU, Asia-Pacific, Emerging Markets. Multi-currency, local compliance.

---

## Goals

- [ ] EU markets (Xetra, Euronext, LSE)
- [ ] Asia-Pacific (HKEX, Nikkei, ASX)
- [ ] Emerging markets (Bovespa, NSE, MOEX)
- [ ] 50+ FX pairs
- [ ] Multi-currency accounts
- [ ] Local compliance (MiFID II, etc.)

---

## Technical Tasks

### 1. European Markets
```rust
src/brokers/europe/
├── mod.rs
├── xetra.rs            // Germany
├── euronext.rs         // France, Netherlands, etc.
├── lse.rs              // London
├── six_swiss.rs        // Switzerland
└── borsa_italiana.rs   // Italy
```

### 2. Asia-Pacific
```rust
src/brokers/asia/
├── mod.rs
├── hkex.rs             // Hong Kong
├── nikkei.rs           // Japan
├── asx.rs              // Australia
├── sgx.rs              // Singapore
└── krx.rs              // South Korea
```

### 3. Emerging Markets
```rust
src/brokers/emerging/
├── mod.rs
├── bovespa.rs          // Brazil
├── nse.rs              // India
├── moex.rs             // Russia (sanctions?)
├── jse.rs              // South Africa
└── bist.rs             // Turkey
```

### 4. Multi-Currency
```rust
src/domain/currency.rs

pub struct MultiCurrencyAccount {
    base_currency: Currency,      // EUR
    balances: HashMap<Currency, Decimal>,
    conversions: HashMap<(Currency, Currency), ExchangeRate>,
}

impl MultiCurrencyAccount {
    pub fn convert(&self, amount: Decimal, from: Currency, to: Currency) -> Decimal;
}
```

### 5. Market Hours Scheduler
```rust
src/market/scheduler.rs

pub struct GlobalMarketScheduler {
    // 24/7 trading across time zones
    pub fn get_open_markets(&self) -> Vec<Market>;
    pub fn get_next_opening(&self, market: Market) -> DateTime;
    pub fn get_next_closing(&self, market: Market) -> DateTime;
}
```

### 6. Local Compliance
```rust
src/compliance/regional/
├── mod.rs
├── mifid_ii.rs         // EU
├── sec.rs              // US (CAT reporting)
├── hkma.rs             // Hong Kong
└── fca.rs              // UK
```

---

## Supported Markets

| Region | Markets | Hours (UTC) |
|--------|---------|-------------|
| Americas | NYSE, NASDAQ, Bovespa | 14:30-21:00 |
| Europe | LSE, Xetra, Euronext | 08:00-16:30 |
| Asia | HKEX, NSE, Nikkei | 01:30-08:00 |
| Pacific | ASX, SGX | 23:00-05:00 |

---

## 24/7 Trading Coverage

```
UTC Time:  00  04  08  12  16  20  24
           |   |   |   |   |   |   |
Sydney:    [=========]
Tokyo:         [=========]
Hong Kong:         [=========]
India:             [=========]
Europe:                    [=========]
UK:                        [=========]
US:                                [=========]
Brazil:                            [=========]
Crypto:    [=================================] (24/7)
```

---

## Success Criteria

- [ ] Trade on 10+ exchanges
- [ ] 50+ FX pairs available
- [ ] Multi-currency P&L
- [ ] Local compliance reports
- [ ] < 200ms order routing

---

## Dependencies

- Sprint 6: IB API (international routing)
- Sprint 11: Multi-asset foundation
- Sprint 16: DeFi (global 24/7)

---

## Golden Path Tests

```rust
#[test]
fn test_xetra_order() { ... }

#[test]
fn test_hkex_connection() { ... }

#[test]
fn test_fx_eurusd_order() { ... }

#[test]
fn test_multi_currency_pnl() { ... }

#[test]
fn test_market_hours_calculation() { ... }

#[test]
fn test_mifid_compliance_report() { ... }

#[test]
fn test_best_execution_routing() { ... }

#[test]
fn test_24_7_coverage() { ... }
```

---

**Next:** Sprint 18 (Automation & Compliance)
