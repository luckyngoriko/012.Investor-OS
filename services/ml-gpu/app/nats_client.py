"""Shared NATS client for ML services (Sprint N3).

Connects to ios-nats and provides pub/sub with JSON envelope
matching the Rust NatsEnvelope<T> format.
"""

import json
import logging
import os
from datetime import datetime, timezone

import nats
from nats.js.api import StreamConfig, RetentionPolicy

logger = logging.getLogger(__name__)

NATS_URL = os.environ.get("NATS_URL", "nats://nats:4222")


class NatsManager:
    """NATS connection manager with JetStream support."""

    def __init__(self, url: str = NATS_URL):
        self.url = url
        self.nc = None
        self.js = None

    async def connect(self) -> bool:
        try:
            self.nc = await nats.connect(self.url)
            self.js = self.nc.jetstream()
            logger.info("NATS connected to %s", self.url)
            return True
        except Exception as e:
            logger.warning("NATS connection failed: %s", e)
            return False

    async def close(self):
        if self.nc:
            await self.nc.close()
            logger.info("NATS disconnected")

    @property
    def is_connected(self) -> bool:
        return self.nc is not None and self.nc.is_connected

    async def publish(self, subject: str, symbol: str, source: str, data: dict):
        """Publish a message in NatsEnvelope format."""
        envelope = {
            "symbol": symbol,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "source": source,
            "version": "v1",
            "data": data,
        }
        payload = json.dumps(envelope).encode()
        await self.nc.publish(subject, payload)
        logger.debug("Published to %s (%d bytes)", subject, len(payload))

    async def subscribe(self, subject: str, callback, durable: str | None = None):
        """Subscribe to a subject with optional durable consumer."""
        if durable and self.js:
            try:
                sub = await self.js.subscribe(subject, durable=durable)
                logger.info("JetStream subscribed to %s (durable=%s)", subject, durable)
                return sub
            except Exception as e:
                logger.warning("JetStream subscribe failed, falling back to core: %s", e)

        sub = await self.nc.subscribe(subject, cb=callback)
        logger.info("Core NATS subscribed to %s", subject)
        return sub

    @staticmethod
    def parse_envelope(data: bytes) -> dict | None:
        """Parse a NatsEnvelope JSON message."""
        try:
            return json.loads(data)
        except Exception:
            return None
