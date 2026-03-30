"""Investor OS — ML Research Service (Tier 3).

Isolated container for experimental models.
Predictions logged as tier=3, no impact on production.
Port 9002.
"""

import time
from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.responses import JSONResponse

_start_time: float = 0.0
_loaded_models: list[str] = []


@asynccontextmanager
async def lifespan(app: FastAPI):
    global _start_time
    _start_time = time.time()
    # Models are loaded on-demand since they're experimental
    yield
    _loaded_models.clear()


app = FastAPI(
    title="Investor OS ML Research",
    version="0.1.0",
    docs_url="/docs",
    lifespan=lifespan,
)


@app.get("/health")
async def health() -> JSONResponse:
    return JSONResponse(
        content={
            "status": "healthy",
            "version": "0.1.0",
            "tier": 3,
            "models_loaded": _loaded_models,
            "gpu_available": False,
            "uptime_seconds": round(time.time() - _start_time, 1),
        }
    )


@app.get("/ready")
async def ready() -> JSONResponse:
    return JSONResponse(content={"ready": True, "models_loaded": len(_loaded_models)})
