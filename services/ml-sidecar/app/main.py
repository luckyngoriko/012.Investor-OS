"""Investor OS — ML Sidecar Service.

FastAPI service providing ML model inference for the Rust backend.
Runs on port 9000, communicates over ios-net Docker bridge.
"""

import logging
import os
import time
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.responses import JSONResponse

from app.config import settings
from app.routers import broker, optimize, predict, sentiment, volatility

logger = logging.getLogger(__name__)

# Track startup time for uptime calculation
_start_time: float = 0.0

# Registry of loaded models (populated during startup)
_loaded_models: list[str] = []


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Startup and shutdown lifecycle."""
    global _start_time
    _start_time = time.time()

    # Try to load CatBoost model
    try:
        from app.models.catboost_model import CatBoostPredictor

        cb = CatBoostPredictor()
        if cb.load("latest"):
            _loaded_models.append("catboost")
            # Store in predict router singleton
            predict._catboost = cb
            logger.info("CatBoost model loaded at startup")
        else:
            logger.warning("No CatBoost model found — prediction will return 503 until trained")
    except Exception as e:
        logger.warning("CatBoost not available: %s", e)

    # Try to load FinBERT
    try:
        from app.models.finbert_model import get_finbert

        fb = get_finbert()
        if fb.load():
            _loaded_models.append("finbert")
            logger.info("FinBERT model loaded at startup")
        else:
            logger.warning("FinBERT not available — will try on first request")
    except Exception as e:
        logger.warning("FinBERT not available: %s", e)

    # GARCH doesn't need preloading (stateless — fits per request)
    _loaded_models.append("garch")

    # Start NATS workers if enabled
    nats_state = None
    nats_enabled = os.environ.get("NATS_ENABLED", "false").lower() == "true"
    if nats_enabled:
        try:
            from app.workers.worker_runner import start_nats_workers
            nats_state = await start_nats_workers()
            if nats_state:
                logger.info("NATS workers active")
        except Exception as e:
            logger.warning("NATS workers failed to start: %s", e)

    yield

    # Cleanup
    if nats_state:
        from app.workers.worker_runner import stop_nats_workers
        nats_mgr, tasks = nats_state
        await stop_nats_workers(nats_mgr, tasks)
    _loaded_models.clear()


app = FastAPI(
    title="Investor OS ML Sidecar",
    version=settings.app_version,
    docs_url="/docs",
    lifespan=lifespan,
)

# Include model routers
app.include_router(predict.router)
app.include_router(volatility.router)
app.include_router(sentiment.router)
app.include_router(optimize.router)
app.include_router(broker.router)


@app.get("/health")
async def health() -> JSONResponse:
    """Health check endpoint for Docker and Rust backend."""
    return JSONResponse(
        content={
            "status": "healthy",
            "version": settings.app_version,
            "models_loaded": _loaded_models,
            "gpu_available": False,
            "uptime_seconds": round(time.time() - _start_time, 1),
        }
    )


@app.get("/ready")
async def ready() -> JSONResponse:
    """Readiness check — returns 200 only when models are loaded and DB is reachable."""
    # For now, always ready (no models to load yet)
    return JSONResponse(
        content={
            "ready": True,
            "models_loaded": len(_loaded_models),
        }
    )


@app.get("/info")
async def info() -> JSONResponse:
    """Service metadata for the Rust backend discovery."""
    return JSONResponse(
        content={
            "service": settings.app_name,
            "version": settings.app_version,
            "tier": 1,
            "endpoints": {
                "health": "/health",
                "ready": "/ready",
                "predict": "/v1/predict/{model}",
                "optimize": "/v1/optimize/{method}",
            },
            "models_available": _loaded_models,
        }
    )
