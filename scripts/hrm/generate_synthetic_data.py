#!/usr/bin/env python3
"""
Generate Synthetic Training Data for HRM

Creates realistic synthetic trading data based on market principles:
- Bull markets: High PEGY, insider buying, positive sentiment, low VIX
- Bear markets: Low PEGY, insider selling, negative sentiment, high VIX
- Crisis: Extreme fear signals

Usage:
    python generate_synthetic_data.py --output-dir data/hrm --samples 10000
"""

import argparse
import json
import random
import numpy as np
from pathlib import Path
from typing import List, Dict, Tuple
import csv

# Set seed for reproducibility
random.seed(42)
np.random.seed(42)


def generate_market_regime() -> Tuple[str, Dict]:
    """Generate a random market regime with appropriate signal distributions."""
    regimes = ["bull", "bear", "sideways", "crisis"]
    weights = [0.4, 0.3, 0.2, 0.1]  # Bull more common than crisis
    
    regime = random.choices(regimes, weights=weights)[0]
    
    params = {
        "bull": {
            "pegy_mean": 0.75, "pegy_std": 0.15,
            "insider_mean": 0.70, "insider_std": 0.20,
            "sentiment_mean": 0.75, "sentiment_std": 0.15,
            "vix_mean": 15.0, "vix_std": 5.0,
            "regime_code": 0.0,
        },
        "bear": {
            "pegy_mean": 0.30, "pegy_std": 0.15,
            "insider_mean": 0.25, "insider_std": 0.20,
            "sentiment_mean": 0.30, "sentiment_std": 0.20,
            "vix_mean": 45.0, "vix_std": 10.0,
            "regime_code": 1.0,
        },
        "sideways": {
            "pegy_mean": 0.50, "pegy_std": 0.15,
            "insider_mean": 0.50, "insider_std": 0.15,
            "sentiment_mean": 0.50, "sentiment_std": 0.15,
            "vix_mean": 20.0, "vix_std": 5.0,
            "regime_code": 2.0,
        },
        "crisis": {
            "pegy_mean": 0.15, "pegy_std": 0.10,
            "insider_mean": 0.15, "insider_std": 0.15,
            "sentiment_mean": 0.15, "sentiment_std": 0.10,
            "vix_mean": 75.0, "vix_std": 10.0,
            "regime_code": 3.0,
        },
    }[regime]
    
    return regime, params


def clamp(value: float, min_val: float, max_val: float) -> float:
    """Clamp value to range."""
    return max(min_val, min(max_val, value))


def generate_sample() -> Dict:
    """Generate a single synthetic sample with realistic market logic."""
    regime, params = generate_market_regime()
    
    # Generate base signals
    pegy = clamp(np.random.normal(params["pegy_mean"], params["pegy_std"]), 0.0, 1.0)
    insider = clamp(np.random.normal(params["insider_mean"], params["insider_std"]), 0.0, 1.0)
    sentiment = clamp(np.random.normal(params["sentiment_mean"], params["sentiment_std"]), 0.0, 1.0)
    vix = clamp(np.random.normal(params["vix_mean"], params["vix_std"]), 5.0, 100.0)
    
    # Time of day (normalized 0-1)
    time_of_day = random.uniform(0.0, 1.0)
    
    # Market regime code
    regime_code = params["regime_code"]
    
    # Calculate target conviction based on signals and regime
    # This is our "ground truth" formula - what HRM should learn
    base_score = (
        pegy * 0.30 +
        insider * 0.30 +
        sentiment * 0.25 +
        (1.0 - vix / 100.0) * 0.15  # Inverse VIX
    )
    
    # Regime adjustments
    if regime == "bull":
        base_score *= 1.15
    elif regime == "bear":
        base_score *= 0.70
    elif regime == "crisis":
        base_score *= 0.30
    
    conviction = clamp(base_score, 0.0, 1.0)
    
    # Confidence based on signal consistency
    signal_variance = np.var([pegy, insider, sentiment])
    confidence = clamp(0.5 + (1.0 - signal_variance) * 0.5, 0.0, 1.0)
    
    # Trading decision (profitable if conviction > 0.7 and not crisis)
    should_trade = conviction > 0.7 and regime != "crisis"
    
    # Profit outcome (with some noise)
    if should_trade:
        base_return = (conviction - 0.5) * 0.20  # 0-10% return range
        noise = np.random.normal(0, 0.02)
        profit_return = base_return + noise
    else:
        profit_return = np.random.normal(-0.01, 0.01)  # Small loss for missed trades
    
    return {
        "signals": {
            "pegy": round(pegy, 4),
            "insider": round(insider, 4),
            "sentiment": round(sentiment, 4),
            "vix": round(vix, 2),
            "regime_code": int(regime_code),
            "time_of_day": round(time_of_day, 4),
        },
        "regime": regime,
        "target": {
            "conviction": round(conviction, 4),
            "confidence": round(confidence, 4),
            "should_trade": should_trade,
            "expected_return": round(profit_return, 4),
        }
    }


def generate_training_data(n_samples: int) -> List[Dict]:
    """Generate training dataset."""
    print(f"Generating {n_samples} training samples...")
    return [generate_sample() for _ in range(n_samples)]


def generate_test_cases(n_cases: int) -> List[Dict]:
    """Generate specific test cases for validation."""
    print(f"Generating {n_cases} test cases...")
    
    test_cases = []
    
    # Extreme cases
    extreme_cases = [
        {"name": "perfect_bull", "pegy": 0.95, "insider": 0.95, "sentiment": 0.95, "vix": 10.0, "regime": 0.0},
        {"name": "perfect_bear", "pegy": 0.05, "insider": 0.05, "sentiment": 0.05, "vix": 60.0, "regime": 1.0},
        {"name": "extreme_crisis", "pegy": 0.05, "insider": 0.05, "sentiment": 0.05, "vix": 90.0, "regime": 3.0},
        {"name": "neutral", "pegy": 0.50, "insider": 0.50, "sentiment": 0.50, "vix": 20.0, "regime": 2.0},
        {"name": "high_vix_bull", "pegy": 0.80, "insider": 0.80, "sentiment": 0.80, "vix": 50.0, "regime": 0.0},
        {"name": "insider_only", "pegy": 0.30, "insider": 0.95, "sentiment": 0.30, "vix": 25.0, "regime": 0.0},
    ]
    
    for case in extreme_cases:
        signals = [
            case["pegy"],
            case["insider"],
            case["sentiment"],
            case["vix"],
            case["regime"],
            0.5,  # time
        ]
        
        # Calculate expected output
        base = case["pegy"] * 0.30 + case["insider"] * 0.30 + case["sentiment"] * 0.25
        base += (1.0 - case["vix"] / 100.0) * 0.15
        
        if case["regime"] == 0.0:
            base *= 1.15
        elif case["regime"] == 1.0:
            base *= 0.70
        elif case["regime"] == 3.0:
            base *= 0.30
        
        conviction = clamp(base, 0.0, 1.0)
        
        test_cases.append({
            "name": case["name"],
            "input": signals,
            "expected": {
                "conviction": round(conviction, 4),
                "regime": int(case["regime"]),
            }
        })
    
    # Random cases
    for i in range(n_cases - len(extreme_cases)):
        sample = generate_sample()
        test_cases.append({
            "name": f"random_{i}",
            "input": [
                sample["signals"]["pegy"],
                sample["signals"]["insider"],
                sample["signals"]["sentiment"],
                sample["signals"]["vix"],
                float(sample["signals"]["regime_code"]),
                sample["signals"]["time_of_day"],
            ],
            "expected": {
                "conviction": sample["target"]["conviction"],
                "regime": sample["signals"]["regime_code"],
            }
        })
    
    return test_cases


def save_csv(data: List[Dict], filepath: Path):
    """Save data as CSV."""
    with open(filepath, 'w', newline='') as f:
        writer = csv.writer(f)
        # Header
        writer.writerow([
            'pegy', 'insider', 'sentiment', 'vix', 'regime_code', 'time_of_day',
            'regime', 'conviction', 'confidence', 'should_trade', 'expected_return'
        ])
        # Data
        for item in data:
            writer.writerow([
                item["signals"]["pegy"],
                item["signals"]["insider"],
                item["signals"]["sentiment"],
                item["signals"]["vix"],
                item["signals"]["regime_code"],
                item["signals"]["time_of_day"],
                item["regime"],
                item["target"]["conviction"],
                item["target"]["confidence"],
                int(item["target"]["should_trade"]),
                item["target"]["expected_return"],
            ])
    print(f"Saved CSV: {filepath}")


def save_json(data: List[Dict], filepath: Path):
    """Save data as JSON."""
    with open(filepath, 'w') as f:
        json.dump(data, f, indent=2)
    print(f"Saved JSON: {filepath}")


def main():
    parser = argparse.ArgumentParser(description="Generate synthetic HRM training data")
    parser.add_argument("--output-dir", default="data/hrm", help="Output directory")
    parser.add_argument("--train-samples", type=int, default=10000, help="Training samples")
    parser.add_argument("--test-samples", type=int, default=1000, help="Test samples")
    
    args = parser.parse_args()
    
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Create subdirectories
    (output_dir / "training").mkdir(exist_ok=True)
    (output_dir / "testing").mkdir(exist_ok=True)
    (output_dir / "golden").mkdir(exist_ok=True)
    
    # Generate training data
    training_data = generate_training_data(args.train_samples)
    save_csv(training_data, output_dir / "training" / "synthetic_trades.csv")
    save_json(training_data, output_dir / "training" / "synthetic_trades.json")
    
    # Generate test cases
    test_cases = generate_test_cases(args.test_samples)
    save_json(test_cases, output_dir / "testing" / "test_cases.json")
    
    # Generate golden dataset (first 100 test cases with precise targets)
    golden_cases = test_cases[:100]
    save_json(golden_cases, output_dir / "golden" / "reference_cases.json")
    
    # Statistics
    print("\n" + "="*60)
    print("Dataset Statistics:")
    print("="*60)
    
    regimes = [item["regime"] for item in training_data]
    for regime in ["bull", "bear", "sideways", "crisis"]:
        count = regimes.count(regime)
        print(f"  {regime:12}: {count:5} samples ({count/len(regimes)*100:.1f}%)")
    
    convictions = [item["target"]["conviction"] for item in training_data]
    print(f"\n  Conviction Range: {min(convictions):.2f} - {max(convictions):.2f}")
    print(f"  Conviction Mean:  {np.mean(convictions):.2f}")
    
    trades = sum(1 for item in training_data if item["target"]["should_trade"])
    print(f"\n  Trades: {trades}/{len(training_data)} ({trades/len(training_data)*100:.1f}%)")
    
    print("\n" + "="*60)
    print("Files created:")
    print("="*60)
    print(f"  Training: {output_dir}/training/synthetic_trades.*")
    print(f"  Testing:  {output_dir}/testing/test_cases.json")
    print(f"  Golden:   {output_dir}/golden/reference_cases.json")


if __name__ == "__main__":
    main()
