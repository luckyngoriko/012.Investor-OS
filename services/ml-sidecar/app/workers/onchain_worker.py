"""On-chain analytics NATS worker (Wave 2 Task 11).

Timer-based worker that runs every 5 minutes.
Fetches on-chain signals and publishes combined analysis
to ios.onchain.{symbol} for downstream consumers.
"""

import asyncio
import logging

from app.models.onchain_model import OnChainAnalyzer
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)

# Polling interval in seconds (5 minutes)
POLL_INTERVAL = 300

# Symbols to track
DEFAULT_SYMBOLS = ["BTCUSDT", "ETHUSDT"]


async def run(nats: NatsManager):
    """Run on-chain analytics worker on a 5-minute timer."""
    analyzer = OnChainAnalyzer()
    logger.info(
        "On-chain worker started (interval=%ds, symbols=%s)",
        POLL_INTERVAL,
        DEFAULT_SYMBOLS,
    )

    while True:
        for symbol in DEFAULT_SYMBOLS:
            try:
                result = analyzer.analyze(symbol=symbol)

                subject = f"ios.onchain.{symbol}"
                await nats.publish(
                    subject,
                    symbol,
                    "onchain-analyzer",
                    {
                        "model_name": "onchain-analyzer",
                        "model_version": result["model_version"],
                        "prediction_type": "onchain_analysis",
                        "symbol": result["symbol"],
                        "score": result["score"],
                        "verdict": result["verdict"],
                        "confidence": result["confidence"],
                        "signals": result["signals"],
                        "latency_ms": result["latency_ms"],
                    },
                )

                logger.info(
                    "On-chain analysis published: %s score=%.1f verdict=%s confidence=%.2f (%dms)",
                    symbol,
                    result["score"],
                    result["verdict"],
                    result["confidence"],
                    result["latency_ms"],
                )

            except Exception as e:
                logger.error("On-chain analysis failed for %s: %s", symbol, e)

        # Wait for next cycle
        await asyncio.sleep(POLL_INTERVAL)
