"""Cross-asset correlation router (Wave 2 Task 10).

Endpoints:
  POST /v1/predict/correlation — DCC-GARCH dynamic correlation matrix
"""

import logging
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from app.models.correlation_model import CorrelationPredictor

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/predict", tags=["predict"])

# Singleton
_correlation = CorrelationPredictor()


class CorrelationRequest(BaseModel):
    """Request body for cross-asset correlation prediction."""

    symbols: list[str] = Field(
        ...,
        min_length=2,
        description="Asset symbols (e.g. ['BTC', 'ETH', 'SPY'])",
    )
    returns: dict[str, list[float]] = Field(
        ...,
        description="Dict mapping each symbol to its daily log returns series",
    )
    lookback_period: int = Field(
        default=90,
        ge=30,
        le=1000,
        description="Number of observations to use (all returns must be this length)",
    )
    decay_factor: float = Field(
        default=0.94,
        gt=0.0,
        lt=1.0,
        description="EWMA decay factor (higher = more weight on older data)",
    )
    top_n: int = Field(
        default=5,
        ge=1,
        le=20,
        description="Number of top correlated/uncorrelated pairs to return",
    )


class CorrelationPair(BaseModel):
    """A correlated/uncorrelated asset pair."""

    pair: str
    asset_a: str
    asset_b: str
    correlation: float


class CorrelationResponse(BaseModel):
    """DCC-GARCH correlation response."""

    model: str = "dcc-garch"
    model_version: str
    symbols: list[str]
    n_assets: int
    n_observations: int
    correlation_matrix: dict[str, dict[str, float]]
    top_correlated: list[CorrelationPair]
    top_uncorrelated: list[CorrelationPair]
    garch_diagnostics: dict[str, dict]
    confidence: float
    latency_ms: int
    timestamp: str


@router.post("/correlation", response_model=CorrelationResponse)
async def predict_correlation(body: CorrelationRequest) -> CorrelationResponse:
    """Compute dynamic cross-asset correlation using DCC-GARCH."""
    # Validate that returns dict matches symbols list
    missing = [s for s in body.symbols if s not in body.returns]
    if missing:
        raise HTTPException(
            status_code=400,
            detail=f"Missing return series for symbols: {missing}",
        )

    extra = [s for s in body.returns if s not in body.symbols]
    if extra:
        raise HTTPException(
            status_code=400,
            detail=f"Extra return series not in symbols list: {extra}",
        )

    # Filter returns to only requested symbols
    filtered_returns = {s: body.returns[s] for s in body.symbols}

    try:
        result = _correlation.predict(
            returns=filtered_returns,
            decay=body.decay_factor,
            top_n=body.top_n,
        )
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except Exception as e:
        logger.error("Correlation prediction failed: %s", e)
        raise HTTPException(
            status_code=500, detail=f"Correlation prediction failed: {e}"
        )

    return CorrelationResponse(
        model="dcc-garch",
        model_version=result["model_version"],
        symbols=result["symbols"],
        n_assets=result["n_assets"],
        n_observations=result["n_observations"],
        correlation_matrix=result["correlation_matrix"],
        top_correlated=[CorrelationPair(**p) for p in result["top_correlated"]],
        top_uncorrelated=[CorrelationPair(**p) for p in result["top_uncorrelated"]],
        garch_diagnostics=result["garch_diagnostics"],
        confidence=result["confidence"],
        latency_ms=result["latency_ms"],
        timestamp=datetime.now(timezone.utc).isoformat(),
    )
