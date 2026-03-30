"""GPU prediction router (Sprints 128-131).

Unified router for all Tier 2 GPU models.
Each model endpoint delegates to its singleton predictor.
"""

import logging
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/predict", tags=["predict-gpu"])


# ── Shared types ──────────────────────────────────────────────────────

class TimeSeriesRequest(BaseModel):
    """Request for time series prediction models (Kronos, Chronos, TFT)."""
    symbol: str
    data: list[float] = Field(..., min_length=10, description="Historical price or return series")
    horizon: int = Field(default=12, ge=1, le=96)


class TimeSeriesResponse(BaseModel):
    """Standard time series prediction response."""
    model: str
    model_version: str
    prediction: dict
    confidence: float
    latency_ms: int
    timestamp: str


class TextRequest(BaseModel):
    """Request for text analysis models (FinGPT)."""
    texts: list[str] = Field(..., min_length=1, max_length=8)
    task: str = Field(default="sentiment", description="sentiment, summarize, entities, qa")


class TextResponse(BaseModel):
    """Text analysis response."""
    model: str
    model_version: str
    results: list[dict]
    latency_ms: int
    timestamp: str


# ── Kronos (Sprint 128) ──────────────────────────────────────────────

@router.post("/kronos", response_model=TimeSeriesResponse)
async def predict_kronos(body: TimeSeriesRequest) -> TimeSeriesResponse:
    """Predict next K-line candles using Kronos financial foundation model."""
    try:
        from app.models.kronos_model import get_kronos
        model = get_kronos()
        if not model.is_loaded:
            raise HTTPException(status_code=503, detail="Kronos model not loaded")
        result = model.predict(body.data, body.horizon)
    except ImportError:
        raise HTTPException(status_code=503, detail="Kronos not available")
    except HTTPException:
        raise
    except Exception as e:
        logger.error("Kronos prediction failed: %s", e)
        raise HTTPException(status_code=500, detail=str(e))

    return TimeSeriesResponse(
        model="kronos",
        model_version=result["model_version"],
        prediction={"symbol": body.symbol, "forecasts": result["forecasts"], "horizon": body.horizon},
        confidence=result["confidence"],
        latency_ms=result["latency_ms"],
        timestamp=datetime.now(timezone.utc).isoformat(),
    )


# ── Chronos-2 (Sprint 129) ───────────────────────────────────────────

@router.post("/chronos", response_model=TimeSeriesResponse)
async def predict_chronos(body: TimeSeriesRequest) -> TimeSeriesResponse:
    """Zero-shot time series forecasting using Chronos-2."""
    try:
        from app.models.chronos_model import get_chronos
        model = get_chronos()
        if not model.is_loaded:
            raise HTTPException(status_code=503, detail="Chronos-2 model not loaded")
        result = model.predict(body.data, body.horizon)
    except ImportError:
        raise HTTPException(status_code=503, detail="Chronos-2 not available")
    except HTTPException:
        raise
    except Exception as e:
        logger.error("Chronos-2 prediction failed: %s", e)
        raise HTTPException(status_code=500, detail=str(e))

    return TimeSeriesResponse(
        model="chronos",
        model_version=result["model_version"],
        prediction={"symbol": body.symbol, "forecasts": result["forecasts"], "horizon": body.horizon},
        confidence=result["confidence"],
        latency_ms=result["latency_ms"],
        timestamp=datetime.now(timezone.utc).isoformat(),
    )


# ── TFT (Sprint 130) ─────────────────────────────────────────────────

@router.post("/tft", response_model=TimeSeriesResponse)
async def predict_tft(body: TimeSeriesRequest) -> TimeSeriesResponse:
    """Multi-horizon interpretable forecasting using Temporal Fusion Transformer."""
    try:
        from app.models.tft_model import get_tft
        model = get_tft()
        if not model.is_loaded:
            raise HTTPException(status_code=503, detail="TFT model not loaded")
        result = model.predict(body.data, body.horizon)
    except ImportError:
        raise HTTPException(status_code=503, detail="TFT not available")
    except HTTPException:
        raise
    except Exception as e:
        logger.error("TFT prediction failed: %s", e)
        raise HTTPException(status_code=500, detail=str(e))

    return TimeSeriesResponse(
        model="tft",
        model_version=result["model_version"],
        prediction={
            "symbol": body.symbol,
            "forecasts": result["forecasts"],
            "horizon": body.horizon,
            "variable_importance": result.get("variable_importance", {}),
        },
        confidence=result["confidence"],
        latency_ms=result["latency_ms"],
        timestamp=datetime.now(timezone.utc).isoformat(),
    )


# ── FinGPT (Sprint 131) ──────────────────────────────────────────────

@router.post("/fingpt", response_model=TextResponse)
async def predict_fingpt(body: TextRequest) -> TextResponse:
    """Advanced financial text analysis using FinGPT with QLoRA."""
    try:
        from app.models.fingpt_model import get_fingpt
        model = get_fingpt()
        if not model.is_loaded:
            raise HTTPException(status_code=503, detail="FinGPT model not loaded")
        results, latency_ms = model.predict(body.texts, task=body.task)
    except ImportError:
        raise HTTPException(status_code=503, detail="FinGPT not available")
    except HTTPException:
        raise
    except Exception as e:
        logger.error("FinGPT prediction failed: %s", e)
        raise HTTPException(status_code=500, detail=str(e))

    return TextResponse(
        model="fingpt",
        model_version=model.model_version,
        results=results,
        latency_ms=latency_ms,
        timestamp=datetime.now(timezone.utc).isoformat(),
    )
