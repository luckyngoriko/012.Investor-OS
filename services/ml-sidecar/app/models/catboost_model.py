"""CatBoost price/return prediction model (Sprint 119).

Wraps CatBoostRegressor for financial return prediction.
Supports train, predict, save, and load operations.
"""

import logging
import os
import time
from pathlib import Path

import numpy as np
from catboost import CatBoostRegressor, Pool

logger = logging.getLogger(__name__)

# Feature columns expected by the model
FEATURE_COLUMNS = [
    "rsi_14",
    "macd_signal",
    "macd_histogram",
    "atr_14",
    "bb_position",
    "obv_trend",
    "volume_change_pct",
    "price_change_pct_5",
    "price_change_pct_20",
]

DEFAULT_MODEL_PATH = os.environ.get("ML_MODEL_DIR", "/app/models")


class CatBoostPredictor:
    """CatBoost-based return prediction model."""

    def __init__(self, model_path: str | None = None):
        self.model: CatBoostRegressor | None = None
        self.model_version: str = "0.0.0"
        self.model_path = model_path or DEFAULT_MODEL_PATH

    def load(self, version: str = "latest") -> bool:
        """Load a trained model from disk."""
        path = Path(self.model_path) / f"catboost_{version}.cbm"
        if not path.exists():
            # Try the default name
            path = Path(self.model_path) / "catboost_latest.cbm"
        if not path.exists():
            logger.warning("No CatBoost model found at %s", path)
            return False

        self.model = CatBoostRegressor()
        self.model.load_model(str(path))
        self.model_version = version
        logger.info("Loaded CatBoost model v%s from %s", version, path)
        return True

    def save(self, version: str) -> str:
        """Save the trained model to disk."""
        if self.model is None:
            raise RuntimeError("No model to save — train first")

        path = Path(self.model_path) / f"catboost_{version}.cbm"
        path.parent.mkdir(parents=True, exist_ok=True)
        self.model.save_model(str(path))
        self.model_version = version
        logger.info("Saved CatBoost model v%s to %s", version, path)
        return str(path)

    def train(
        self,
        X_train: np.ndarray,
        y_train: np.ndarray,
        X_val: np.ndarray | None = None,
        y_val: np.ndarray | None = None,
        iterations: int = 500,
        learning_rate: float = 0.05,
        depth: int = 6,
    ) -> dict:
        """Train the CatBoost model. Returns training metrics."""
        self.model = CatBoostRegressor(
            iterations=iterations,
            learning_rate=learning_rate,
            depth=depth,
            loss_function="RMSE",
            eval_metric="RMSE",
            random_seed=42,
            verbose=0,
        )

        eval_set = None
        if X_val is not None and y_val is not None:
            eval_set = Pool(X_val, y_val, feature_names=FEATURE_COLUMNS)

        train_pool = Pool(X_train, y_train, feature_names=FEATURE_COLUMNS)

        self.model.fit(train_pool, eval_set=eval_set)

        # Compute metrics
        train_pred = self.model.predict(X_train)
        train_rmse = float(np.sqrt(np.mean((y_train - train_pred) ** 2)))
        train_mae = float(np.mean(np.abs(y_train - train_pred)))

        metrics = {
            "train_rmse": train_rmse,
            "train_mae": train_mae,
            "iterations": iterations,
            "learning_rate": learning_rate,
            "depth": depth,
            "n_features": len(FEATURE_COLUMNS),
            "n_train_samples": len(y_train),
        }

        if X_val is not None and y_val is not None:
            val_pred = self.model.predict(X_val)
            metrics["val_rmse"] = float(np.sqrt(np.mean((y_val - val_pred) ** 2)))
            metrics["val_mae"] = float(np.mean(np.abs(y_val - val_pred)))
            metrics["n_val_samples"] = len(y_val)

        logger.info("CatBoost training complete: %s", metrics)
        return metrics

    def predict(self, features: dict) -> dict:
        """Predict return for a single observation.

        Args:
            features: Dict with keys matching FEATURE_COLUMNS.

        Returns:
            Dict with prediction, confidence, and metadata.
        """
        if self.model is None:
            raise RuntimeError("No model loaded — load or train first")

        start = time.time()

        # Build feature vector in correct order
        feature_vec = np.array(
            [[features.get(col, 0.0) for col in FEATURE_COLUMNS]]
        )

        pred = float(self.model.predict(feature_vec)[0])

        # Confidence heuristic: based on feature importance alignment
        # Higher confidence when prediction is moderate (not extreme)
        confidence = min(1.0, max(0.1, 1.0 - abs(pred) * 10))

        latency_ms = int((time.time() - start) * 1000)

        direction = "long" if pred > 0.001 else ("short" if pred < -0.001 else "neutral")

        return {
            "return_pct": round(pred, 6),
            "direction": direction,
            "confidence": round(confidence, 4),
            "latency_ms": latency_ms,
            "model_version": self.model_version,
            "features_used": FEATURE_COLUMNS,
        }

    @property
    def is_loaded(self) -> bool:
        return self.model is not None

    def feature_importance(self) -> dict[str, float]:
        """Return feature importances from the trained model."""
        if self.model is None:
            return {}
        importances = self.model.get_feature_importance()
        return dict(zip(FEATURE_COLUMNS, [float(x) for x in importances]))
