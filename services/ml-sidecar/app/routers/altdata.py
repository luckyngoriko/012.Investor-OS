"""Alternative Data router (Wave 3 Task 21).

Endpoints:
  GET /v1/altdata/analysis    -- combined alternative data analysis (score + verdict)
  GET /v1/altdata/fear-greed  -- current Crypto Fear & Greed Index
"""

import logging
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel, Field

from app.models.alt_data_model import AltDataCollector

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/altdata", tags=["altdata"])

# Singleton
_collector = AltDataCollector()


# --------------- Response Models ---------------


class FearGreedResponse(BaseModel):
    """Crypto Fear & Greed Index response."""
    value: int = Field(description="0 (Extreme Fear) to 100 (Extreme Greed)")
    classification: str = Field(description="Human-readable classification")
    data_timestamp: int = Field(description="UNIX timestamp of the data point")
    source: str
    timestamp: str


class SignalDetail(BaseModel):
    """Detail for a single alternative data signal."""
    score: float
    weight: float
    source: str
    model_config = {"extra": "allow"}


class AnalysisSignals(BaseModel):
    """Breakdown of individual alternative data signals."""
    fear_greed: dict
    google_trends: dict


class AnalysisResponse(BaseModel):
    """Combined alternative data analysis result."""
    keyword: str
    score: float = Field(description="Combined score from -100 (bearish) to +100 (bullish)")
    verdict: str = Field(description="strongly_bullish | bullish | neutral | bearish | strongly_bearish")
    confidence: float = Field(description="0.0 to 1.0 based on data source availability")
    signals: AnalysisSignals
    latency_ms: int
    model_version: str
    timestamp: str


# --------------- Endpoints ---------------


@router.get("/fear-greed", response_model=FearGreedResponse)
async def get_fear_greed() -> FearGreedResponse:
    """Get the current Crypto Fear & Greed Index.

    Source: alternative.me (free, no API key required).
    0 = Extreme Fear, 100 = Extreme Greed.
    """
    try:
        result = _collector.get_fear_greed_index()
    except Exception as e:
        logger.error("Fear & Greed fetch failed: %s", e)
        raise HTTPException(status_code=500, detail=f"Fear & Greed fetch failed: {e}")

    return FearGreedResponse(
        **result,
        timestamp=datetime.now(timezone.utc).isoformat(),
    )


@router.get("/analysis", response_model=AnalysisResponse)
async def get_analysis(
    keyword: str = Query(default="bitcoin", description="Keyword for Google Trends lookup"),
) -> AnalysisResponse:
    """Get combined alternative data analysis with bullish/bearish score.

    Combines Fear & Greed Index (60% weight) and Google Trends
    interest (40% weight) into a single score (-100 to +100).

    Score interpretation:
    - >= +40: strongly bullish
    - >= +15: bullish
    - -15 to +15: neutral
    - <= -15: bearish
    - <= -40: strongly bearish
    """
    try:
        result = _collector.analyze(keyword=keyword)
    except Exception as e:
        logger.error("Alternative data analysis failed: %s", e)
        raise HTTPException(status_code=500, detail=f"Alternative data analysis failed: {e}")

    return AnalysisResponse(**result)
