#!/usr/bin/env python3
"""
Convert PyTorch HRM model to SafeTensors format for Rust inference.

Usage:
    python convert_to_safetensors.py input.pt output.safetensors
    
Requirements:
    pip install torch safetensors
"""

import argparse
import sys
from pathlib import Path

try:
    import torch
    from safetensors.torch import save_file
except ImportError:
    print("Error: Required packages not installed.")
    print("Run: pip install torch safetensors")
    sys.exit(1)


def convert_pytorch_to_safetensors(input_path: str, output_path: str) -> None:
    """
    Convert PyTorch model/state_dict to SafeTensors format.
    
    Args:
        input_path: Path to .pt or .pth file
        output_path: Path to output .safetensors file
    """
    print(f"Loading PyTorch model from: {input_path}")
    
    # Load the PyTorch model
    checkpoint = torch.load(input_path, map_location="cpu")
    
    # Extract state dict
    if isinstance(checkpoint, dict):
        if "state_dict" in checkpoint:
            state_dict = checkpoint["state_dict"]
        elif "model" in checkpoint:
            state_dict = checkpoint["model"]
        else:
            state_dict = checkpoint
    else:
        # Assume it's already a state dict
        state_dict = checkpoint
    
    # Filter out non-tensor entries and convert to float32
    tensors = {}
    for key, value in state_dict.items():
        if isinstance(value, torch.Tensor):
            # Ensure float32 for compatibility
            tensors[key] = value.to(torch.float32)
            print(f"  - {key}: {value.shape}")
        else:
            print(f"  - Skipping {key} (not a tensor)")
    
    print(f"\nSaving to SafeTensors: {output_path}")
    
    # Save as SafeTensors
    save_file(tensors, output_path)
    
    # Print statistics
    total_params = sum(t.numel() for t in tensors.values())
    file_size = Path(output_path).stat().st_size / (1024 * 1024)  # MB
    
    print(f"\nConversion complete!")
    print(f"  Total parameters: {total_params:,}")
    print(f"  File size: {file_size:.2f} MB")
    print(f"  Tensors: {len(tensors)}")


def verify_safetensors(path: str) -> None:
    """Verify the converted SafeTensors file."""
    from safetensors.torch import load_file
    
    print(f"\nVerifying: {path}")
    tensors = load_file(path)
    
    print(f"  Successfully loaded {len(tensors)} tensors")
    for key, tensor in tensors.items():
        print(f"    {key}: {tensor.shape}, dtype={tensor.dtype}")


def main():
    parser = argparse.ArgumentParser(
        description="Convert PyTorch HRM model to SafeTensors format"
    )
    parser.add_argument("input", help="Input PyTorch model (.pt or .pth)")
    parser.add_argument("output", help="Output SafeTensors file (.safetensors)")
    parser.add_argument(
        "--verify", 
        action="store_true", 
        help="Verify the output file after conversion"
    )
    
    args = parser.parse_args()
    
    # Validate input
    if not Path(args.input).exists():
        print(f"Error: Input file not found: {args.input}")
        sys.exit(1)
    
    # Convert
    try:
        convert_pytorch_to_safetensors(args.input, args.output)
        
        if args.verify:
            verify_safetensors(args.output)
            
    except Exception as e:
        print(f"Error during conversion: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
