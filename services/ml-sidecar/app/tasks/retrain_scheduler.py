"""Model retraining scheduler (Sprint 126).

Monitors model accuracy and triggers retraining when:
- RMSE increases by >20% vs baseline (first 30 days after training)
- Model has been active for >30 days without retraining
- Explicit retrain request via API

Retraining is done by invoking the training script as a subprocess.
"""

import logging
import subprocess
import sys
from pathlib import Path

logger = logging.getLogger(__name__)

# Thresholds
RMSE_DEGRADATION_THRESHOLD = 0.20  # 20% increase triggers retraining
MAX_DAYS_WITHOUT_RETRAIN = 30


async def check_retrain_needed(db_pool) -> list[str]:
    """Check which models need retraining.

    Returns list of model names that should be retrained.
    """
    # Placeholder — will compare current RMSE vs baseline from ml_model_registry.metrics
    retrain_sql = """
        WITH current_accuracy AS (
            SELECT
                model_name,
                SQRT(AVG(error_metric * error_metric)) AS current_rmse
            FROM ml_predictions
            WHERE resolved_at > NOW() - INTERVAL '7 days'
              AND error_metric IS NOT NULL
            GROUP BY model_name
        ),
        baseline AS (
            SELECT
                model_name,
                (metrics->>'train_rmse')::float AS baseline_rmse,
                trained_at
            FROM ml_model_registry
            WHERE status = 'active'
        )
        SELECT
            b.model_name,
            c.current_rmse,
            b.baseline_rmse,
            (c.current_rmse - b.baseline_rmse) / NULLIF(b.baseline_rmse, 0) AS degradation_pct
        FROM baseline b
        JOIN current_accuracy c ON c.model_name = b.model_name
        WHERE (c.current_rmse - b.baseline_rmse) / NULLIF(b.baseline_rmse, 0) > %(threshold)s
           OR b.trained_at < NOW() - INTERVAL '%(max_days)s days'
    """

    logger.debug("Retrain check SQL ready")
    return []  # Placeholder


def trigger_retrain(model_name: str, output_dir: str = "/app/models") -> bool:
    """Trigger model retraining by running the training script.

    Returns True if retraining succeeded.
    """
    scripts = {
        "catboost": "scripts/ml/train_catboost.py",
    }

    script = scripts.get(model_name)
    if script is None:
        logger.warning("No training script for model: %s", model_name)
        return False

    script_path = Path(__file__).resolve().parent.parent.parent.parent.parent / script
    if not script_path.exists():
        logger.error("Training script not found: %s", script_path)
        return False

    version = f"retrained_{model_name}"

    cmd = [
        sys.executable,
        str(script_path),
        "--version", version,
        "--output-dir", output_dir,
    ]

    logger.info("Triggering retrain for %s: %s", model_name, " ".join(cmd))

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=600,  # 10 minute timeout
        )
        if result.returncode == 0:
            logger.info("Retrain succeeded for %s", model_name)
            return True
        else:
            logger.error("Retrain failed for %s: %s", model_name, result.stderr)
            return False
    except subprocess.TimeoutExpired:
        logger.error("Retrain timed out for %s", model_name)
        return False
    except Exception as e:
        logger.error("Retrain error for %s: %s", model_name, e)
        return False
