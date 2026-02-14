#!/usr/bin/env python3
"""
Generate Golden Dataset for HRM Validation (Sprint 41)

This script generates reference test cases using the trained Python model.
The dataset is used to validate that Rust HRM produces identical outputs.
"""

import torch
import json
import numpy as np
from pathlib import Path

# Model architecture (must match trained model)
class HRMModel(torch.nn.Module):
    def __init__(self):
        super().__init__()
        self.fc1 = torch.nn.Linear(6, 128)
        self.fc2 = torch.nn.Linear(128, 64)
        self.fc3 = torch.nn.Linear(64, 3)
    
    def forward(self, x):
        x = torch.relu(self.fc1(x))
        x = torch.relu(self.fc2(x))
        x = torch.sigmoid(self.fc3(x))
        return x

def load_model(weights_path: str) -> HRMModel:
    """Load trained model from SafeTensors."""
    from safetensors.torch import load_file
    
    model = HRMModel()
    state_dict = load_file(weights_path)
    model.load_state_dict(state_dict)
    model.eval()
    return model

def generate_test_cases() -> list:
    """Generate diverse test cases covering all regimes and signal combinations."""
    cases = []
    
    # Test Case 1: Strong Bull (all signals excellent)
    cases.append({
        "name": "strong_bull",
        "input": [0.9, 0.9, 0.9, 10.0, 0.0, 0.5],
        "description": "Strong bull market with excellent signals"
    })
    
    # Test Case 2: Moderate Bull
    cases.append({
        "name": "moderate_bull",
        "input": [0.7, 0.7, 0.7, 15.0, 0.0, 0.5],
        "description": "Moderate bull market"
    })
    
    # Test Case 3: Weak Bull (borderline)
    cases.append({
        "name": "weak_bull",
        "input": [0.5, 0.5, 0.5, 20.0, 0.0, 0.5],
        "description": "Weak bull signals"
    })
    
    # Test Case 4: Strong Bear
    cases.append({
        "name": "strong_bear",
        "input": [0.1, 0.1, 0.1, 50.0, 1.0, 0.5],
        "description": "Strong bear market"
    })
    
    # Test Case 5: Moderate Bear
    cases.append({
        "name": "moderate_bear",
        "input": [0.3, 0.3, 0.3, 40.0, 1.0, 0.5],
        "description": "Moderate bear market"
    })
    
    # Test Case 6: Sideways/Mixed
    cases.append({
        "name": "sideways",
        "input": [0.5, 0.5, 0.5, 25.0, 2.0, 0.5],
        "description": "Sideways market with mixed signals"
    })
    
    # Test Case 7: Crisis
    cases.append({
        "name": "crisis",
        "input": [0.1, 0.1, 0.1, 80.0, 3.0, 0.5],
        "description": "Crisis market - extreme fear"
    })
    
    # Test Case 8: High PEGY, Low Sentiment
    cases.append({
        "name": "high_pegy_low_sentiment",
        "input": [0.9, 0.5, 0.2, 20.0, 0.0, 0.5],
        "description": "Good fundamentals but poor sentiment"
    })
    
    # Test Case 9: Low PEGY, High Insider
    cases.append({
        "name": "low_pegy_high_insider",
        "input": [0.2, 0.9, 0.5, 20.0, 0.0, 0.5],
        "description": "Poor fundamentals but strong insider buying"
    })
    
    # Test Case 10: High Volatility Impact
    cases.append({
        "name": "high_volatility",
        "input": [0.8, 0.8, 0.8, 60.0, 0.0, 0.5],
        "description": "Good signals but high VIX"
    })
    
    # Test Case 11: Low Volatility Boost
    cases.append({
        "name": "low_volatility",
        "input": [0.6, 0.6, 0.6, 10.0, 0.0, 0.5],
        "description": "Moderate signals with low VIX"
    })
    
    # Test Case 12: Edge Case - All Zeros
    cases.append({
        "name": "all_zeros",
        "input": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        "description": "Edge case - all zeros"
    })
    
    # Test Case 13: Edge Case - All Ones
    cases.append({
        "name": "all_ones",
        "input": [1.0, 1.0, 1.0, 100.0, 3.0, 1.0],
        "description": "Edge case - all max values"
    })
    
    # Test Case 14: Random 1
    cases.append({
        "name": "random_1",
        "input": [0.65, 0.42, 0.78, 23.5, 1.0, 0.3],
        "description": "Random test case 1"
    })
    
    # Test Case 15: Random 2
    cases.append({
        "name": "random_2",
        "input": [0.33, 0.88, 0.45, 45.2, 2.0, 0.7],
        "description": "Random test case 2"
    })
    
    # Test Case 16: Random 3
    cases.append({
        "name": "random_3",
        "input": [0.82, 0.15, 0.91, 12.8, 0.0, 0.9],
        "description": "Random test case 3"
    })
    
    # Test Case 17: Random 4
    cases.append({
        "name": "random_4",
        "input": [0.21, 0.67, 0.34, 67.3, 3.0, 0.2],
        "description": "Random test case 4"
    })
    
    # Test Case 18: Random 5
    cases.append({
        "name": "random_5",
        "input": [0.55, 0.73, 0.29, 31.4, 1.0, 0.6],
        "description": "Random test case 5"
    })
    
    # Test Case 19: Time Impact - Opening
    cases.append({
        "name": "opening",
        "input": [0.7, 0.7, 0.7, 20.0, 0.0, 0.1],
        "description": "Market opening (time=0.1)"
    })
    
    # Test Case 20: Time Impact - Closing
    cases.append({
        "name": "closing",
        "input": [0.7, 0.7, 0.7, 20.0, 0.0, 0.9],
        "description": "Market closing (time=0.9)"
    })
    
    return cases

def generate_golden_dataset():
    """Generate and save golden dataset."""
    
    # Load model
    weights_path = "models/hrm_synthetic_v1.safetensors"
    print(f"Loading model from {weights_path}...")
    model = load_model(weights_path)
    
    # Generate test cases
    test_cases = generate_test_cases()
    
    # Compute expected outputs
    golden_cases = []
    for case in test_cases:
        input_tensor = torch.tensor([case["input"]], dtype=torch.float32)
        
        with torch.no_grad():
            output = model(input_tensor)
        
        output_values = output[0].numpy().tolist()
        
        # Determine regime from third output
        regime_value = output_values[2]
        if regime_value < 0.5:
            regime = "Bull"
        elif regime_value < 1.5:
            regime = "Bear"
        elif regime_value < 2.5:
            regime = "Sideways"
        else:
            regime = "Crisis"
        
        golden_cases.append({
            "name": case["name"],
            "description": case["description"],
            "input": case["input"],
            "expected": {
                "conviction": round(output_values[0], 6),
                "confidence": round(output_values[1], 6),
                "regime_raw": round(output_values[2], 6),
                "regime": regime
            }
        })
        
        print(f"  {case['name']}: conv={output_values[0]:.4f}, conf={output_values[1]:.4f}, regime={regime}")
    
    # Save to JSON
    output_path = Path("tests/golden_path/hrm_golden_dataset.json")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    dataset = {
        "version": "1.0",
        "model": "hrm_synthetic_v1",
        "generated_at": str(torch.__version__),
        "total_cases": len(golden_cases),
        "tolerance": 0.001,  # 0.1% tolerance for floating point comparison
        "cases": golden_cases
    }
    
    with open(output_path, 'w') as f:
        json.dump(dataset, f, indent=2)
    
    print(f"\n✅ Saved {len(golden_cases)} golden cases to {output_path}")
    
    # Print summary
    convictions = [c["expected"]["conviction"] for c in golden_cases]
    print(f"\nConviction range: {min(convictions):.4f} - {max(convictions):.4f}")
    print(f"Mean conviction: {sum(convictions)/len(convictions):.4f}")

if __name__ == "__main__":
    generate_golden_dataset()
