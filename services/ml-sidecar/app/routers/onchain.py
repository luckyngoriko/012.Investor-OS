"""On-chain analytics router (Wave 2 Task 11).

Endpoints:
  GET /v1/onchain/analysis      — combined on-chain analysis (score + verdict)
  GET /v1/onchain/funding-rates — current Binance perpetual funding rates
  GET /v1/onchain/whale-alerts  — recent large BTC transactions
"""

import logging
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from app.models.onchain_model import OnChainAnalyzer

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/onchain", tags=["onchain"])

# Singleton
_analyzer = OnChainAnalyzer()


# --------------- Response Models ---------------


class FundingRateEntry(BaseModel):
    """Single funding rate data point."""
    time: int
    rate: float
    rate_pct: float


class FundingRatesResponse(BaseModel):
    """Binance perpetual futures funding rates."""
    symbol: str
    rates: list[FundingRateEntry]
    current_rate: float
    avg_rate: float
    sentiment: str
    timestamp: str


class WhaleOutput(BaseModel):
    """A single large output in a whale transaction."""
    address: str
    value_btc: float
    value_satoshi: int


class WhaleTransaction(BaseModel):
    """A whale transaction with large BTC outputs."""
    tx_hash: str
    total_large_output_btc: float
    large_outputs: list[WhaleOutput]
    n_large_outputs: int


class WhaleAlertsResponse(BaseModel):
    """Recent whale (large BTC) transactions from latest block."""
    block_height: int
    block_hash: str
    block_time: int = 0
    min_btc_threshold: float
    whale_transactions: list[WhaleTransaction]
    total_whale_volume_btc: float
    whale_tx_count: int
    timestamp: str


class SignalDetail(BaseModel):
    """Detail for a single on-chain signal category."""
    score: float
    # Allow extra fields per signal type
    model_config = {"extra": "allow"}


class AnalysisSignals(BaseModel):
    """Breakdown of individual on-chain signal scores."""
    funding: dict
    exchange_flow: dict
    whale_activity: dict


class AnalysisResponse(BaseModel):
    """Combined on-chain analysis result."""
    symbol: str
    score: float = Field(description="Combined score from -100 (bearish) to +100 (bullish)")
    verdict: str = Field(description="strongly_bullish | bullish | neutral | bearish | strongly_bearish")
    confidence: float = Field(description="0.0 to 1.0 based on data source availability")
    signals: AnalysisSignals
    latency_ms: int
    model_version: str
    timestamp: str


# --------------- Endpoints ---------------


@router.get("/funding-rates", response_model=FundingRatesResponse)
async def get_funding_rates(
    symbol: str = Query(default="BTCUSDT", description="Futures trading pair"),
    limit: int = Query(default=10, ge=1, le=1000, description="Number of recent entries"),
) -> FundingRatesResponse:
    """Get Binance perpetual futures funding rates.

    Positive rate = longs pay shorts (market is bullish-biased).
    Negative rate = shorts pay longs (market is bearish-biased).
    """
    try:
        result = _analyzer.get_funding_rates(symbol=symbol, limit=limit)
    except Exception as e:
        logger.error("Funding rates fetch failed: %s", e)
        raise HTTPException(status_code=500, detail=f"Funding rates fetch failed: {e}")

    return FundingRatesResponse(
        **result,
        timestamp=datetime.now(timezone.utc).isoformat(),
    )


@router.get("/whale-alerts", response_model=WhaleAlertsResponse)
async def get_whale_alerts(
    min_btc: float = Query(default=100.0, ge=1.0, le=100000.0, description="Minimum BTC per output"),
) -> WhaleAlertsResponse:
    """Get large BTC transactions from the most recent blockchain block.

    Scans the latest Bitcoin block for transactions with outputs
    exceeding the min_btc threshold.
    """
    try:
        result = _analyzer.get_whale_transactions(min_btc=min_btc)
    except Exception as e:
        logger.error("Whale alerts fetch failed: %s", e)
        raise HTTPException(status_code=500, detail=f"Whale alerts fetch failed: {e}")

    if "error" in result:
        raise HTTPException(
            status_code=502,
            detail=f"Upstream API error: {result['error']}",
        )

    return WhaleAlertsResponse(
        **result,
        timestamp=datetime.now(timezone.utc).isoformat(),
    )


@router.get("/analysis", response_model=AnalysisResponse)
async def get_analysis(
    symbol: str = Query(default="BTCUSDT", description="Futures trading pair"),
    min_whale_btc: float = Query(default=100.0, ge=1.0, description="Whale detection threshold in BTC"),
) -> AnalysisResponse:
    """Get combined on-chain analysis with bullish/bearish score.

    Combines funding rate sentiment, open interest flow direction,
    and whale transaction activity into a single score (-100 to +100).

    Score interpretation:
    - >= +40: strongly bullish
    - >= +15: bullish
    - -15 to +15: neutral
    - <= -15: bearish
    - <= -40: strongly bearish
    """
    try:
        result = _analyzer.analyze(symbol=symbol, min_whale_btc=min_whale_btc)
    except Exception as e:
        logger.error("On-chain analysis failed: %s", e)
        raise HTTPException(status_code=500, detail=f"On-chain analysis failed: {e}")

    return AnalysisResponse(**result)
