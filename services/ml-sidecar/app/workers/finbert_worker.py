"""FinBERT NATS worker (Sprint N3).

Subscribes to ios.news.> — when news arrives, runs sentiment analysis
and publishes to ios.sentiment.{symbol}.

Also runs periodically on Binance 24h price change as sentiment proxy.
"""

import logging

from app.models.finbert_model import get_finbert
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)


async def run(nats: NatsManager):
    """Run FinBERT worker — subscribe to news, analyze sentiment."""
    fb = get_finbert()
    if not fb.is_loaded:
        if not fb.load():
            logger.warning("FinBERT not loaded — worker using keyword fallback")

    logger.info("FinBERT worker started")

    sub = await nats.nc.subscribe("ios.news.>")

    async for msg in sub.messages:
        envelope = nats.parse_envelope(msg.data)
        if not envelope:
            continue

        symbol = envelope["symbol"]
        texts = envelope["data"].get("texts", [])

        if not texts:
            continue

        try:
            if fb.is_loaded:
                results, latency_ms = fb.predict(texts)
                scores = [r.get("score", 0) for r in results]
                avg_score = sum(scores) / len(scores) if scores else 0
            else:
                # Keyword fallback
                positive = {"surge", "bull", "rally", "gain", "rise", "high", "up", "growth", "bullish"}
                negative = {"crash", "bear", "fall", "drop", "low", "down", "loss", "bearish", "dump"}
                scores = []
                for text in texts:
                    words = set(text.lower().split())
                    pos = len(words & positive)
                    neg = len(words & negative)
                    scores.append((pos - neg) / max(pos + neg, 1))
                avg_score = sum(scores) / len(scores) if scores else 0
                latency_ms = 0

            await nats.publish(
                f"ios.sentiment.{symbol}",
                symbol,
                "finbert",
                {
                    "avg_sentiment_score": round(avg_score, 4),
                    "n_texts": len(texts),
                    "confidence": 0.7 if fb.is_loaded else 0.3,
                    "latency_ms": latency_ms,
                    "method": "finbert" if fb.is_loaded else "keyword_fallback",
                },
            )

            logger.info("FinBERT %s: sentiment=%.3f (%d texts)", symbol, avg_score, len(texts))

        except Exception as e:
            logger.error("FinBERT failed for %s: %s", symbol, e)
