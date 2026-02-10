# Trading Charts in Professional Systems

## Overview

Professional trading platforms use various chart types to visualize price action, volume, and technical indicators. This document outlines the most important chart types used in systems like TradingView, Bloomberg Terminal, Interactive Brokers TWS, and ThinkorSwim.

---

## 1. Price Charts

### Candlestick Charts (Most Popular)
**Used by:** TradingView, Bloomberg, All major platforms

```
    ┌───┐         Shadow (High)
    │   │
    ├───┤ ← Body (Open-Close)
    │   │
    └───┘         Shadow (Low)
```

- **Green/White Candle**: Close > Open (Bullish)
- **Red/Black Candle**: Close < Open (Bearish)
- **Timeframes**: 1m, 5m, 15m, 1H, 4H, 1D, 1W, 1M
- **Features**: 
  - Wick/shadow shows high/low
  - Body shows open/close range
  - Patterns: Doji, Hammer, Engulfing, etc.

### Heikin Ashi
**Used by:** Trend traders, swing traders

- Modified candlestick that filters noise
- Smoother trends, easier to spot direction
- Formula averages current and previous prices
- Best for: Identifying trend direction

### Line Charts
**Used by:** Long-term investors, overview

- Simple closing price connected by line
- Clean, minimal noise
- Best for: Long-term trend visualization

### Area Charts
**Used by:** Portfolio tracking

- Line chart with filled area below
- Good for showing accumulated value
- Used in: Portfolio performance charts

---

## 2. Volume-Based Charts

### Volume Bars
**Used by:** All platforms

```
Price    │    ████ Volume
Chart    │    ████
         │    ████
```

- Histogram below price chart
- Green: Volume on up candle
- Red: Volume on down candle
- High volume = Strong conviction

### Volume Profile
**Used by:** TradingView, Sierra Chart

- Shows volume traded at each price level
- Identifies support/resistance zones
- POC (Point of Control): Most traded price
- Value Area: 70% of volume

### Footprint Charts (Number Bars)
**Used by:** Order flow traders, Sierra Chart, NinjaTrader

```
    125│Bid  │Ask│  89
    124│ 45  │ 67│ 112
    123│ 89  │ 34│  55
```

- Shows bid/ask volume at each price
- Delta: Difference between buy/sell pressure
- Identifies: Absorption, exhaustion, imbalances

### Cluster Charts
**Used by:** Advanced order flow analysis

- Volume broken down by price level
- Shows where large orders executed
- Colors indicate buy/sell imbalance

---

## 3. Alternative Chart Types

### Renko Charts
**Used by:** Noise filtering, trend following

- Bricks of fixed size (e.g., $1, $5)
- Time-independent
- Only draws when price moves X units
- Filters out small noise

### Range Bars
**Used by:** Volatility-based traders

- New bar when price moves X range
- Time-independent
- Equal range bars for consistent analysis

### Tick Charts
**Used by:** Scalpers, high-frequency

- New bar after X transactions
- 100-tick, 500-tick, 1000-tick charts
- Shows activity intensity

### Point & Figure
**Used by:** Traditional technical analysts

- X's for up moves, O's for down moves
- Filters time, focuses on price
- Clear support/resistance levels

---

## 4. Market Depth & Order Flow

### Depth Chart (Order Book Visualization)
**Used by:** Crypto exchanges, L2 data platforms

```
Price
  $105 │         ╱│ Ask Side
  $104 │        ╱ │
  $103 │       ╱  │
  $102 ├──────┼───┤ ← Mid Price
  $101 │  ╲   │   │
  $100 │   ╲  │   │ Bid Side
  $99  │    ╲ │   │
       └────────────
         Cumulative
```

- Visualizes buy/sell walls
- Shows liquidity at each price
- Identifies: Support/resistance walls

### Heatmaps
**Used by:** Bookmap, TradingLite

- Color-coded visualization of order book over time
- X-axis: Time
- Y-axis: Price
- Color intensity: Order size
- Shows: Iceberg orders, liquidity removal

### DOM (Depth of Market) Ladder
**Used by:** Futures traders, Jigsaw Trading

```
    Bid    │Price │    Ask
    150    │100.50│    89
    234    │100.49│    45
    567    │100.48│    23
```

- Real-time order book
- Shows bid/ask sizes at each level
- One-click trading interface

---

## 5. Technical Indicators

### Trend Indicators

#### Moving Averages
- **SMA**: Simple Moving Average
- **EMA**: Exponential Moving Average (more weight to recent)
- **SMMA**: Smoothed Moving Average
- Common periods: 20, 50, 200

#### Bollinger Bands
```
Upper Band (SMA20 + 2σ)
    ───────────────────
SMA20 ─────────────────
    ───────────────────
Lower Band (SMA20 - 2σ)
```

- Shows volatility
- Price mean-reverts to middle band
- Squeeze = Low volatility (often precedes breakout)

#### VWAP (Volume Weighted Average Price)
**Used by:** Institutional traders, algos

- Average price weighted by volume
- Benchmark for execution quality
- Support/resistance level
- Bands show standard deviations

### Momentum Indicators

#### RSI (Relative Strength Index)
- Range: 0-100
- >70: Overbought
- <30: Oversold
- Divergence signals trend change

#### MACD (Moving Average Convergence Divergence)
```
MACD Line: EMA12 - EMA26
Signal Line: EMA9 of MACD
Histogram: MACD - Signal
```

- Crossover signals
- Histogram shows momentum

#### Stochastic Oscillator
- Range: 0-100
- Shows momentum relative to range
- %K and %D lines

### Volatility Indicators

#### ATR (Average True Range)
- Measures volatility
- Used for stop-loss placement
- Position sizing

#### Keltner Channels
- Similar to Bollinger Bands
- Based on ATR instead of std dev

### Volume Indicators

#### OBV (On-Balance Volume)
- Cumulative volume
- Confirms trends with volume

#### Volume RSI
- RSI applied to volume
- Shows volume momentum

---

## 6. Multi-Timeframe Analysis

### Chart Linking
**Used by:** TradingView, ThinkorSwim

- Multiple charts synchronized
- Different timeframes side-by-side
- Drawing tools sync across charts

### Mini Charts
- Small overview chart showing longer timeframe
- Context for current price action

---

## 7. Recommended Charts for Investor OS

### Primary Chart (Main Trading View)
**Type**: Candlestick with Volume
**Timeframes**: 1m, 5m, 15m, 1H, 4H, 1D
**Indicators**:
- EMA 20, 50, 200
- Bollinger Bands
- VWAP (intraday)
- Volume Profile (right side)

### Risk Visualization Overlay
**Custom for Investor OS**:
- Entry price line
- Stop-loss level (red zone)
- Take-profit level (green zone)
- Position size indicator
- Risk/Reward ratio display

### Market Regime Indicator
**Custom for Investor OS**:
- Color-coded background based on regime
- Risk On: Green tint
- Risk Off: Red tint
- Uncertain: Yellow tint

### Real-time Mode Switching
**Features**:
- Mode toggle buttons on chart
- Visual feedback of current mode
- Quick actions based on mode

---

## 8. Implementation Priority

### Phase 1 (MVP)
1. Candlestick chart with volume
2. EMA 20/50 overlay
3. Risk levels visualization
4. Real-time updates via WebSocket

### Phase 2
1. Bollinger Bands
2. VWAP
3. RSI indicator panel
4. Multiple timeframes

### Phase 3
1. Volume Profile
2. Depth chart
3. Heatmap
4. Drawing tools

---

## References

- **TradingView**: https://www.tradingview.com/chart/
- **Bloomberg Terminal**: Professional charting
- **NinjaTrader**: Advanced charting features
- **Sierra Chart**: Order flow analysis
- **Bookmap**: Heatmap visualization
