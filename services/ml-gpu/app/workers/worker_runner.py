"""GPU NATS worker coordinator (Sprint N3)."""

import asyncio
import logging

from app.nats_client import NatsManager
from app.workers import chronos_worker

logger = logging.getLogger(__name__)


async def start_nats_workers():
    nats = NatsManager()
    if not await nats.connect():
        logger.warning("NATS unavailable — GPU workers not started")
        return None

    tasks = [
        asyncio.create_task(chronos_worker.run(nats), name="chronos-worker"),
    ]

    logger.info("Started %d GPU NATS workers", len(tasks))
    return nats, tasks


async def stop_nats_workers(nats, tasks):
    if tasks:
        for task in tasks:
            task.cancel()
        await asyncio.gather(*tasks, return_exceptions=True)
    if nats:
        await nats.close()
