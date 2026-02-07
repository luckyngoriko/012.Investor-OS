# Sprint 3: Decision Engine + API

> **Duration:** Week 5-6
> **Goal:** CQ calculator, proposal generator, REST API
> **Ref:** [SPEC-v1.0](./SPEC-v1.0.md) | [Golden Path](./GOLDEN-PATH.md)

---

## Scope

### ✅ In Scope
- Conviction Quotient (CQ) calculator
- Trade proposal generator
- Position sizing logic
- Constraint checker
- FastAPI REST endpoints
- Decision journal logging
- Notification service (email/webhook)

### ❌ Out of Scope
- Web UI (Sprint 4)
- Broker integration (future)
- Automated execution (future)

---

## Deliverables

| ID | Deliverable | Acceptance Criteria |
|----|-------------|---------------------|
| S3-D1 | CQ calculator | Correctly combines 4 signals with weights |
| S3-D2 | Proposal generator | Creates proposals for CQ > 0.5 |
| S3-D3 | Position sizer | Respects regime-based limits |
| S3-D4 | Constraint checker | Blocks trades violating limits |
| S3-D5 | Proposals API | CRUD for trade proposals |
| S3-D6 | Positions API | Read current positions |
| S3-D7 | Signals API | Read latest signals |
| S3-D8 | Journal API | Decision journal CRUD |
| S3-D9 | Kill switch API | Emergency stop endpoint |
| S3-D10 | Notifications | Email/webhook on new proposals |

---

## Technical Implementation

### S3-D1: CQ Calculator

```python
# decision_engine/cq_calculator.py

def calculate_cq(signals: dict) -> float:
    """
    Conviction Quotient = weighted average of 4 signal groups
    
    CQ = (PEGY_rel × 0.25) + (Insider × 0.25) + 
         (Sentiment × 0.25) + (Regime_fit × 0.25)
    """
    # QVM average for value component
    qvm_score = (signals['quality_score'] + signals['value_score'] + signals['momentum_score']) / 3
    
    cq = (
        qvm_score * 0.25 +
        signals['insider_score'] * 0.25 +
        signals['sentiment_score'] * 0.25 +
        signals['regime_fit'] * 0.25
    )
    
    return round(cq, 4)
```

### S3-D2: Proposal Generator

```python
# decision_engine/proposal_generator.py
from models import TradeProposal

def generate_proposals(universe: list[str]) -> list[TradeProposal]:
    """
    For each ticker in universe:
    1. Get latest signals
    2. Calculate CQ
    3. If CQ > threshold, create proposal
    """
    proposals = []
    
    for ticker in universe:
        signals = get_latest_signals(ticker)
        cq = calculate_cq(signals)
        
        if cq < 0.50:
            continue  # No trade
        
        action = determine_action(ticker, cq)
        size = calculate_position_size(ticker, cq, signals['regime_fit'])
        
        proposal = TradeProposal(
            ticker=ticker,
            action=action,
            proposed_size=size,
            cq_score=cq,
            rationale=generate_rationale(signals),
            invalidation_price=calculate_invalidation(ticker, signals)
        )
        proposals.append(proposal)
    
    return proposals

def determine_action(ticker: str, cq: float) -> str:
    """Determine BUY/SELL/HOLD based on CQ and current position"""
    current_position = get_position(ticker)
    
    if current_position is None:
        return 'BUY' if cq >= 0.50 else 'WATCH'
    elif cq >= 0.75:
        return 'HOLD'  # High conviction, keep
    elif cq < 0.40:
        return 'SELL'  # Lost conviction
    else:
        return 'HOLD'
```

### S3-D3: Position Sizer

```python
def calculate_position_size(ticker: str, cq: float, regime_fit: float) -> float:
    """
    Position size based on:
    - CQ level (0.5-0.75 = half, 0.75+ = full)
    - Regime limits
    - Portfolio constraints
    """
    # Base size from regime
    regime_limits = {
        1.0: 0.05,   # RISK_ON: 5% max
        0.5: 0.02,   # UNCERTAIN: 2% max
        0.0: 0.00    # RISK_OFF: no trades
    }
    base_size = regime_limits.get(regime_fit, 0.02)
    
    # Adjust by CQ confidence
    if cq >= 0.75:
        size_multiplier = 1.0  # Full position
    elif cq >= 0.50:
        size_multiplier = 0.5  # Half position
    else:
        size_multiplier = 0.0  # No position
    
    return base_size * size_multiplier
```

### S3-D4: Constraint Checker

```python
def check_constraints(proposal: TradeProposal) -> tuple[bool, str]:
    """
    Check if proposal violates any constraints.
    Returns (is_valid, reason)
    """
    portfolio = get_portfolio_state()
    
    # 1. Max concurrent positions
    if proposal.action == 'BUY' and len(portfolio['positions']) >= 8:
        return False, "Max positions (8) reached"
    
    # 2. Sector concentration
    sector = get_sector(proposal.ticker)
    sector_exposure = sum(p['value'] for p in portfolio['positions'] if p['sector'] == sector)
    if sector_exposure / portfolio['nav'] > 0.25:
        return False, f"Sector {sector} exposure > 25%"
    
    # 3. Regime allows trading
    if get_current_regime() == 'RISK_OFF':
        return False, "RISK_OFF regime: no new trades"
    
    # 4. Daily loss limit
    if portfolio['daily_pnl'] < -0.015:
        return False, "Daily loss limit (-1.5%) hit"
    
    return True, "OK"
```

### S3-D5: FastAPI Endpoints

```python
# api/main.py
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

app = FastAPI(title="Investor OS API", version="1.0")

# === PROPOSALS ===

@app.get("/api/proposals")
async def list_proposals(status: str = "PENDING"):
    """List trade proposals by status"""
    return db.query(TradeProposal).filter_by(status=status).all()

@app.get("/api/proposals/{id}")
async def get_proposal(id: str):
    """Get single proposal details"""
    return db.query(TradeProposal).get(id)

@app.post("/api/proposals/{id}/confirm")
async def confirm_proposal(id: str):
    """Confirm trade proposal"""
    proposal = db.query(TradeProposal).get(id)
    if not proposal:
        raise HTTPException(404, "Proposal not found")
    
    proposal.status = "CONFIRMED"
    proposal.confirmed_at = datetime.now()
    db.commit()
    
    # Log to journal
    log_decision(proposal, "CONFIRMED")
    
    return {"status": "confirmed", "id": id}

@app.post("/api/proposals/{id}/reject")
async def reject_proposal(id: str, reason: str = None):
    """Reject trade proposal"""
    proposal = db.query(TradeProposal).get(id)
    proposal.status = "REJECTED"
    db.commit()
    
    log_decision(proposal, "REJECTED", reason)
    
    return {"status": "rejected", "id": id}

# === POSITIONS ===

@app.get("/api/positions")
async def list_positions(status: str = "OPEN"):
    """List current positions"""
    return db.query(Position).filter_by(status=status).all()

# === SIGNALS ===

@app.get("/api/signals/{ticker}")
async def get_signals(ticker: str):
    """Get latest signals for ticker"""
    return db.query(Signal).filter_by(ticker=ticker).order_by(Signal.date.desc()).first()

# === REGIME ===

@app.get("/api/regime")
async def get_regime():
    """Get current market regime"""
    return {
        "regime": get_current_regime(),
        "vix": get_current_vix(),
        "updated_at": get_regime_updated_at()
    }

# === KILL SWITCH ===

@app.post("/api/killswitch")
async def trigger_killswitch(reason: str):
    """Emergency stop all trading"""
    set_trading_enabled(False)
    log_killswitch(reason)
    send_notification(f"KILL SWITCH: {reason}")
    return {"status": "killed", "reason": reason}
```

---

## Golden Path Tests

### Automated Tests

```python
# tests/test_sprint3.py

def test_cq_calculation():
    """GP-S3-01: CQ calculated correctly"""
    signals = {
        'quality_score': 0.8,
        'value_score': 0.7,
        'momentum_score': 0.6,
        'insider_score': 0.9,
        'sentiment_score': 0.5,
        'regime_fit': 1.0
    }
    cq = calculate_cq(signals)
    expected = ((0.8+0.7+0.6)/3 * 0.25) + (0.9 * 0.25) + (0.5 * 0.25) + (1.0 * 0.25)
    assert abs(cq - expected) < 0.01

def test_proposal_generated_for_high_cq():
    """GP-S3-02: Proposal created when CQ > 0.5"""
    proposals = generate_proposals(['AAPL'])
    # Assuming AAPL has CQ > 0.5 in test data
    assert len(proposals) > 0

def test_no_proposal_for_low_cq():
    """GP-S3-03: No proposal when CQ < 0.5"""
    # Use ticker with known low signals
    proposals = generate_proposals(['LOW_CQ_TICKER'])
    assert len(proposals) == 0

def test_position_sizing_by_regime():
    """GP-S3-04: Position size respects regime"""
    size_risk_on = calculate_position_size('AAPL', 0.8, 1.0)
    size_uncertain = calculate_position_size('AAPL', 0.8, 0.5)
    assert size_risk_on > size_uncertain

def test_constraint_blocks_over_limit():
    """GP-S3-05: Constraint checker blocks when limit hit"""
    # Setup: max positions already reached
    valid, reason = check_constraints(new_buy_proposal)
    assert valid == False
    assert "Max positions" in reason

def test_api_proposals_endpoint():
    """GP-S3-06: API returns proposals"""
    response = client.get("/api/proposals")
    assert response.status_code == 200
    assert isinstance(response.json(), list)

def test_api_confirm_proposal():
    """GP-S3-07: Confirm updates status"""
    response = client.post(f"/api/proposals/{proposal_id}/confirm")
    assert response.json()['status'] == 'confirmed'
    
    # Verify in DB
    proposal = db.query(TradeProposal).get(proposal_id)
    assert proposal.status == 'CONFIRMED'

def test_journal_entry_created():
    """GP-S3-08: Decision logged in journal"""
    journal = db.query(DecisionJournal).filter_by(ticker='AAPL').first()
    assert journal is not None
```

### Manual Checklist

- [ ] CQ score appears correct for sample tickers
- [ ] Proposals appear in database after engine runs
- [ ] API `/api/proposals` returns pending proposals
- [ ] Confirm endpoint updates proposal to CONFIRMED
- [ ] Reject endpoint updates proposal to REJECTED
- [ ] Decision journal has entries
- [ ] Kill switch endpoint disables trading
- [ ] Notifications sent on new proposals

---

## Schedule

| Day | Focus |
|-----|-------|
| Day 1 | CQ calculator |
| Day 2 | Proposal generator |
| Day 3 | Position sizing + constraints |
| Day 4 | FastAPI setup + proposals endpoints |
| Day 5 | Positions + signals endpoints |
| Day 6 | Kill switch + notifications |
| Day 7 | Decision journal integration |
| Day 8 | Integration testing |

---

## Exit Criteria

Sprint 3 is **COMPLETE** when:
- ✅ All 8 automated tests pass
- ✅ Manual checklist 100% verified
- ✅ CQ scores match expected calculations
- ✅ API fully functional
- ✅ No critical bugs open
