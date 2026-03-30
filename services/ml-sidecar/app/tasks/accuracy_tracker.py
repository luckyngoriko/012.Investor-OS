"""Prediction accuracy tracker (Sprint 126).

Periodic task that:
1. Finds predictions whose horizon has elapsed but actual_value is not yet filled.
2. Fetches the actual price/return from the database.
3. Computes error metrics (MAE, RMSE).
4. Updates ml_predictions with actual_value, error_metric, resolved_at.
5. Tracks rolling accuracy per model.
"""

import asyncio
import logging
from datetime import datetime, timezone

logger = logging.getLogger(__name__)


async def resolve_predictions(db_pool) -> int:
    """Resolve unresolved predictions whose horizon has elapsed.

    Returns the number of predictions resolved.
    """
    # This is a placeholder that will be connected to a real DB pool
    # when asyncpg/sqlalchemy is wired up in a later sprint.
    #
    # The SQL logic:
    # 1. SELECT from ml_predictions WHERE resolved_at IS NULL
    #    AND predicted_at + horizon_interval < NOW()
    # 2. For each, look up actual price from prices table
    # 3. Compute error_metric = |predicted - actual| / actual
    # 4. UPDATE ml_predictions SET actual_value, error_metric, resolved_at

    logger.info("Accuracy tracker: checking for unresolved predictions...")

    # Placeholder query — will be replaced with actual DB queries
    resolve_sql = """
        UPDATE ml_predictions
        SET
            resolved_at = NOW(),
            error_metric = ABS(
                (predicted_value->>'return_pct')::float -
                COALESCE((actual_value->>'return_pct')::float, 0)
            )
        WHERE resolved_at IS NULL
          AND predicted_at + (
              CASE horizon
                  WHEN '1h' THEN INTERVAL '1 hour'
                  WHEN '4h' THEN INTERVAL '4 hours'
                  WHEN '1d' THEN INTERVAL '1 day'
                  WHEN '1w' THEN INTERVAL '1 week'
                  WHEN '1m' THEN INTERVAL '1 month'
                  ELSE INTERVAL '1 day'
              END
          ) < NOW()
    """

    logger.debug("Resolve SQL ready: %s", resolve_sql[:80])
    return 0  # Placeholder — returns count when DB is connected


async def compute_model_accuracy(db_pool) -> dict:
    """Compute rolling accuracy metrics per model.

    Returns dict of {model_name: {rmse_7d, rmse_30d, mae_7d, mae_30d, n_predictions}}.
    """
    accuracy_sql = """
        SELECT
            model_name,
            COUNT(*) FILTER (WHERE resolved_at > NOW() - INTERVAL '7 days') AS n_7d,
            AVG(error_metric) FILTER (WHERE resolved_at > NOW() - INTERVAL '7 days') AS mae_7d,
            SQRT(AVG(error_metric * error_metric) FILTER (WHERE resolved_at > NOW() - INTERVAL '7 days')) AS rmse_7d,
            COUNT(*) FILTER (WHERE resolved_at > NOW() - INTERVAL '30 days') AS n_30d,
            AVG(error_metric) FILTER (WHERE resolved_at > NOW() - INTERVAL '30 days') AS mae_30d,
            SQRT(AVG(error_metric * error_metric) FILTER (WHERE resolved_at > NOW() - INTERVAL '30 days')) AS rmse_30d
        FROM ml_predictions
        WHERE resolved_at IS NOT NULL AND error_metric IS NOT NULL
        GROUP BY model_name
    """

    logger.debug("Accuracy SQL ready: %s", accuracy_sql[:80])
    return {}  # Placeholder


async def run_accuracy_loop(db_pool, interval_seconds: int = 3600):
    """Run accuracy tracking in a loop (default: every hour)."""
    while True:
        try:
            resolved = await resolve_predictions(db_pool)
            if resolved > 0:
                logger.info("Resolved %d predictions", resolved)

            accuracy = await compute_model_accuracy(db_pool)
            for model, metrics in accuracy.items():
                logger.info("Model %s accuracy: %s", model, metrics)

        except Exception as e:
            logger.error("Accuracy tracker error: %s", e)

        await asyncio.sleep(interval_seconds)
