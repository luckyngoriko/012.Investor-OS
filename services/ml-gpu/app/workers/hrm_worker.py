"""Real HRM (sapientinc) NATS worker (Sprint N4 upgrade).

Subscribes to ios.features.{symbol} — runs real HRM Transformer inference
on GPU and publishes to ios.predict.hrm.{symbol}.
"""

import json
import logging

import psycopg2

from hrm.trading_adapter import HRMTradingModel
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)

DB_URL = "postgresql://investor:trjkNPtO1ykTKxrF1hMosUKvQGBp7c@postgres:5432/investor_os"


async def run(nats: NatsManager):
    """Run HRM worker — subscribe to features, predict, publish."""
    hrm = HRMTradingModel(device="cuda")
    n_params = hrm.init_model(
        hidden_size=256,
        num_heads=8,
        h_layers=3,
        l_layers=3,
        h_cycles=2,
        l_cycles=2,
    )

    # Try loading trained weights
    hrm.load("/app/models/hrm_trading.pth")

    logger.info("HRM worker started (%d params, %s)", n_params, hrm.device)

    sub = await nats.nc.subscribe("ios.features.>")

    async for msg in sub.messages:
        envelope = nats.parse_envelope(msg.data)
        if not envelope:
            continue

        symbol = envelope["symbol"]
        features = envelope["data"]

        try:
            # Fetch recent feature history from DB for context
            conn = psycopg2.connect(DB_URL)
            cur = conn.cursor()
            cur.execute(
                "SELECT features FROM ml_feature_store WHERE symbol=%s AND feature_set='technical' ORDER BY computed_at DESC LIMIT 8",
                (symbol,),
            )
            rows = cur.fetchall()
            cur.close()
            conn.close()

            # Build feature list (most recent last)
            features_list = [row[0] if isinstance(row[0], dict) else json.loads(row[0]) for row in reversed(rows)]
            if not features_list:
                features_list = [features]

            result = hrm.predict(features_list)

            # Publish to NATS
            await nats.publish(
                f"ios.predict.hrm.{symbol}",
                symbol,
                "hrm",
                {
                    "model_name": "hrm",
                    "model_version": result["model_version"],
                    "prediction_type": "return",
                    "direction": result["direction"],
                    "predicted_return": 0.02 if result["direction"] == "long" else (-0.02 if result["direction"] == "short" else 0.0),
                    "confidence": result["confidence"],
                    "latency_ms": result["latency_ms"],
                    "action": result["action"],
                    "action_probs": result["action_probs"],
                },
            )

            logger.info(
                "HRM %s: %s (action=%s, conf=%.2f, %dms, %s)",
                symbol, result["direction"], result["action"],
                result["confidence"], result["latency_ms"], result["device"],
            )

        except Exception as e:
            logger.error("HRM failed for %s: %s", symbol, e)
