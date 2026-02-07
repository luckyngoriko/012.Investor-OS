# Sprint 2: Signal Engine

> **Duration:** Week 3-4
> **Goal:** Implement all signal scorers
> **Ref:** [SPEC-v1.0](./SPEC-v1.0.md) | [Golden Path](./GOLDEN-PATH.md)

---

## Scope

### ✅ In Scope
- QVM scorer (Quality, Value, Momentum)
- SEC EDGAR Form 4 scraper
- Insider transaction scorer
- StockTwits sentiment collector
- VADER sentiment scorer
- FinBERT earnings call sentiment
- VIX-based regime detector
- Signal storage in database

### ❌ Out of Scope
- CQ calculation (Sprint 3)
- Trade proposals (Sprint 3)
- Web UI (Sprint 4)

---

## Deliverables

| ID | Deliverable | Acceptance Criteria |
|----|-------------|---------------------|
| S2-D1 | Quality scorer | Returns 0-1 score based on ROA, FCF, debt |
| S2-D2 | Value scorer | Returns 0-1 based on PEGY relative to peers |
| S2-D3 | Momentum scorer | Returns 0-1 based on price + revision momentum |
| S2-D4 | SEC EDGAR scraper | Fetches Form 4 filings daily |
| S2-D5 | Insider scorer | Returns 0-1 based on buy/sell flow |
| S2-D6 | StockTwits collector | Fetches posts every 15 min |
| S2-D7 | VADER sentiment | Scores social media sentiment |
| S2-D8 | FinBERT sentiment | Scores earnings call transcripts |
| S2-D9 | Regime detector | Returns RISK_ON/UNCERTAIN/RISK_OFF |
| S2-D10 | Signals table | All signals stored with date/ticker key |

---

## Technical Implementation

### S2-D1: Quality Scorer

```python
# scorers/qvm.py

def calculate_quality_score(fundamentals: dict) -> float:
    """
    Quality score based on:
    - ROA > 10% (profitability)
    - Free Cash Flow > 0 (cash generation)
    - Debt/Equity < sector median (leverage)
    - Earnings-Cash correlation (accounting quality)
    """
    roa_score = min(fundamentals['roa'] / 0.10, 1.0) if fundamentals['roa'] > 0 else 0
    fcf_score = 1.0 if fundamentals['fcf'] > 0 else 0
    debt_score = 1.0 if fundamentals['debt_equity'] < fundamentals['sector_median_de'] else 0
    accrual_score = fundamentals.get('earnings_cash_corr', 0.5)
    
    return (roa_score + fcf_score + debt_score + accrual_score) / 4
```

### S2-D2: Value Scorer

```python
def calculate_value_score(metrics: dict, peer_metrics: list) -> float:
    """
    Value score based on relative PEGY.
    Lower PEGY = higher score.
    """
    pe = metrics['pe_ratio']
    growth = metrics['eps_growth']
    div_yield = metrics['dividend_yield']
    
    pegy = pe / (growth + div_yield) if (growth + div_yield) > 0 else 999
    peer_pegys = [p['pegy'] for p in peer_metrics]
    peer_median = np.median(peer_pegys)
    
    pegy_relative = pegy / peer_median if peer_median > 0 else 1
    
    # Invert: lower is better → higher score
    return max(0, 1 - (pegy_relative - 0.5))  # 0.5 = very cheap, 1.5 = expensive
```

### S2-D3: Momentum Scorer

```python
def calculate_momentum_score(prices: pd.DataFrame, revisions: dict) -> float:
    """
    Momentum based on:
    - 6-month price return
    - EPS revision trend (3-month)
    """
    price_now = prices.iloc[-1]['close']
    price_6m = prices.iloc[-126]['close'] if len(prices) >= 126 else prices.iloc[0]['close']
    price_momentum = (price_now / price_6m) - 1
    
    revision_momentum = revisions.get('eps_revision_3m', 0)
    
    # Normalize both to 0-1
    price_score = min(max(price_momentum + 0.5, 0), 1)  # Center at 0
    revision_score = min(max(revision_momentum + 0.5, 0), 1)
    
    return (price_score + revision_score) / 2
```

### S2-D4: SEC EDGAR Scraper

```python
# collectors/edgar.py
from sec_edgar_downloader import Downloader

@shared_task
def fetch_form4_filings(ticker: str):
    """Fetch Form 4 insider transaction filings"""
    dl = Downloader("investor-os", "email@example.com")
    dl.get("4", ticker, limit=10)
    # Parse XML and store transactions
    ...
```

### S2-D5: Insider Scorer

```python
def calculate_insider_score(transactions: list) -> float:
    """
    Insider score based on Form 4 in last 90 days.
    - Buys are positive
    - Sells are negative (but less weight)
    - Clustered buying (3+ insiders) is strong signal
    """
    recent = [t for t in transactions if t['date'] >= date.today() - timedelta(days=90)]
    
    buys = len([t for t in recent if t['type'] == 'P'])
    sells = len([t for t in recent if t['type'] == 'S'])
    
    # Net flow score
    total = buys + sells
    if total == 0:
        flow_score = 0.5  # Neutral
    else:
        flow_score = 0.5 + (buys - sells) / (2 * total)
    
    # Cluster bonus
    cluster_bonus = 0.25 if buys >= 3 else 0
    
    return min(flow_score + cluster_bonus, 1.0)
```

### S2-D8: FinBERT Sentiment

```python
# scorers/sentiment.py
from transformers import BertTokenizer, BertForSequenceClassification
import torch

class FinBERTScorer:
    def __init__(self):
        self.tokenizer = BertTokenizer.from_pretrained('yiyanghkust/finbert-tone')
        self.model = BertForSequenceClassification.from_pretrained('yiyanghkust/finbert-tone')
    
    def score(self, text: str) -> float:
        """Return sentiment score 0-1 (0=negative, 1=positive)"""
        inputs = self.tokenizer(text, return_tensors="pt", truncation=True, max_length=512)
        outputs = self.model(**inputs)
        probs = torch.softmax(outputs.logits, dim=1)
        # FinBERT: [negative, neutral, positive]
        return probs[0][2].item()  # Return positive probability
```

### S2-D9: Regime Detector

```python
# scorers/regime.py
from hmmlearn import hmm
import numpy as np

class RegimeDetector:
    """
    HMM with 3 states:
    0 = RISK_ON (low vol, bullish)
    1 = UNCERTAIN (transition)
    2 = RISK_OFF (high vol, bearish)
    """
    
    def __init__(self, model_path: str = None):
        if model_path:
            self.model = joblib.load(model_path)
        else:
            self.model = hmm.GaussianHMM(n_components=3, covariance_type="diag")
    
    def predict(self, vix: float, breadth: float, credit_spread: float) -> tuple:
        """Returns (regime_state, confidence)"""
        features = np.array([[vix, breadth, credit_spread]])
        state = self.model.predict(features)[0]
        
        regime_map = {0: 'RISK_ON', 1: 'UNCERTAIN', 2: 'RISK_OFF'}
        regime_fit = {0: 1.0, 1: 0.5, 2: 0.0}
        
        return regime_map[state], regime_fit[state]
```

---

## Golden Path Tests

### Automated Tests

```python
# tests/test_sprint2.py

def test_quality_score_range():
    """GP-S2-01: Quality score is 0-1"""
    score = calculate_quality_score(sample_fundamentals)
    assert 0 <= score <= 1

def test_value_score_lower_is_better():
    """GP-S2-02: Lower PEGY gets higher score"""
    cheap = calculate_value_score({'pe': 10, 'growth': 20, 'yield': 2}, peers)
    expensive = calculate_value_score({'pe': 30, 'growth': 5, 'yield': 1}, peers)
    assert cheap > expensive

def test_momentum_score_positive():
    """GP-S2-03: Rising prices get higher score"""
    rising = calculate_momentum_score(rising_prices, {'eps_revision_3m': 0.1})
    falling = calculate_momentum_score(falling_prices, {'eps_revision_3m': -0.1})
    assert rising > falling

def test_insider_cluster_bonus():
    """GP-S2-04: 3+ buys gets cluster bonus"""
    clustered = calculate_insider_score([{'type': 'P'}] * 3)
    single = calculate_insider_score([{'type': 'P'}])
    assert clustered > single

def test_finbert_sentiment():
    """GP-S2-05: FinBERT returns valid score"""
    scorer = FinBERTScorer()
    score = scorer.score("The company exceeded expectations")
    assert 0 <= score <= 1
    assert score > 0.5  # Should be positive

def test_regime_detector_states():
    """GP-S2-06: Regime detector returns valid state"""
    detector = RegimeDetector()
    regime, fit = detector.predict(vix=15, breadth=0.7, credit_spread=2)
    assert regime in ['RISK_ON', 'UNCERTAIN', 'RISK_OFF']
    assert 0 <= fit <= 1

def test_signals_stored():
    """GP-S2-07: All signals stored in database"""
    result = execute("""
        SELECT quality_score, value_score, momentum_score, 
               insider_score, sentiment_score, regime_fit
        FROM signals WHERE ticker = 'AAPL' AND date = CURRENT_DATE
    """)
    assert result is not None
    assert all(0 <= s <= 1 for s in result[0])
```

### Manual Checklist

- [ ] QVM scorer runs without errors for test universe
- [ ] SEC EDGAR scraper fetches Form 4 filings
- [ ] Insider transactions stored in database
- [ ] StockTwits collector fetches posts
- [ ] VADER sentiment returns scores
- [ ] FinBERT model loads and scores text
- [ ] Regime detector returns consistent states
- [ ] All signals visible in `signals` table

---

## Schedule

| Day | Focus |
|-----|-------|
| Day 1 | Quality scorer |
| Day 2 | Value scorer + peer comparison |
| Day 3 | Momentum scorer |
| Day 4 | SEC EDGAR scraper |
| Day 5 | Insider scorer |
| Day 6 | Sentiment (VADER + StockTwits) |
| Day 7 | FinBERT integration |
| Day 8 | Regime detector |
| Day 9 | Signal aggregation + storage |
| Day 10 | Integration testing |

---

## Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| transformers | 4.30+ | FinBERT |
| hmmlearn | 0.3+ | Regime HMM |
| vaderSentiment | 3.3+ | Social sentiment |
| sec-edgar-downloader | 5.0+ | Form 4 filings |
| numpy | 1.24+ | Calculations |
| pandas | 2.0+ | Data processing |

---

## Exit Criteria

Sprint 2 is **COMPLETE** when:
- ✅ All 7 automated tests pass
- ✅ Manual checklist 100% verified
- ✅ All signals calculated for test universe
- ✅ No critical bugs open
