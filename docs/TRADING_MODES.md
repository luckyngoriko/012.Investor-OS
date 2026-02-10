# Trading Modes in Investor OS

## Overview

Investor OS supports three distinct trading modes that determine how autonomous the AI is when making and executing trading decisions. Each mode offers different levels of control and automation to suit different trading styles and experience levels.

---

## The Three Trading Modes

### 1. Manual Mode 🔵

**Icon:** User | **Color:** Blue | **Autonomy Level:** 1/3

**Best for:** Beginners, learning, maximum control

#### How it works:
- AI continuously analyzes market data and generates trade proposals
- Each proposal includes:
  - **CQ Score** (Conviction Quotient): 0-100% confidence rating
  - **Rationale**: Detailed explanation of why this trade
  - **Factors**: Breakdown of PEGY, Insider, Sentiment, Technical scores
  - **Regime Check**: Current market regime compatibility
- **You manually review all proposals** in the dashboard
- **You execute all trades** through your own broker interface
- AI tracks performance and provides insights

#### Execution Flow:
```
Market Data → AI Analysis → Proposal Generated → Dashboard Display
                                                        ↓
You Review → You Confirm/Reject → You Trade via Broker → Position Opened
```

#### When to use:
- You're learning how the AI makes decisions
- You want maximum control over every trade
- You prefer to use your existing broker interface
- You're testing the system before going live

#### UI Indicators:
- Blue badge in header showing "Manual"
- Proposals show "Copy to your broker to execute" text
- Manual trade button prominently displayed

---

### 2. Semi-Automatic Mode 🟠

**Icon:** UserCog | **Color:** Amber/Orange | **Autonomy Level:** 2/3

**Best for:** Most users, balanced approach

#### How it works:
- AI analyzes markets and generates proposals with CQ scores
- **You receive notifications** for new proposals (push/email)
- **You review and confirm/reject** each proposal in the dashboard
- **AI automatically executes** confirmed trades through connected broker
- Configurable confirmation timeout (default: 5 minutes)

#### Execution Flow:
```
Market Data → AI Analysis → Proposal Generated → Notification Sent
                                                        ↓
You Review → You Confirm/Reject → AI Executes via API → Position Opened
              ↓                         ↓
         Rejected                  Filled/Failed
```

#### Configuration Options:
- **CQ Threshold**: Minimum 65% (configurable)
- **Max Trade Value**: $10,000 default
- **Confirmation Timeout**: 5 minutes
- **Notifications**: Push + Email enabled

#### When to use:
- You want AI assistance but final say on trades
- You're comfortable with AI execution after approval
- You want the convenience of automated execution
- You check notifications regularly

#### UI Indicators:
- Amber badge showing "Semi-Auto"
- Pending proposals count in header
- Confirm/Reject buttons on each proposal
- "3 proposals waiting" badge in sidebar

---

### 3. Fully Automatic Mode 🟢

**Icon:** Bot | **Color:** Emerald/Green | **Autonomy Level:** 3/3

**Best for:** Experienced users, hands-off trading

#### How it works:
- AI continuously monitors markets 24/7
- **Auto-executes trades** when CQ >= threshold AND within risk limits
- Respects all configured risk limits:
  - Position size limits
  - VaR (Value at Risk) limits
  - Max drawdown limits
  - Daily loss limits
  - Kill switch triggers
- **You receive notifications** of all executed trades
- **Emergency kill switch** available at all times

#### Execution Flow:
```
Continuous Market Monitoring
            ↓
   AI Generates Proposal
            ↓
    ┌───────┴───────┐
    ↓               ↓
CQ >= 80%      CQ < 80%
    ↓               ↓
Risk Check      Queue for Manual
    ↓
Within Limits?
    ↓
Yes → Auto-Execute → Notification Sent
```

#### Configuration Options:
- **Auto-Execute CQ Threshold**: 80% default (50-100% configurable)
- **Max Auto Trade Value**: $10,000 default
- **Risk Limits**: All standard risk controls apply
- **Notifications**: All trade executions notified

#### When to use:
- You trust the AI and want hands-off trading
- You understand and accept the risks
- You have strict risk limits configured
- You want to trade 24/7 without monitoring

#### Safety Features:
- **Confirmation dialog** when enabling this mode
- **Kill switch** always accessible in header
- **All risk limits enforced** even in auto mode
- **Trade size limits** prevent large unexpected trades

#### UI Indicators:
- Green badge showing "Auto"
- "AI is actively trading" status message
- Real-time trade notifications
- Kill switch button always visible

---

## Mode Comparison Matrix

| Feature | Manual | Semi-Auto | Fully Auto |
|---------|--------|-----------|------------|
| AI Generates Proposals | ✅ | ✅ | ✅ |
| Human Review Required | ✅ | ✅ | ❌ |
| AI Auto-Executes | ❌ | ✅ (confirmed) | ✅ |
| Notifications | Proposals | Proposals | Executions |
| Response Time | Hours | Minutes | Seconds |
| Control Level | Maximum | Balanced | Minimum |
| Best For | Learning | Most Users | Experienced |

---

## Switching Between Modes

### How to Change Mode:
1. Click the **Trading Mode badge** in the dashboard header
2. Select desired mode from the dropdown
3. Configure mode-specific settings
4. Click "Save Settings"

### Mode Change Rules:
- **Any time**: Can switch from any mode to any other mode
- **Fully Auto Warning**: Confirmation dialog shown when enabling (high autonomy)
- **Active Trades**: Mode change doesn't affect already open positions
- **Pending Proposals**: 
  - Manual → Semi: Proposals remain, now executable
  - Semi → Auto: High CQ proposals may auto-execute immediately

---

## Mode-Specific Quick Actions

### Manual Mode Actions:
- **Review Proposals**: See AI analysis for each trade idea
- **Manual Trade**: Place trade directly through your broker
- **Copy Symbol**: Quick copy to clipboard for broker search

### Semi-Auto Mode Actions:
- **Confirm All**: Bulk confirm multiple proposals (if similar)
- **Set Timeout**: Configure confirmation window
- **Snooze**: Temporarily pause proposals

### Fully Auto Mode Actions:
- **Pause Auto**: Temporarily halt auto-execution
- **Kill Switch**: Emergency stop all trading
- **Adjust Threshold**: Change CQ threshold for auto-execution

---

## Configuration Reference

### Environment Variables:
```bash
# Trading Mode (manual, semi_auto, fully_auto)
TRADING_MODE=semi_auto

# Auto-execution settings (for Semi and Fully Auto)
AUTO_EXECUTE_CQ_THRESHOLD=80
MAX_AUTO_TRADE_VALUE=10000

# Notifications
NOTIFY_ON_PROPOSAL=true
NOTIFY_ON_EXECUTION=true
NOTIFY_ON_RISK_ALERT=true
```

### Settings Panel:
Access via: **Dashboard → Header Badge → Settings** or **Sidebar → Settings**

Configurable per mode:
- CQ Threshold slider (50-100%)
- Max trade value ($1,000 - $100,000)
- Notification preferences
- Confirmation timeout (Semi-Auto)

---

## Security & Risk Management

### Mode-Specific Risks:

| Mode | Primary Risk | Mitigation |
|------|--------------|------------|
| Manual | Missing opportunities | Email notifications for new proposals |
| Semi-Auto | Delayed response | Mobile push notifications, 5-min timeout |
| Fully Auto | Unintended trades | CQ threshold, trade limits, kill switch |

### Universal Safeguards (All Modes):
- ✅ Kill switch (emergency stop)
- ✅ Position size limits
- ✅ VaR monitoring
- ✅ Daily loss limits
- ✅ Paper trading mode available
- ✅ All trades logged and auditable

---

## Recommendations by User Type

### Beginner Traders:
1. **Start with Manual Mode**
2. Paper trade for 2-4 weeks
3. Review AI proposals to learn patterns
4. Gradually move to Semi-Auto

### Intermediate Traders:
1. **Use Semi-Auto Mode**
2. Set CQ threshold to 70-75%
3. Review weekly performance
4. Adjust thresholds based on results

### Advanced Traders:
1. **Consider Fully Auto Mode**
2. Set conservative risk limits
3. Start with small position sizes
4. Monitor daily for first month

---

## FAQ

**Q: Can I use different modes for different accounts?**
A: Currently, mode is global per instance. Per-account modes are on the roadmap.

**Q: What happens to pending proposals when I switch modes?**
A: They remain in the queue. In Semi-Auto they await confirmation; in Fully Auto, high-CQ ones may execute.

**Q: Can I set different CQ thresholds for buy vs sell?**
A: Not yet. This feature is planned for v4.0.

**Q: Does Fully Auto work during market hours only?**
A: Yes. AI generates proposals 24/7 but only executes during market hours.

**Q: What if the AI makes a mistake in Fully Auto?**
A: Risk limits prevent catastrophic losses. You can:
1. Trigger kill switch immediately
2. Close position manually
3. Switch to Manual mode
4. Contact support for analysis

---

## Related Documentation

- [Risk Management](./RISK_MANAGEMENT.md)
- [CQ Score Calculation](./CQ_SCORE.md)
- [Kill Switch](./KILL_SWITCH.md)
- [API Reference](./API_REFERENCE.md)
