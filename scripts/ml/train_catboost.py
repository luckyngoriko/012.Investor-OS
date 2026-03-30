#!/usr/bin/env python3
"""CatBoost training script for Investor OS (Sprint 119).

Generates synthetic financial features, trains a CatBoost model,
and saves it for the ML sidecar to load.

Usage:
    python scripts/ml/train_catboost.py [--samples 5000] [--version v1]
"""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

# Add sidecar app to path for model import
sys.path.insert(0, str(Path(__file__).resolve().parent.parent.parent / "services" / "ml-sidecar"))

from app.models.catboost_model import FEATURE_COLUMNS, CatBoostPredictor


def generate_synthetic_data(n_samples: int = 5000, seed: int = 42) -> tuple:
    """Generate synthetic financial feature data with realistic distributions.

    Returns (X_train, y_train, X_val, y_val).
    """
    rng = np.random.default_rng(seed)

    features = {}

    # RSI: 0-100, normally distributed around 50
    features["rsi_14"] = rng.normal(50, 15, n_samples).clip(0, 100)

    # MACD signal: small values around 0
    features["macd_signal"] = rng.normal(0, 0.005, n_samples)

    # MACD histogram
    features["macd_histogram"] = rng.normal(0, 0.003, n_samples)

    # ATR: positive, typically 1-5% of price
    features["atr_14"] = rng.lognormal(0, 0.5, n_samples).clip(0.5, 20)

    # Bollinger position: 0-1
    features["bb_position"] = rng.beta(2, 2, n_samples)

    # OBV trend: -1 to 1
    features["obv_trend"] = rng.normal(0, 0.3, n_samples).clip(-1, 1)

    # Volume change %: mostly small
    features["volume_change_pct"] = rng.normal(0, 0.3, n_samples)

    # Price change 5-period
    features["price_change_pct_5"] = rng.normal(0.001, 0.02, n_samples)

    # Price change 20-period
    features["price_change_pct_20"] = rng.normal(0.004, 0.05, n_samples)

    # Build feature matrix
    X = np.column_stack([features[col] for col in FEATURE_COLUMNS])

    # Target: next-period return (synthetic, correlated with features)
    # RSI < 30 → positive return (oversold bounce)
    # RSI > 70 → negative return (overbought pullback)
    rsi_signal = (50 - features["rsi_14"]) / 500
    macd_signal = features["macd_histogram"] * 5
    momentum = features["price_change_pct_5"] * 0.3
    noise = rng.normal(0, 0.01, n_samples)

    y = rsi_signal + macd_signal + momentum + noise

    # Train/val split (80/20)
    split = int(0.8 * n_samples)
    return X[:split], y[:split], X[split:], y[split:]


def main():
    parser = argparse.ArgumentParser(description="Train CatBoost model for Investor OS")
    parser.add_argument("--samples", type=int, default=5000, help="Training samples")
    parser.add_argument("--version", type=str, default="latest", help="Model version tag")
    parser.add_argument("--iterations", type=int, default=500, help="Training iterations")
    parser.add_argument("--depth", type=int, default=6, help="Tree depth")
    parser.add_argument("--lr", type=float, default=0.05, help="Learning rate")
    parser.add_argument(
        "--output-dir",
        type=str,
        default=str(Path(__file__).resolve().parent.parent.parent / "models"),
        help="Model output directory",
    )
    args = parser.parse_args()

    print(f"Generating {args.samples} synthetic training samples...")
    X_train, y_train, X_val, y_val = generate_synthetic_data(args.samples)
    print(f"  Train: {X_train.shape}, Val: {X_val.shape}")

    print(f"Training CatBoost (iterations={args.iterations}, depth={args.depth}, lr={args.lr})...")
    predictor = CatBoostPredictor(model_path=args.output_dir)
    metrics = predictor.train(
        X_train, y_train, X_val, y_val,
        iterations=args.iterations,
        learning_rate=args.lr,
        depth=args.depth,
    )
    print(f"  Train RMSE: {metrics['train_rmse']:.6f}")
    print(f"  Val RMSE:   {metrics.get('val_rmse', 'N/A')}")
    print(f"  Train MAE:  {metrics['train_mae']:.6f}")
    print(f"  Val MAE:    {metrics.get('val_mae', 'N/A')}")

    # Save model
    model_path = predictor.save(args.version)
    print(f"Model saved to: {model_path}")

    # Save metadata
    meta = {
        "model_name": "catboost",
        "model_version": args.version,
        "model_type": "catboost",
        "tier": 1,
        "trained_at": datetime.now(timezone.utc).isoformat(),
        "training_samples": int(X_train.shape[0]),
        "validation_samples": int(X_val.shape[0]),
        "features": FEATURE_COLUMNS,
        "metrics": metrics,
        "parameters": {
            "iterations": args.iterations,
            "depth": args.depth,
            "learning_rate": args.lr,
            "loss_function": "RMSE",
        },
    }

    meta_path = Path(args.output_dir) / f"catboost_{args.version}_meta.json"
    meta_path.write_text(json.dumps(meta, indent=2, default=str))
    print(f"Metadata saved to: {meta_path}")

    # Feature importance
    importance = predictor.feature_importance()
    print("\nFeature importance:")
    for feat, imp in sorted(importance.items(), key=lambda x: -x[1]):
        print(f"  {feat:25s} {imp:6.2f}%")

    print(f"\nDone. Model ready at: {model_path}")


if __name__ == "__main__":
    main()
