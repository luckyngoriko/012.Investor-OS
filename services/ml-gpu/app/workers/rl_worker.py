"""RL Trading Agent NATS worker (Wave 3 Task 19).

Subscribes to ios.consensus.> -- when a consensus signal arrives, extracts
the 9 CatBoost-compatible features, runs the RL policy network for action
prediction, and publishes to ios.predict.rl.{symbol}.
"""

import json
import logging

from app.models.rl_agent import RLTradingAgent, ACTION_SPACE
from app.nats_client import NatsManager

logger = logging.getLogger(__name__)

MODEL_PATH = "/app/models/rl_agent.pth"

# Feature keys expected in the consensus envelope (same as CatBoost pipeline)
FEATURE_KEYS = [
    "rsi",
    "macd",
    "atr",
    "bb",
    "obv",
    "vol_change",
    "pct5",
    "pct20",
    "consensus_confidence",
]


def extract_features(data: dict) -> list[float] | None:
    """Extract the 9-dim state vector from a consensus data envelope.

    Looks for features in `data` directly, or inside `data["features"]`.
    Returns None if any required feature is missing.
    """
    source = data.get("features", data)
    features = []
    for key in FEATURE_KEYS:
        val = source.get(key)
        if val is None:
            # Try top-level data as fallback
            val = data.get(key)
        if val is None:
            logger.debug("Missing feature '%s' in consensus data", key)
            return None
        try:
            features.append(float(val))
        except (TypeError, ValueError):
            logger.debug("Non-numeric feature '%s': %s", key, val)
            return None
    return features


async def run(nats: NatsManager):
    """Run RL worker -- subscribe to consensus, predict, publish."""
    agent = RLTradingAgent(device="cuda")

    # Try loading pre-trained weights
    loaded = agent.load(MODEL_PATH)
    if not loaded:
        logger.info("RL agent starting with random policy (no saved weights)")

    logger.info("RL worker started (device=%s)", agent.device)

    sub = await nats.nc.subscribe("ios.consensus.>")

    async for msg in sub.messages:
        envelope = nats.parse_envelope(msg.data)
        if not envelope:
            continue

        symbol = envelope.get("symbol", "UNKNOWN")
        data = envelope.get("data", {})

        try:
            features = extract_features(data)
            if features is None:
                logger.debug("Skipping %s -- incomplete features", symbol)
                continue

            action, probs = agent.predict_with_probs(features)

            # Determine direction from position size
            if action > 0:
                direction = "long"
            elif action < 0:
                direction = "short"
            else:
                direction = "neutral"

            # Map action to index for logging
            action_idx = ACTION_SPACE.index(action)

            await nats.publish(
                f"ios.predict.rl.{symbol}",
                symbol,
                "rl_agent",
                {
                    "model_name": "rl_agent",
                    "model_version": "v1.0",
                    "prediction_type": "position_size",
                    "direction": direction,
                    "position_size": action,
                    "action_index": action_idx,
                    "action_probs": probs,
                    "confidence": max(probs),
                },
            )

            logger.info(
                "RL %s: %s (size=%.1f, conf=%.2f, action_idx=%d)",
                symbol, direction, action, max(probs), action_idx,
            )

        except Exception as e:
            logger.error("RL worker failed for %s: %s", symbol, e)
