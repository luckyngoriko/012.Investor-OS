"""DEPRECATED: Use NATS event-driven pipeline instead.

This script is kept for manual one-shot predictions only.
The canonical prediction pipeline is now NATS-based:
  ios.prices → ios.features → ios.predict.* → ios.consensus

To run manually: python -m app.tasks.prediction_loop
"""

import asyncio
import json
import logging
import os
import time
from datetime import datetime, timezone

import numpy as np
import psycopg2

logger = logging.getLogger(__name__)

DB_URL = os.environ.get("DATABASE_URL", "postgresql://investor:trjkNPtO1ykTKxrF1hMosUKvQGBp7c@postgres:5432/investor_os")
SYMBOLS = ["BTCUSDT", "ETHUSDT"]
HORIZON = "1h"


def compute_features_for_symbol(cur, symbol: str) -> dict | None:
    """Compute technical features from the last 100 candles."""
    cur.execute(
        "SELECT time, open, high, low, close, volume FROM prices WHERE symbol=%s ORDER BY time DESC LIMIT 100",
        (symbol,),
    )
    rows = cur.fetchall()
    if len(rows) < 30:
        logger.warning("%s: not enough candles (%d)", symbol, len(rows))
        return None

    # Reverse to chronological order
    rows = rows[::-1]
    closes = np.array([r[4] for r in rows])
    highs = np.array([r[2] for r in rows])
    lows = np.array([r[3] for r in rows])
    volumes = np.array([r[5] for r in rows])

    # RSI-14
    diffs = np.diff(closes[-15:])
    gains = np.mean(np.maximum(diffs, 0))
    losses = np.mean(np.maximum(-diffs, 0))
    rsi = 100 - 100 / (1 + gains / losses) if losses > 0 else 100.0

    # Bollinger position
    bb_mean = np.mean(closes[-20:])
    bb_std = np.std(closes[-20:])
    bb_pos = (closes[-1] - (bb_mean - 2 * bb_std)) / (4 * bb_std) if bb_std > 0 else 0.5

    # ATR-14
    trs = [
        max(highs[-j] - lows[-j], abs(highs[-j] - closes[-j - 1]), abs(lows[-j] - closes[-j - 1]))
        for j in range(1, min(15, len(highs)))
    ]
    atr = float(np.mean(trs)) if trs else 0.0

    # MACD simplified
    ema12 = float(np.mean(closes[-12:]))
    ema26 = float(np.mean(closes[-26:]))
    macd_sig = ema12 - ema26
    macd_hist = macd_sig - (float(np.mean(closes[-9:])) - float(np.mean(closes[-35:]))) if len(closes) >= 35 else 0.0

    # Volume and price changes
    vol_change = float((volumes[-1] - volumes[-2]) / volumes[-2]) if volumes[-2] > 0 else 0.0
    pct5 = float((closes[-1] - closes[-6]) / closes[-6]) if len(closes) > 5 else 0.0
    pct20 = float((closes[-1] - closes[-21]) / closes[-21]) if len(closes) > 20 else 0.0

    # OBV trend
    obv = sum(
        volumes[j] if closes[j] > closes[j - 1] else -volumes[j] if closes[j] < closes[j - 1] else 0
        for j in range(max(1, len(closes) - 10), len(closes))
    )
    avg_vol = float(np.mean(volumes[-10:]))
    obv_trend = float(obv / avg_vol) if avg_vol > 0 else 0.0

    return {
        "rsi_14": round(float(rsi), 4),
        "macd_signal": round(float(macd_sig), 4),
        "macd_histogram": round(float(macd_hist), 4),
        "atr_14": round(atr, 4),
        "bb_position": round(float(bb_pos), 4),
        "obv_trend": round(obv_trend, 4),
        "volume_change_pct": round(vol_change, 4),
        "price_change_pct_5": round(pct5, 6),
        "price_change_pct_20": round(pct20, 6),
    }


def run_catboost_prediction(features: dict) -> dict | None:
    """Run CatBoost prediction using the loaded model."""
    try:
        from app.models.catboost_model import CatBoostPredictor

        cb = CatBoostPredictor(model_path="/app/logs")
        if not cb.load("latest"):
            logger.warning("CatBoost model not found")
            return None
        return cb.predict(features)
    except Exception as e:
        logger.error("CatBoost prediction failed: %s", e)
        return None


def run_garch_prediction(cur, symbol: str) -> dict | None:
    """Run GARCH volatility forecast using real returns."""
    try:
        cur.execute("SELECT close FROM prices WHERE symbol=%s ORDER BY time ASC", (symbol,))
        closes = [r[0] for r in cur.fetchall()]
        if len(closes) < 50:
            return None

        returns = [np.log(closes[i] / closes[i - 1]) for i in range(1, len(closes))]

        from app.models.garch_model import GarchPredictor

        garch = GarchPredictor()
        return garch.predict(returns, horizon_days=1)
    except Exception as e:
        logger.error("GARCH prediction failed: %s", e)
        return None


def store_prediction(cur, model_id: str, model_name: str, version: str, symbol: str,
                     pred_type: str, horizon: str, value: dict, confidence: float,
                     latency_ms: int, features: dict):
    """Insert prediction into ml_predictions."""
    cur.execute(
        """INSERT INTO ml_predictions
           (id, model_id, model_name, model_version, symbol, prediction_type, horizon,
            predicted_value, confidence, latency_ms, is_fallback, features_used, predicted_at)
           VALUES (gen_random_uuid(), %s, %s, %s, %s, %s, %s, %s, %s, %s, FALSE, %s, NOW())""",
        (model_id, model_name, version, symbol, pred_type, horizon,
         json.dumps(value), confidence, latency_ms, json.dumps(features)),
    )


def get_model_id(cur, model_name: str) -> str:
    """Get the active model UUID from registry."""
    cur.execute(
        "SELECT id FROM ml_model_registry WHERE model_name=%s AND status='active' ORDER BY created_at DESC LIMIT 1",
        (model_name,),
    )
    row = cur.fetchone()
    return str(row[0]) if row else "00000000-0000-0000-0000-000000000000"


def run_prediction_cycle():
    """Execute one full prediction cycle for all symbols."""
    start = time.time()
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()

    catboost_model_id = get_model_id(cur, "catboost")
    garch_model_id = get_model_id(cur, "garch")
    consensus_model_id = get_model_id(cur, "consensus")
    total_predictions = 0

    for symbol in SYMBOLS:
        logger.info("Running predictions for %s...", symbol)

        # 1. Compute features
        features = compute_features_for_symbol(cur, symbol)
        if features is None:
            continue

        # Store features
        cur.execute(
            "INSERT INTO ml_feature_store (id, symbol, feature_set, features, source, computed_at) "
            "VALUES (gen_random_uuid(), %s, 'technical', %s, 'prediction_loop', NOW())",
            (symbol, json.dumps(features)),
        )

        # 2. CatBoost prediction
        cb_result = run_catboost_prediction(features)
        if cb_result:
            store_prediction(
                cur, catboost_model_id, "catboost", cb_result.get("model_version", "v1"),
                symbol, "return", HORIZON,
                {"direction": cb_result["direction"], "return_pct": cb_result["return_pct"]},
                cb_result["confidence"], cb_result["latency_ms"], features,
            )
            total_predictions += 1
            logger.info("  CatBoost: %s return=%.4f%% conf=%.2f",
                        cb_result["direction"], cb_result["return_pct"] * 100, cb_result["confidence"])

        # 3. GARCH volatility
        garch_result = run_garch_prediction(cur, symbol)
        if garch_result:
            store_prediction(
                cur, garch_model_id, "garch", garch_result["model_version"],
                symbol, "volatility", HORIZON,
                {
                    "volatility_daily": garch_result["forecasted_volatility_daily"],
                    "volatility_ann": garch_result["forecasted_volatility_ann"],
                    "var_95": garch_result["var_95"],
                    "var_99": garch_result["var_99"],
                },
                garch_result["confidence"], garch_result["latency_ms"], {},
            )
            total_predictions += 1
            logger.info("  GARCH: vol=%.2f%% VaR95=%.2f%%",
                        garch_result["forecasted_volatility_daily"] * 100,
                        garch_result["var_95"] * 100)

        # 4. Simple consensus
        if cb_result and garch_result:
            cb_conf = cb_result["confidence"]
            garch_conf = garch_result["confidence"]
            consensus_conf = (cb_conf + garch_conf) / 2

            # If GARCH says high vol + CatBoost says long → reduce confidence
            if garch_result["forecasted_volatility_daily"] > 0.02 and cb_result["direction"] == "long":
                consensus_conf *= 0.8

            store_prediction(
                cur, consensus_model_id, "consensus", "v1",
                symbol, "return", HORIZON,
                {
                    "direction": cb_result["direction"],
                    "return_pct": cb_result["return_pct"],
                    "volatility_daily": garch_result["forecasted_volatility_daily"],
                    "var_95": garch_result["var_95"],
                    "models_agreed": True,
                },
                round(consensus_conf, 4), 0, {},
            )
            total_predictions += 1

    conn.commit()
    cur.close()
    conn.close()

    elapsed = round(time.time() - start, 1)
    logger.info("Prediction cycle complete: %d predictions in %.1fs", total_predictions, elapsed)
    return total_predictions


async def run_prediction_loop(interval_seconds: int = 3600):
    """Run prediction cycles on a schedule."""
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
    logger.info("Prediction loop started (interval=%ds)", interval_seconds)

    # Run immediately on startup
    run_prediction_cycle()

    while True:
        await asyncio.sleep(interval_seconds)
        try:
            run_prediction_cycle()
        except Exception as e:
            logger.error("Prediction cycle failed: %s", e)


if __name__ == "__main__":
    # Can be run standalone for testing
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
    n = run_prediction_cycle()
    print(f"Generated {n} predictions")
