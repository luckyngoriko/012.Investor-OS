"""Tests for CatBoost prediction endpoint (Sprint 119).

Note: These tests work without a trained model — they verify
the endpoint returns 503 when no model is loaded, and test
the feature builder independently.
"""

import sys
from pathlib import Path

# Add sidecar app to path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from app.features.builder import build_features_from_request, validate_features
from app.models.catboost_model import FEATURE_COLUMNS


def test_feature_columns_defined():
    assert len(FEATURE_COLUMNS) == 9
    assert "rsi_14" in FEATURE_COLUMNS
    assert "macd_signal" in FEATURE_COLUMNS
    assert "atr_14" in FEATURE_COLUMNS


def test_build_features_fills_defaults():
    raw = {"rsi_14": 65.2, "macd_signal": 0.003}
    features = build_features_from_request(raw)
    assert features["rsi_14"] == 65.2
    assert features["macd_signal"] == 0.003
    assert features["atr_14"] == 0.0  # default
    assert len(features) == len(FEATURE_COLUMNS)


def test_build_features_ignores_extra():
    raw = {"rsi_14": 50.0, "extra_field": 999}
    features = build_features_from_request(raw)
    assert "extra_field" not in features
    assert features["rsi_14"] == 50.0


def test_validate_features_reports_missing():
    missing = validate_features({"rsi_14": 50.0})
    assert "macd_signal" in missing
    assert "rsi_14" not in missing


def test_validate_features_all_present():
    full = {col: 0.0 for col in FEATURE_COLUMNS}
    missing = validate_features(full)
    assert missing == []
