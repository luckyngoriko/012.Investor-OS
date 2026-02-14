#!/usr/bin/env python3
# Use: source ~/.venv/hrm/bin/activate before running
"""
Train Hierarchical Reasoning Model (HRM) for Investor OS.

This script trains an HRM model on synthetic trading data for Sprint 36.
In production, replace synthetic data with actual historical trading data.

Usage:
    python train_hrm.py --output models/hrm_v1.safetensors
"""

import argparse
import json
import sys
from pathlib import Path

try:
    import torch
    import torch.nn as nn
    import torch.optim as optim
    from torch.utils.data import Dataset, DataLoader
    from safetensors.torch import save_file
except ImportError:
    print("Error: Required packages not installed.")
    print("Run: pip install torch safetensors")
    sys.exit(1)


class LSTMCell(nn.Module):
    """Single LSTM cell matching Rust implementation."""
    
    def __init__(self, input_size: int, hidden_size: int):
        super().__init__()
        self.input_size = input_size
        self.hidden_size = hidden_size
        
        # Combined weights for all gates
        self.weight_ih = nn.Linear(input_size, 4 * hidden_size, bias=False)
        self.weight_hh = nn.Linear(hidden_size, 4 * hidden_size, bias=False)
    
    def forward(self, x, hidden=None):
        batch_size = x.size(0)
        
        if hidden is None:
            h = torch.zeros(batch_size, self.hidden_size, device=x.device)
            c = torch.zeros(batch_size, self.hidden_size, device=x.device)
        else:
            h, c = hidden
        
        # Compute gates
        gates = self.weight_ih(x) + self.weight_hh(h)
        i, f, g, o = gates.chunk(4, 1)
        
        # Apply activations
        i = torch.sigmoid(i)
        f = torch.sigmoid(f)
        g = torch.tanh(g)
        o = torch.sigmoid(o)
        
        # Update cell and hidden
        c_new = f * c + i * g
        h_new = o * torch.tanh(c_new)
        
        return h_new, c_new


class HRM(nn.Module):
    """
    Hierarchical Reasoning Model matching Rust architecture.
    
    Architecture:
    - High-level module: LSTM(input=6, hidden=128)
    - Low-level module: LSTM(input=64, hidden=64)
    - Cross-connections
    - Output: Linear(192, 3)
    """
    
    def __init__(
        self,
        input_size: int = 6,
        high_hidden: int = 128,
        low_hidden: int = 64,
        output_size: int = 3,
    ):
        super().__init__()
        
        # High-level module (slow, abstract)
        self.high_lstm = LSTMCell(input_size, high_hidden)
        
        # Cross-connection: high -> low
        self.high_to_low = nn.Linear(high_hidden, low_hidden, bias=False)
        
        # Low-level module (fast, detailed)
        self.low_lstm = LSTMCell(low_hidden, low_hidden)
        
        # Output layer
        self.output = nn.Linear(high_hidden + low_hidden, output_size)
    
    def forward(self, x):
        """
        Forward pass.
        
        Args:
            x: Input tensor [batch, input_size]
        
        Returns:
            output: [batch, 3] (conviction, confidence, regime)
        """
        # High-level processing
        high_out, _ = self.high_lstm(x)
        
        # Cross-connection to low-level
        low_input = self.high_to_low(high_out)
        
        # Low-level processing
        low_out, _ = self.low_lstm(low_input)
        
        # Combine and output
        combined = torch.cat([high_out, low_out], dim=1)
        output = self.output(combined)
        
        # Apply output activations
        conviction = torch.sigmoid(output[:, 0])
        confidence = torch.sigmoid(output[:, 1])
        regime = output[:, 2]  # Raw logits for classification
        
        return torch.stack([conviction, confidence, regime], dim=1)


class TradingDataset(Dataset):
    """Synthetic trading dataset for training."""
    
    def __init__(self, size: int = 10000):
        self.size = size
        
        # Generate synthetic data
        torch.manual_seed(42)
        
        self.inputs = []
        self.targets = []
        
        for _ in range(size):
            # Random market conditions
            pegy = torch.rand(1).item()
            insider = torch.rand(1).item()
            sentiment = torch.rand(1).item()
            vix = torch.rand(1).item() * 100
            regime = torch.randint(0, 4, (1,)).item()
            time = torch.rand(1).item()
            
            # Simple heuristic for target
            base_conviction = (pegy * 0.3 + insider * 0.3 + sentiment * 0.4)
            volatility_factor = 1.0 - (vix / 100.0)
            conviction = base_conviction * volatility_factor
            
            confidence = 0.5 + (pegy + insider + sentiment) / 6.0
            
            self.inputs.append([pegy, insider, sentiment, vix, regime, time])
            self.targets.append([conviction, confidence, regime])
    
    def __len__(self):
        return self.size
    
    def __getitem__(self, idx):
        return (
            torch.tensor(self.inputs[idx], dtype=torch.float32),
            torch.tensor(self.targets[idx], dtype=torch.float32),
        )


def train_hrm(
    output_path: str,
    epochs: int = 100,
    batch_size: int = 32,
    learning_rate: float = 0.001,
    device: str = "cuda" if torch.cuda.is_available() else "cpu",
):
    """Train HRM model."""
    
    print(f"Training HRM on {device}")
    print(f"Output: {output_path}")
    
    # Create model
    model = HRM().to(device)
    
    # Count parameters
    total_params = sum(p.numel() for p in model.parameters())
    print(f"Total parameters: {total_params:,}")
    
    # Create dataset
    train_dataset = TradingDataset(size=10000)
    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True)
    
    # Optimizer and loss
    optimizer = optim.Adam(model.parameters(), lr=learning_rate)
    criterion = nn.MSELoss()
    
    # Training loop
    model.train()
    for epoch in range(epochs):
        total_loss = 0.0
        
        for inputs, targets in train_loader:
            inputs = inputs.to(device)
            targets = targets.to(device)
            
            # Forward
            optimizer.zero_grad()
            outputs = model(inputs)
            
            # Loss (only on conviction and confidence, regime is auxiliary)
            loss = criterion(outputs[:, :2], targets[:, :2])
            
            # Backward
            loss.backward()
            optimizer.step()
            
            total_loss += loss.item()
        
        if (epoch + 1) % 10 == 0:
            avg_loss = total_loss / len(train_loader)
            print(f"Epoch {epoch + 1}/{epochs}, Loss: {avg_loss:.6f}")
    
    # Save model
    print(f"\nSaving model to: {output_path}")
    
    # Extract state dict
    state_dict = model.state_dict()
    
    # Convert to float32
    tensors = {k: v.to(torch.float32).contiguous() for k, v in state_dict.items()}
    
    # Save as SafeTensors
    save_file(tensors, output_path)
    
    # Save metadata
    metadata = {
        "input_size": 6,
        "high_hidden_size": 128,
        "low_hidden_size": 64,
        "output_size": 3,
        "parameters": total_params,
        "training_epochs": epochs,
        "training_samples": len(train_dataset),
    }
    
    metadata_path = output_path.replace(".safetensors", "_metadata.json")
    with open(metadata_path, "w") as f:
        json.dump(metadata, f, indent=2)
    
    print(f"Metadata saved to: {metadata_path}")
    print("\nTraining complete!")
    
    # Test inference
    model.eval()
    with torch.no_grad():
        test_input = torch.tensor([[0.8, 0.9, 0.7, 15.0, 0.0, 0.5]], device=device)
        test_output = model(test_input)
        print(f"\nTest inference:")
        print(f"  Input: {test_input[0].tolist()}")
        print(f"  Output (conviction, confidence, regime): {test_output[0].tolist()}")


def main():
    parser = argparse.ArgumentParser(description="Train HRM model")
    parser.add_argument(
        "--output",
        default="models/hrm_v1.safetensors",
        help="Output path for trained model",
    )
    parser.add_argument(
        "--epochs",
        type=int,
        default=100,
        help="Number of training epochs",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=32,
        help="Batch size",
    )
    parser.add_argument(
        "--lr",
        type=float,
        default=0.001,
        help="Learning rate",
    )
    parser.add_argument(
        "--device",
        default="cuda" if torch.cuda.is_available() else "cpu",
        help="Device to train on",
    )
    
    args = parser.parse_args()
    
    # Create output directory
    output_dir = Path(args.output).parent
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Train
    train_hrm(
        output_path=args.output,
        epochs=args.epochs,
        batch_size=args.batch_size,
        learning_rate=args.lr,
        device=args.device,
    )


if __name__ == "__main__":
    main()
