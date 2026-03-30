"""Portfolio optimization router (Sprint 122).

Endpoints:
  POST /v1/optimize/skfolio — Portfolio optimization via HRP/CVaR/MeanVariance
"""

import logging
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

from app.models.portfolio_optimizer import SUPPORTED_METHODS, PortfolioOptimizer

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/optimize", tags=["optimize"])

_optimizer = PortfolioOptimizer()


class OptimizeRequest(BaseModel):
    """Request body for portfolio optimization."""

    method: str = Field(default="hrp", description=f"One of: {SUPPORTED_METHODS}")
    symbols: list[str] = Field(..., min_length=2)
    returns_data: list[list[float]] = Field(..., min_length=10)
    constraints: dict = Field(default_factory=lambda: {"max_weight": 0.4, "min_weight": 0.0})


class OptimizeResponse(BaseModel):
    """Portfolio optimization response."""

    model: str = "skfolio"
    model_version: str
    weights: dict[str, float]
    expected_return: float
    expected_risk: float
    sharpe_ratio: float
    concentration_hhi: float
    method: str
    latency_ms: int
    timestamp: str


@router.post("/skfolio", response_model=OptimizeResponse)
async def optimize_portfolio(body: OptimizeRequest) -> OptimizeResponse:
    """Optimize portfolio allocation using selected method."""
    max_w = body.constraints.get("max_weight", 0.4)
    min_w = body.constraints.get("min_weight", 0.0)

    try:
        result = _optimizer.optimize(
            symbols=body.symbols,
            returns_matrix=body.returns_data,
            method=body.method,
            max_weight=max_w,
            min_weight=min_w,
        )
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))
    except Exception as e:
        logger.error("Portfolio optimization failed: %s", e)
        raise HTTPException(status_code=500, detail=f"Optimization failed: {e}")

    return OptimizeResponse(
        model="skfolio",
        model_version=result["model_version"],
        weights=result["weights"],
        expected_return=result["expected_return"],
        expected_risk=result["expected_risk"],
        sharpe_ratio=result["sharpe_ratio"],
        concentration_hhi=result["concentration_hhi"],
        method=result["method"],
        latency_ms=result["latency_ms"],
        timestamp=datetime.now(timezone.utc).isoformat(),
    )
