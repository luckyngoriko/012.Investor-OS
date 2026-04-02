"""Resolve predictions by comparing with actual prices (Phase D2).

Finds predictions whose horizon has elapsed, looks up the actual price,
computes error metrics, and updates ml_predictions.
"""

import json
import logging
import os
import time

import numpy as np
import psycopg2

logger = logging.getLogger(__name__)

DB_URL = os.environ.get("DATABASE_URL", "postgresql://investor:trjkNPtO1ykTKxrF1hMosUKvQGBp7c@postgres:5432/investor_os")

# Horizon to interval mapping
HORIZON_INTERVALS = {
    "1h": "1 hour",
    "4h": "4 hours",
    "1d": "1 day",
    "1w": "7 days",
}


def resolve_predictions() -> int:
    """Find and resolve unresolved predictions."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()

    resolved = 0

    for horizon, interval in HORIZON_INTERVALS.items():
        # Find unresolved predictions where horizon has elapsed
        cur.execute(f"""
            SELECT p.id, p.symbol, p.model_name, p.prediction_type, p.predicted_value,
                   p.predicted_at, p.confidence
            FROM ml_predictions p
            WHERE p.resolved_at IS NULL
              AND p.horizon = %s
              AND p.predicted_at + INTERVAL '{interval}' < NOW()
            LIMIT 100
        """, (horizon,))

        rows = cur.fetchall()
        if not rows:
            continue

        for pred_id, symbol, model_name, pred_type, pred_value, predicted_at, confidence in rows:
            # Find the actual price at prediction_time + horizon
            target_time_sql = f"'{predicted_at}'::timestamptz + INTERVAL '{interval}'"
            cur.execute(f"""
                SELECT close FROM prices
                WHERE symbol = %s AND time >= {target_time_sql}
                ORDER BY time ASC LIMIT 1
            """, (symbol,))

            actual_row = cur.fetchone()
            if actual_row is None:
                continue  # Actual price not yet available

            actual_close = actual_row[0]

            # Get the price at prediction time
            cur.execute("""
                SELECT close FROM prices
                WHERE symbol = %s AND time <= %s
                ORDER BY time DESC LIMIT 1
            """, (symbol, predicted_at))

            base_row = cur.fetchone()
            if base_row is None:
                continue

            base_close = base_row[0]
            actual_return = (actual_close - base_close) / base_close

            # Compute error metric based on prediction type
            if pred_type == "return":
                predicted_return = pred_value.get("return_pct", 0) if isinstance(pred_value, dict) else 0
                error = abs(predicted_return - actual_return)
                direction_correct = (predicted_return > 0) == (actual_return > 0)
            elif pred_type == "volatility":
                # For volatility predictions, compare predicted vol with realized vol
                error = abs(actual_return)  # Simplified — actual absolute return as proxy
                direction_correct = None
            else:
                error = abs(actual_return)
                direction_correct = None

            actual_value = {
                "actual_close": actual_close,
                "base_close": base_close,
                "actual_return": round(actual_return, 6),
                "direction_correct": direction_correct,
            }

            # Update prediction
            cur.execute("""
                UPDATE ml_predictions
                SET actual_value = %s, error_metric = %s, resolved_at = NOW()
                WHERE id = %s AND predicted_at = %s
            """, (json.dumps(actual_value), error, pred_id, predicted_at))

            resolved += 1
            logger.info(
                "Resolved %s %s: predicted=%.4f%% actual=%.4f%% error=%.4f%% correct=%s",
                symbol, model_name,
                (pred_value.get("return_pct", 0) * 100) if isinstance(pred_value, dict) else 0,
                actual_return * 100,
                error * 100,
                direction_correct,
            )

    conn.commit()

    # Refresh materialized view
    try:
        cur.execute("REFRESH MATERIALIZED VIEW CONCURRENTLY mv_ml_model_accuracy")
        conn.commit()
        logger.info("Refreshed mv_ml_model_accuracy")
    except Exception as e:
        logger.warning("Failed to refresh materialized view: %s", e)
        conn.rollback()

    cur.close()
    conn.close()

    logger.info("Resolved %d predictions", resolved)
    return resolved


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
    n = resolve_predictions()
    print(f"Resolved {n} predictions")
