"""Feature assembly for ML models (Sprint 119).

Builds feature dictionaries from raw input or database.
Used by prediction endpoints to prepare model inputs.
"""

from app.models.catboost_model import FEATURE_COLUMNS


def build_features_from_request(raw_features: dict) -> dict:
    """Extract and validate features from a prediction request.

    Missing features default to 0.0 (neutral).
    Extra features are ignored.
    """
    return {col: float(raw_features.get(col, 0.0)) for col in FEATURE_COLUMNS}


def validate_features(features: dict) -> list[str]:
    """Return list of missing feature names (empty = all present)."""
    return [col for col in FEATURE_COLUMNS if col not in features]
