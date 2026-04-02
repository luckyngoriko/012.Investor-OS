"""Cross-asset correlation NATS worker (Wave 2 Task 10).

Subscribes to ios.prices.> — accumulates price data for multiple assets.
When enough data is accumulated (>= MIN_OBSERVATIONS for >= 2 assets),
computes dynamic correlations and publishes to ios.predict.correlation.
"""

import json
import logging
import time
from collections import defaultdict

import numpy as np

from app.models.correlation_model import CorrelationPredictor
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)

# Minimum observations needed before computing correlations
MIN_OBSERVATIONS = 50

# Maximum stored prices per asset (rolling window)
MAX_WINDOW = 500

# How many new prices must arrive before recomputing (avoids excessive computation)
RECOMPUTE_INTERVAL = 10


class PriceAccumulator:
    """Accumulates close prices per asset for correlation computation."""

    def __init__(self):
        self.prices: dict[str, list[float]] = defaultdict(list)
        self._new_count = 0

    def add(self, symbol: str, close: float):
        """Add a close price for a symbol."""
        self.prices[symbol].append(close)
        # Trim to max window
        if len(self.prices[symbol]) > MAX_WINDOW:
            self.prices[symbol] = self.prices[symbol][-MAX_WINDOW:]
        self._new_count += 1

    @property
    def should_recompute(self) -> bool:
        """Check if enough new data has arrived to warrant recomputation."""
        return self._new_count >= RECOMPUTE_INTERVAL

    def reset_counter(self):
        self._new_count = 0

    def get_eligible_returns(self) -> dict[str, list[float]] | None:
        """Get log returns for all assets with enough data.

        Returns None if fewer than 2 assets have enough observations.
        All returned series are trimmed to the same length.
        """
        eligible = {}
        for sym, prices in self.prices.items():
            if len(prices) >= MIN_OBSERVATIONS:
                # Compute log returns
                p = np.array(prices)
                returns = np.diff(np.log(p)).tolist()
                eligible[sym] = returns

        if len(eligible) < 2:
            return None

        # Trim all to the shortest length
        min_len = min(len(r) for r in eligible.values())
        trimmed = {sym: r[-min_len:] for sym, r in eligible.items()}

        return trimmed


async def run(nats: NatsManager):
    """Run correlation worker — subscribe to prices, compute correlations."""
    predictor = CorrelationPredictor()
    accumulator = PriceAccumulator()
    logger.info("Correlation worker started")

    sub = await nats.nc.subscribe("ios.prices.>")

    async for msg in sub.messages:
        envelope = nats.parse_envelope(msg.data)
        if not envelope:
            continue

        symbol = envelope.get("symbol", "")
        data = envelope.get("data", {})
        close = data.get("close")

        if not symbol or close is None:
            continue

        try:
            accumulator.add(symbol, float(close))
        except (ValueError, TypeError):
            continue

        if not accumulator.should_recompute:
            continue

        # Attempt correlation computation
        returns = accumulator.get_eligible_returns()
        if returns is None:
            continue

        try:
            result = predictor.predict(returns=returns, top_n=5)
            accumulator.reset_counter()

            # Publish correlation results
            await nats.publish(
                "ios.predict.correlation",
                ",".join(result["symbols"]),
                "dcc-garch",
                {
                    "model_name": "dcc-garch",
                    "model_version": result["model_version"],
                    "prediction_type": "correlation",
                    "symbols": result["symbols"],
                    "n_assets": result["n_assets"],
                    "n_observations": result["n_observations"],
                    "correlation_matrix": result["correlation_matrix"],
                    "top_correlated": result["top_correlated"],
                    "top_uncorrelated": result["top_uncorrelated"],
                    "confidence": result["confidence"],
                    "latency_ms": result["latency_ms"],
                },
            )

            logger.info(
                "Correlation computed: %d assets, %d obs, confidence=%.2f (%dms)",
                result["n_assets"],
                result["n_observations"],
                result["confidence"],
                result["latency_ms"],
            )

        except Exception as e:
            logger.error("Correlation computation failed: %s", e)
