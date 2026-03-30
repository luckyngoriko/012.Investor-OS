"""Prediction router for ML models (Sprint 119).

Endpoints:
  POST /v1/predict/catboost — CatBoost return prediction
"""

import logging
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from app.features.builder import build_features_from_request
from app.models.catboost_model import CatBoostPredictor

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/predict", tags=["predict"])

# Singleton model instance — loaded on first request or at startup
_catboost: CatBoostPredictor | None = None


def get_catboost() -> CatBoostPredictor:
    """Get or initialize the CatBoost model singleton."""
    global _catboost
    if _catboost is None:
        _catboost = CatBoostPredictor()
        _catboost.load("latest")
    return _catboost


class PredictRequest(BaseModel):
    """Request body for prediction endpoints."""

    model: str = "catboost"
    symbol: str
    features: dict = Field(default_factory=dict)
    horizon: str | None = None


class PredictResponse(BaseModel):
    """Standard prediction response."""

    model: str
    model_version: str
    prediction: dict
    confidence: float
    latency_ms: int
    timestamp: str


@router.post("/catboost", response_model=PredictResponse)
async def predict_catboost(body: PredictRequest) -> PredictResponse:
    """Generate a return prediction using CatBoost."""
    cb = get_catboost()

    if not cb.is_loaded:
        raise HTTPException(
            status_code=503,
            detail="CatBoost model not loaded. Train and deploy a model first.",
        )

    features = build_features_from_request(body.features)

    try:
        result = cb.predict(features)
    except Exception as e:
        logger.error("CatBoost prediction failed: %s", e)
        raise HTTPException(status_code=500, detail=f"Prediction failed: {e}")

    return PredictResponse(
        model="catboost",
        model_version=result["model_version"],
        prediction={
            "direction": result["direction"],
            "return_pct": result["return_pct"],
            "symbol": body.symbol,
            "horizon": body.horizon or "1d",
        },
        confidence=result["confidence"],
        latency_ms=result["latency_ms"],
        timestamp=datetime.now(timezone.utc).isoformat(),
    )
