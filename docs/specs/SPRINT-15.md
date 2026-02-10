# Sprint 15: Social Trading & Mobile App

> **Status:** PLANNED  
> **Duration:** 2 weeks  
> **Goal:** Build social features and mobile application  
> **Depends on:** Sprint 4 (Web UI), Sprint 14 (Sentiment)

---

## Overview

Social trading features: leaderboards, copy trading, community. React Native mobile app for iOS/Android.

---

## Goals

- [ ] Leaderboards (top traders)
- [ ] Copy trading system
- [ ] React Native mobile app
- [ ] Push notifications
- [ ] Voice commands
- [ ] Gamification (badges, challenges)

---

## Technical Tasks

### 1. Social Trading Backend
```rust
src/social/
├── mod.rs
├── leaderboard.rs
├── copy_trading.rs
├── profiles.rs
└── verification.rs
```

#### Leaderboard
```rust
pub struct Leaderboard {
    period: LeaderboardPeriod, // Daily, Weekly, Monthly, All-time
    metric: RankingMetric,     // Sharpe, CAGR, Win Rate
}

pub struct TraderProfile {
    user_id: UUID,
    verified_pnl: Decimal,     // Cryptographically signed
    strategy_description: String,
    followers: u32,
    copiers: u32,
    risk_score: u8,            // 1-10
}
```

#### Copy Trading
```rust
pub struct CopyTradingEngine {
    pub async fn copy_trades(&self, leader: UserId, allocation: Decimal) -> Result<()>;
    pub async fn stop_copying(&self, leader: UserId) -> Result<()>;
    pub async fn get_copiers(&self, leader: UserId) -> Vec<Copier>;
}
```

### 2. Mobile App (React Native)
```typescript
apps/mobile/
├── src/
│   ├── screens/
│   │   ├── Dashboard.tsx
│   │   ├── Signals.tsx
│   │   ├── Portfolio.tsx
│   │   ├── Leaderboard.tsx
│   │   └── Settings.tsx
│   ├── components/
│   ├── hooks/
│   └── api/
├── ios/
└── android/
```

#### Screens
- [ ] Dashboard (portfolio summary)
- [ ] Signals (CQ alerts)
- [ ] Leaderboard (top traders)
- [ ] Portfolio (positions, P&L)
- [ ] Settings (preferences)

### 3. Push Notifications
```rust
src/notifications/
├── mod.rs
├── push.rs           // Firebase Cloud Messaging
├── email.rs
└── sms.rs
```

#### Notification Types
- High CQ signal detected
- Trade executed
- Stop loss triggered
- Leaderboard position change
- Copy trade executed

### 4. Voice Commands
```rust
src/voice/
├── mod.rs
├── asr.rs            // Whisper API
├── nlu.rs            // Intent recognition
├── commands.rs
└── tts.rs            // ElevenLabs
```

#### Supported Commands
- "Buy 10 shares of AAPL"
- "What's my portfolio value?"
- "Show me high CQ signals"
- "Stop copying trader XYZ"

### 5. Gamification
```rust
src/gamification/
├── mod.rs
├── achievements.rs
├── challenges.rs
├── leagues.rs
└── rewards.rs
```

#### Achievements
- First Trade
- Sharpe > 2.0
- 10 Profitable Months
- Risk Manager (DD < 10%)
- Master Strategist

---

## Mobile Features

| Feature | iOS | Android |
|---------|-----|---------|
| Biometric Auth | Face ID | Fingerprint |
| Widgets | ✅ | ✅ |
| Watch App | Apple Watch | Wear OS |
| Push | APNs | FCM |
| Offline | ✅ | ✅ |

---

## Success Criteria

- [ ] Mobile app in App Store / Play Store
- [ ] < 100ms API response
- [ ] Push notifications < 1s
- [ ] Voice commands 95% accuracy
- [ ] 1000+ users in leaderboard

---

## Dependencies

- Sprint 4: Next.js API (reuse for mobile)
- Sprint 6: Order management (copy trading)
- Sprint 14: Sentiment (community features)

---

## Golden Path Tests

```rust
#[test]
fn test_leaderboard_ranking() { ... }

#[test]
fn test_copy_trade_execution() { ... }

#[test]
fn test_verified_pnl_signature() { ... }

#[test]
fn test_push_notification_delivery() { ... }
```

```typescript
// Mobile tests
it('renders dashboard correctly', () => {});
it('executes trade from mobile', () => {});
it('receives push notification', () => {});
it('voice command parsing', () => {});
```

---

**Next:** Sprint 16 (DeFi Integration)
