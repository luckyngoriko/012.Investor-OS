"""CatBoost NATS worker (Sprint N3).

Subscribes to ios.features.> — when features arrive, runs CatBoost prediction
and publishes result to ios.predict.catboost.{symbol}.
"""

import json
import logging

import psycopg2

from app.models.catboost_model import CatBoostPredictor
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)

DB_URL = "postgresql://investor:trjkNPtO1ykTKxrF1hMosUKvQGBp7c@postgres:5432/investor_os"


async def run(nats: NatsManager):
    """Run CatBoost worker — subscribe to features, predict, publish."""
    cb = CatBoostPredictor(model_path="/app/logs")
    if not cb.load("latest"):
        logger.warning("CatBoost model not loaded — worker inactive")
        return

    logger.info("CatBoost worker started")

    sub = await nats.nc.subscribe("ios.features.>")

    async for msg in sub.messages:
        envelope = nats.parse_envelope(msg.data)
        if not envelope:
            continue

        symbol = envelope["symbol"]
        features = envelope["data"]

        try:
            result = cb.predict(features)

            # Publish prediction
            await nats.publish(
                f"ios.predict.catboost.{symbol}",
                symbol,
                "catboost",
                {
                    "model_name": "catboost",
                    "model_version": result["model_version"],
                    "prediction_type": "return",
                    "direction": result["direction"],
                    "predicted_return": result["return_pct"],
                    "confidence": result["confidence"],
                    "latency_ms": result["latency_ms"],
                },
            )

            # Store in DB
            try:
                conn = psycopg2.connect(DB_URL)
                cur = conn.cursor()
                cur.execute(
                    """INSERT INTO ml_predictions
                       (id, model_id, model_name, model_version, symbol, prediction_type,
                        horizon, predicted_value, confidence, latency_ms, is_fallback, features_used)
                       VALUES (gen_random_uuid(),
                               (SELECT id FROM ml_model_registry WHERE model_name='catboost' AND status='active' LIMIT 1),
                               'catboost', %s, %s, 'return', '1h', %s, %s, %s, FALSE, %s)""",
                    (
                        result["model_version"],
                        symbol,
                        json.dumps({"direction": result["direction"], "return_pct": result["return_pct"]}),
                        result["confidence"],
                        result["latency_ms"],
                        json.dumps(features),
                    ),
                )
                conn.commit()
                cur.close()
                conn.close()
            except Exception as e:
                logger.warning("DB write failed: %s", e)

            logger.info(
                "CatBoost %s: %s %.4f%% conf=%.2f",
                symbol, result["direction"], result["return_pct"] * 100, result["confidence"],
            )

        except Exception as e:
            logger.error("CatBoost prediction failed for %s: %s", symbol, e)
