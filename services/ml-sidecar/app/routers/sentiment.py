"""Sentiment analysis router (Sprint 121).

Endpoints:
  POST /v1/predict/finbert — FinBERT financial sentiment analysis
"""

import logging
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from app.models.finbert_model import get_finbert

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/predict", tags=["predict"])


class SentimentRequest(BaseModel):
    """Request body for sentiment analysis."""

    texts: list[str] = Field(..., min_length=1, max_length=16)


class SentimentResult(BaseModel):
    """Single text sentiment result."""

    text: str
    sentiment: str
    score: float
    confidence: float
    scores: dict[str, float]


class SentimentResponse(BaseModel):
    """Batch sentiment response."""

    model: str = "finbert"
    model_version: str
    results: list[SentimentResult]
    latency_ms: int
    timestamp: str


@router.post("/finbert", response_model=SentimentResponse)
async def predict_finbert(body: SentimentRequest) -> SentimentResponse:
    """Analyze financial sentiment for a batch of texts."""
    fb = get_finbert()

    if not fb.is_loaded:
        # Try loading on first request
        if not fb.load():
            raise HTTPException(
                status_code=503,
                detail="FinBERT model not available. Install transformers and download ProsusAI/finbert.",
            )

    try:
        results, latency_ms = fb.predict(body.texts)
    except Exception as e:
        logger.error("FinBERT prediction failed: %s", e)
        raise HTTPException(status_code=500, detail=f"Sentiment analysis failed: {e}")

    return SentimentResponse(
        model="finbert",
        model_version=fb.model_version,
        results=[SentimentResult(**r) for r in results],
        latency_ms=latency_ms,
        timestamp=datetime.now(timezone.utc).isoformat(),
    )
