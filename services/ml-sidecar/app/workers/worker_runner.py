"""NATS worker coordinator (Sprint N3).

Starts all ML model workers as concurrent tasks.
Called from main.py lifespan if NATS is enabled.
"""

import asyncio
import logging

from app.nats_client import NatsManager
from app.workers import altdata_worker, catboost_worker, correlation_worker, garch_worker, finbert_worker, onchain_worker

logger = logging.getLogger(__name__)


async def start_nats_workers():
    """Connect to NATS and start all model workers."""
    nats = NatsManager()
    if not await nats.connect():
        logger.warning("NATS unavailable — workers not started, REST-only mode")
        return None

    logger.info("Starting NATS model workers...")

    tasks = [
        asyncio.create_task(catboost_worker.run(nats), name="catboost-worker"),
        asyncio.create_task(correlation_worker.run(nats), name="correlation-worker"),
        asyncio.create_task(garch_worker.run(nats), name="garch-worker"),
        asyncio.create_task(finbert_worker.run(nats), name="finbert-worker"),
        asyncio.create_task(onchain_worker.run(nats), name="onchain-worker"),
        asyncio.create_task(altdata_worker.run(nats), name="altdata-worker"),
    ]

    logger.info("Started %d NATS workers", len(tasks))
    return nats, tasks


async def stop_nats_workers(nats, tasks):
    """Gracefully stop all workers."""
    if tasks:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
        logger.info("All NATS workers stopped")

    if nats:
        await nats.close()
