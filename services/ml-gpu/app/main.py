"""Investor OS — ML GPU Service (Tier 2).

FastAPI service for GPU-accelerated model inference.
Runs on port 9001, requires NVIDIA Container Toolkit.
Models: Kronos, Chronos-2, TFT, FinGPT.
"""

import logging
import time
from contextlib import asynccontextmanager

import torch
from fastapi import FastAPI
from fastapi.responses import JSONResponse

from app.routers import predict_gpu

logger = logging.getLogger(__name__)

_start_time: float = 0.0
_loaded_models: list[str] = []


def _detect_gpu() -> dict:
    """Detect GPU capabilities."""
    if not torch.cuda.is_available():
        return {"available": False, "device": "cpu"}

    return {
        "available": True,
        "device": torch.cuda.get_device_name(0),
        "vram_total_gb": round(getattr(torch.cuda.get_device_properties(0), 'total_memory', 0) / 1e9, 1),
        "vram_free_gb": round(torch.cuda.mem_get_info()[0] / 1e9, 1),
        "cuda_version": torch.version.cuda or "unknown",
        "torch_version": torch.__version__,
    }


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Startup and shutdown lifecycle."""
    global _start_time
    _start_time = time.time()

    gpu_info = _detect_gpu()
    logger.info("GPU info: %s", gpu_info)

    # Load models that are available
    for model_name, loader in [
        ("kronos", _try_load_kronos),
        ("chronos", _try_load_chronos),
        ("tft", _try_load_tft),
        ("fingpt", _try_load_fingpt),
    ]:
        if loader():
            _loaded_models.append(model_name)

    logger.info("GPU service ready with models: %s", _loaded_models)
    yield
    _loaded_models.clear()


def _try_load_kronos() -> bool:
    try:
        from app.models.kronos_model import get_kronos
        return get_kronos().load()
    except Exception as e:
        logger.warning("Kronos not available: %s", e)
        return False


def _try_load_chronos() -> bool:
    try:
        from app.models.chronos_model import get_chronos
        return get_chronos().load()
    except Exception as e:
        logger.warning("Chronos-2 not available: %s", e)
        return False


def _try_load_tft() -> bool:
    try:
        from app.models.tft_model import get_tft
        return get_tft().load()
    except Exception as e:
        logger.warning("TFT not available: %s", e)
        return False


def _try_load_fingpt() -> bool:
    try:
        from app.models.fingpt_model import get_fingpt
        return get_fingpt().load()
    except Exception as e:
        logger.warning("FinGPT not available: %s", e)
        return False


app = FastAPI(
    title="Investor OS ML GPU Service",
    version="0.1.0",
    docs_url="/docs",
    lifespan=lifespan,
)

app.include_router(predict_gpu.router)


@app.get("/health")
async def health() -> JSONResponse:
    """Health check endpoint."""
    gpu_info = _detect_gpu()
    return JSONResponse(
        content={
            "status": "healthy",
            "version": "0.1.0",
            "tier": 2,
            "models_loaded": _loaded_models,
            "gpu": gpu_info,
            "uptime_seconds": round(time.time() - _start_time, 1),
        }
    )


@app.get("/ready")
async def ready() -> JSONResponse:
    return JSONResponse(
        content={"ready": len(_loaded_models) > 0, "models_loaded": len(_loaded_models)}
    )
