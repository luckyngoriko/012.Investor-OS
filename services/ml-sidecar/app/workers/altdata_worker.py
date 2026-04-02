"""Alternative Data NATS worker (Wave 3 Task 21).

Timer-based worker that runs every 30 minutes.
Fetches alternative data signals (Fear & Greed, Google Trends)
and publishes combined analysis to ios.altdata.{symbol}
for downstream consumers.
"""

import asyncio
import logging

from app.models.alt_data_model import AltDataCollector
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)

# Polling interval in seconds (30 minutes)
POLL_INTERVAL = 1800

# Keywords to track (mapped from trading symbols)
DEFAULT_KEYWORDS = [
    ("BTCUSDT", "bitcoin"),
    ("ETHUSDT", "ethereum"),
]


async def run(nats: NatsManager):
    """Run alternative data worker on a 30-minute timer."""
    collector = AltDataCollector()
    logger.info(
        "Alt-data worker started (interval=%ds, keywords=%s)",
        POLL_INTERVAL,
        [k for _, k in DEFAULT_KEYWORDS],
    )

    while True:
        for symbol, keyword in DEFAULT_KEYWORDS:
            try:
                result = collector.analyze(keyword=keyword)

                subject = f"ios.altdata.{symbol}"
                await nats.publish(
                    subject,
                    symbol,
                    "altdata-collector",
                    {
                        "model_name": "altdata-collector",
                        "model_version": result["model_version"],
                        "prediction_type": "altdata_analysis",
                        "keyword": result["keyword"],
                        "symbol": symbol,
                        "score": result["score"],
                        "verdict": result["verdict"],
                        "confidence": result["confidence"],
                        "signals": result["signals"],
                        "latency_ms": result["latency_ms"],
                    },
                )

                logger.info(
                    "Alt-data analysis published: %s (%s) score=%.1f verdict=%s confidence=%.2f (%dms)",
                    symbol,
                    keyword,
                    result["score"],
                    result["verdict"],
                    result["confidence"],
                    result["latency_ms"],
                )

            except Exception as e:
                logger.error("Alt-data analysis failed for %s (%s): %s", symbol, keyword, e)

        # Wait for next cycle
        await asyncio.sleep(POLL_INTERVAL)
