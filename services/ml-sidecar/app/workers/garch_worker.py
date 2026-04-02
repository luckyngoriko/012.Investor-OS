"""GARCH NATS worker (Sprint N3).

Subscribes to ios.prices.> — when a new candle arrives, fits GARCH(1,1)
and publishes volatility + VaR to ios.predict.garch.{symbol}.
"""

import json
import logging

import numpy as np
import psycopg2

from app.models.garch_model import GarchPredictor
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)

DB_URL = "postgresql://investor:trjkNPtO1ykTKxrF1hMosUKvQGBp7c@postgres:5432/investor_os"


async def run(nats: NatsManager):
    """Run GARCH worker — subscribe to prices, forecast volatility."""
    garch = GarchPredictor()
    logger.info("GARCH worker started")

    sub = await nats.nc.subscribe("ios.prices.>")

    async for msg in sub.messages:
        envelope = nats.parse_envelope(msg.data)
        if not envelope:
            continue

        symbol = envelope["symbol"]

        try:
            # Fetch returns from DB
            conn = psycopg2.connect(DB_URL)
            cur = conn.cursor()
            cur.execute(
                "SELECT close FROM prices WHERE symbol=%s ORDER BY time ASC",
                (symbol,),
            )
            closes = [r[0] for r in cur.fetchall()]
            cur.close()
            conn.close()

            if len(closes) < 50:
                continue

            returns = [np.log(closes[i] / closes[i - 1]) for i in range(1, len(closes))]
            result = garch.predict(returns, horizon_days=1)

            # Publish
            await nats.publish(
                f"ios.predict.garch.{symbol}",
                symbol,
                "garch",
                {
                    "model_name": "garch",
                    "model_version": result["model_version"],
                    "prediction_type": "volatility",
                    "volatility": result["forecasted_volatility_daily"],
                    "var_95": result["var_95"],
                    "var_99": result["var_99"],
                    "confidence": result["confidence"],
                    "latency_ms": result["latency_ms"],
                },
            )

            logger.info(
                "GARCH %s: vol=%.2f%% VaR95=%.2f%% (%dms)",
                symbol,
                result["forecasted_volatility_daily"] * 100,
                result["var_95"] * 100,
                result["latency_ms"],
            )

        except Exception as e:
            logger.error("GARCH failed for %s: %s", symbol, e)
