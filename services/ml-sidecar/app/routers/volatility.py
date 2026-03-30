"""Volatility prediction router (Sprint 120).

Endpoints:
  POST /v1/predict/garch — GARCH(1,1) volatility forecast + VaR/CVaR
"""

import logging
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from app.models.garch_model import GarchPredictor

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/predict", tags=["predict"])

# Singleton
_garch = GarchPredictor()


class GarchRequest(BaseModel):
    """Request body for GARCH volatility prediction."""

    symbol: str
    returns: list[float] = Field(..., min_length=30, description="Historical daily log returns")
    horizon_days: int = Field(default=1, ge=1, le=90)
    confidence_levels: list[float] = Field(default=[0.95, 0.99])


class GarchResponse(BaseModel):
    """GARCH prediction response."""

    model: str = "garch"
    model_version: str
    prediction: dict
    confidence: float
    latency_ms: int
    timestamp: str


@router.post("/garch", response_model=GarchResponse)
async def predict_garch(body: GarchRequest) -> GarchResponse:
    """Forecast volatility and compute VaR/CVaR using GARCH(1,1)."""
    try:
        result = _garch.predict(
            returns=body.returns,
            horizon_days=body.horizon_days,
            confidence_levels=body.confidence_levels,
        )
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except Exception as e:
        logger.error("GARCH prediction failed: %s", e)
        raise HTTPException(status_code=500, detail=f"GARCH prediction failed: {e}")

    return GarchResponse(
        model="garch",
        model_version=result["model_version"],
        prediction={
            "symbol": body.symbol,
            "forecasted_volatility_daily": result["forecasted_volatility_daily"],
            "forecasted_volatility_ann": result["forecasted_volatility_ann"],
            "current_volatility_daily": result["current_volatility_daily"],
            "horizon_days": result["horizon_days"],
            "var_95": result.get("var_95"),
            "var_99": result.get("var_99"),
            "cvar_95": result.get("cvar_95"),
            "cvar_99": result.get("cvar_99"),
            "persistence": result["persistence"],
            "garch_params": result["garch_params"],
            "n_observations": result["n_observations"],
        },
        confidence=result["confidence"],
        latency_ms=result["latency_ms"],
        timestamp=datetime.now(timezone.utc).isoformat(),
    )
