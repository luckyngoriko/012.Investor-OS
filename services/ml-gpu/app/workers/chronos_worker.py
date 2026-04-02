"""Chronos-2 NATS worker (Sprint N3).

Subscribes to ios.prices.> — when a new candle arrives, runs Chronos-2
GPU forecasting and publishes to ios.predict.chronos.{symbol}.
"""

import logging

import psycopg2
import numpy as np

from app.models.chronos_model import get_chronos
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)

DB_URL = "postgresql://investor:trjkNPtO1ykTKxrF1hMosUKvQGBp7c@postgres:5432/investor_os"


async def run(nats: NatsManager):
    """Run Chronos-2 GPU worker — subscribe to prices, forecast."""
    chronos = get_chronos()
    if not chronos.is_loaded:
        if not chronos.load():
            logger.warning("Chronos-2 not loaded — worker inactive")
            return

    logger.info("Chronos-2 GPU worker started (device=%s)", chronos.device)

    sub = await nats.nc.subscribe("ios.prices.>")

    async for msg in sub.messages:
        envelope = nats.parse_envelope(msg.data)
        if not envelope:
            continue

        symbol = envelope["symbol"]

        try:
            # Fetch recent prices from DB for context
            conn = psycopg2.connect(DB_URL)
            cur = conn.cursor()
            cur.execute(
                "SELECT close FROM prices WHERE symbol=%s ORDER BY time DESC LIMIT 96",
                (symbol,),
            )
            closes = [r[0] for r in cur.fetchall()][::-1]  # chronological
            cur.close()
            conn.close()

            if len(closes) < 24:
                continue

            result = chronos.predict(closes, horizon=12)

            # Derive direction from forecast
            last_price = closes[-1]
            forecast_avg = np.mean(result["forecasts"][:6])
            direction = "long" if forecast_avg > last_price else "short" if forecast_avg < last_price else "neutral"
            predicted_return = (forecast_avg - last_price) / last_price

            await nats.publish(
                f"ios.predict.chronos.{symbol}",
                symbol,
                "chronos",
                {
                    "model_name": "chronos",
                    "model_version": result["model_version"],
                    "prediction_type": "return",
                    "direction": direction,
                    "predicted_return": round(predicted_return, 6),
                    "forecasts": result["forecasts"],
                    "confidence": result["confidence"],
                    "latency_ms": result["latency_ms"],
                },
            )

            logger.info(
                "Chronos %s: %s %.3f%% (%dms, GPU)",
                symbol, direction, predicted_return * 100, result["latency_ms"],
            )

        except Exception as e:
            logger.error("Chronos failed for %s: %s", symbol, e)
