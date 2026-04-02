#!/usr/bin/env python3
"""Train sapientinc/HRM on real BTCUSDT data from Investor OS postgres.

Loads OHLCV candles, computes technical indicators (RSI, MACD, BB, trend),
tokenizes with HRM trading_adapter, and trains for next-action prediction.

Usage (inside ios-ml-gpu container):
    python3 /app/scripts/train_hrm_real.py

Environment:
    DB_URL  -- postgres connection string (default: internal docker network)
"""

import logging
import math
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F
from torch.optim import AdamW
from torch.optim.lr_scheduler import CosineAnnealingLR
from torch.utils.data import DataLoader, Dataset

# Ensure hrm module is importable from /app
sys.path.insert(0, "/app")

from hrm.trading_adapter import (
    VOCAB_SIZE,
    SEQ_LEN,
    tokenize_candle,
    HRMTradingModel,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
)
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
DB_URL = os.environ.get(
    "DB_URL",
    "postgresql://investor:trjkNPtO1ykTKxrF1hMosUKvQGBp7c@postgres:5432/investor_os",
)

HIDDEN_SIZE = 256
NUM_HEADS = 8
H_LAYERS = 3
L_LAYERS = 3

EPOCHS = 50
BATCH_SIZE = 16
LR = 5e-4
LOOKBACK = 30        # min candles needed before first usable index
WINDOW = SEQ_LEN     # sliding window length (in tokens)
TOKENS_PER_CANDLE = 6  # tokenize_candle returns 6 tokens

BEST_MODEL_PATH = "/tmp/hrm_trading_best.pth"
FINAL_MODEL_PATH = "/app/models/hrm_trading.pth"


# ---------------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------------
def load_candles(db_url: str) -> list[dict]:
    """Load all BTCUSDT candles from postgres, ordered by time ascending."""
    import psycopg2

    logger.info("Connecting to postgres ...")
    conn = psycopg2.connect(db_url)
    cur = conn.cursor()
    cur.execute(
        """
        SELECT time, open, high, low, close, volume
        FROM prices
        WHERE symbol = 'BTCUSDT'
        ORDER BY time ASC
        """
    )
    rows = cur.fetchall()
    cur.close()
    conn.close()

    candles = []
    for row in rows:
        candles.append(
            {
                "timestamp": row[0],
                "open": float(row[1]),
                "high": float(row[2]),
                "low": float(row[3]),
                "close": float(row[4]),
                "volume": float(row[5]),
            }
        )
    logger.info("Loaded %d BTCUSDT candles", len(candles))
    return candles


# ---------------------------------------------------------------------------
# Technical indicators
# ---------------------------------------------------------------------------
def compute_rsi(closes: np.ndarray, period: int = 14) -> np.ndarray:
    """Compute RSI over an array of close prices."""
    deltas = np.diff(closes, prepend=closes[0])
    gains = np.where(deltas > 0, deltas, 0.0)
    losses = np.where(deltas < 0, -deltas, 0.0)

    avg_gain = np.zeros_like(closes)
    avg_loss = np.zeros_like(closes)
    avg_gain[period] = gains[1 : period + 1].mean()
    avg_loss[period] = losses[1 : period + 1].mean()

    for i in range(period + 1, len(closes)):
        avg_gain[i] = (avg_gain[i - 1] * (period - 1) + gains[i]) / period
        avg_loss[i] = (avg_loss[i - 1] * (period - 1) + losses[i]) / period

    rs = np.divide(avg_gain, avg_loss, out=np.ones_like(avg_gain), where=avg_loss != 0)
    rsi = 100.0 - 100.0 / (1.0 + rs)
    return rsi


def compute_ema(data: np.ndarray, span: int) -> np.ndarray:
    """Exponential moving average."""
    alpha = 2.0 / (span + 1)
    ema = np.zeros_like(data)
    ema[0] = data[0]
    for i in range(1, len(data)):
        ema[i] = alpha * data[i] + (1 - alpha) * ema[i - 1]
    return ema


def compute_macd_hist(closes: np.ndarray) -> np.ndarray:
    """MACD histogram = MACD line - signal line."""
    ema12 = compute_ema(closes, 12)
    ema26 = compute_ema(closes, 26)
    macd_line = ema12 - ema26
    signal = compute_ema(macd_line, 9)
    return macd_line - signal


def compute_bb_position(closes: np.ndarray, period: int = 20) -> np.ndarray:
    """Bollinger Band position: 0 = at lower band, 1 = at upper band."""
    bb_pos = np.full_like(closes, 0.5)
    for i in range(period, len(closes)):
        window = closes[i - period + 1 : i + 1]
        mean = window.mean()
        std = window.std()
        if std > 0:
            bb_pos[i] = (closes[i] - (mean - 2 * std)) / (4 * std)
        else:
            bb_pos[i] = 0.5
    return np.clip(bb_pos, 0.0, 1.0)


def compute_features(candles: list[dict]) -> list[dict]:
    """Compute all technical indicators for every candle.

    Returns a list of feature dicts (one per candle) starting from index 0,
    but only indices >= LOOKBACK are reliable.
    """
    closes = np.array([c["close"] for c in candles])
    volumes = np.array([c["volume"] for c in candles])

    rsi = compute_rsi(closes, 14)
    macd_hist = compute_macd_hist(closes)
    bb_pos = compute_bb_position(closes, 20)

    # Price change pct (1-candle)
    pct_change = np.zeros_like(closes)
    pct_change[1:] = (closes[1:] - closes[:-1]) / closes[:-1]

    # Volume z-score (rolling 20)
    vol_zscore = np.zeros_like(volumes)
    for i in range(20, len(volumes)):
        win = volumes[i - 20 : i]
        mu, sigma = win.mean(), win.std()
        vol_zscore[i] = (volumes[i] - mu) / sigma if sigma > 0 else 0.0

    # Trend: 20-period price change
    trend = np.zeros_like(closes)
    for i in range(20, len(closes)):
        trend[i] = (closes[i] - closes[i - 20]) / closes[i - 20]

    features = []
    for i in range(len(candles)):
        features.append(
            {
                "pct_change": float(pct_change[i]),
                "rsi": float(rsi[i]),
                "macd_hist": float(macd_hist[i]),
                "volume_zscore": float(vol_zscore[i]),
                "bb_position": float(bb_pos[i]),
                "trend": float(trend[i]),
            }
        )
    return features


# ---------------------------------------------------------------------------
# Action label from next-candle return
# ---------------------------------------------------------------------------
def next_return_to_action_token(next_return: float) -> int:
    """Map next-candle return to an action token.

    >  2%  -> buy_strong  (50)
    >  0.5%-> buy         (51)
    < -2%  -> sell_strong (54)
    < -0.5%-> sell        (53)
    else   -> hold        (52)
    """
    if next_return > 0.02:
        return 50
    elif next_return > 0.005:
        return 51
    elif next_return < -0.02:
        return 54
    elif next_return < -0.005:
        return 53
    else:
        return 52


# ---------------------------------------------------------------------------
# Dataset
# ---------------------------------------------------------------------------
class HRMTradingDataset(Dataset):
    """Sliding-window dataset of tokenized candle sequences + action target."""

    def __init__(self, token_sequences: list[list[int]], targets: list[int]):
        assert len(token_sequences) == len(targets)
        self.sequences = token_sequences
        self.targets = targets

    def __len__(self):
        return len(self.sequences)

    def __getitem__(self, idx):
        seq = torch.LongTensor(self.sequences[idx])  # (SEQ_LEN,)
        tgt = torch.LongTensor([self.targets[idx]])    # (1,)
        return seq, tgt


def build_dataset(candles: list[dict]) -> tuple[list[list[int]], list[int]]:
    """Build tokenized sequences and action targets from candles.

    For each candle i (LOOKBACK <= i < len-1):
      - Tokenize candles [i-window+1 .. i] (window = SEQ_LEN // TOKENS_PER_CANDLE)
      - Pad/truncate to SEQ_LEN tokens
      - Target = action token from next candle return
    """
    features = compute_features(candles)
    closes = np.array([c["close"] for c in candles])

    # Number of candles per window to fill SEQ_LEN tokens
    candles_per_window = SEQ_LEN // TOKENS_PER_CANDLE  # 48 // 6 = 8

    sequences = []
    targets = []

    for i in range(max(LOOKBACK, candles_per_window), len(candles) - 1):
        # Tokenize the window of candles
        window_start = i - candles_per_window + 1
        tokens = []
        for j in range(window_start, i + 1):
            f = features[j]
            candle_tokens = tokenize_candle(
                pct_change=f["pct_change"],
                rsi=f["rsi"],
                macd_hist=f["macd_hist"],
                volume_zscore=f["volume_zscore"],
                bb_position=f["bb_position"],
                trend=f["trend"],
            )
            tokens.extend(candle_tokens)

        # Ensure exactly SEQ_LEN tokens
        if len(tokens) > SEQ_LEN:
            tokens = tokens[-SEQ_LEN:]
        elif len(tokens) < SEQ_LEN:
            tokens = [0] * (SEQ_LEN - len(tokens)) + tokens

        # Target: next-candle return
        next_ret = (closes[i + 1] - closes[i]) / closes[i]
        action_token = next_return_to_action_token(next_ret)

        sequences.append(tokens)
        targets.append(action_token)

    return sequences, targets


# ---------------------------------------------------------------------------
# Training loop
# ---------------------------------------------------------------------------
def set_model_training(model):
    """Put model in training mode."""
    model.train()


def set_model_inference(model):
    """Put model in inference (non-training) mode."""
    model.training = False
    for m in model.modules():
        m.training = False


def train_model(
    model: torch.nn.Module,
    train_loader: DataLoader,
    val_loader: DataLoader,
    device: str,
    epochs: int = EPOCHS,
    lr: float = LR,
) -> dict:
    """Train the HRM model for action prediction."""
    optimizer = AdamW(model.parameters(), lr=lr, weight_decay=1e-2)
    scheduler = CosineAnnealingLR(optimizer, T_max=epochs)

    best_val_loss = float("inf")
    best_epoch = -1
    history = {"train_loss": [], "val_loss": [], "val_acc": []}

    for epoch in range(1, epochs + 1):
        # --- Train ---
        set_model_training(model)
        train_loss_sum = 0.0
        train_count = 0

        for seqs, tgts in train_loader:
            seqs = seqs.to(device)   # (B, SEQ_LEN)
            tgts = tgts.to(device).squeeze(-1)  # (B,)

            # Build targets tensor: same as input but last token = action token
            # We predict the action at the last position
            target_seq = seqs.clone()
            target_seq[:, -1] = tgts

            batch = {
                "inputs": seqs,
                "targets": target_seq,
                "puzzle_identifiers": torch.zeros(seqs.size(0), dtype=torch.long, device=device),
            }

            # Create fresh carry for this batch
            carry = model.initial_carry(batch)
            # Move carry tensors to device
            carry.inner_carry.z_H = carry.inner_carry.z_H.to(device)
            carry.inner_carry.z_L = carry.inner_carry.z_L.to(device)
            carry.steps = carry.steps.to(device)
            carry.halted = carry.halted.to(device)
            carry.current_data = {k: v.to(device) for k, v in carry.current_data.items()}

            new_carry, outputs = model(carry, batch)

            # Cross-entropy loss on last position logits vs action token
            logits = outputs["logits"]  # (B, SEQ_LEN, VOCAB_SIZE)
            last_logits = logits[:, -1, :]  # (B, VOCAB_SIZE)
            loss = F.cross_entropy(last_logits, tgts)

            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()

            train_loss_sum += loss.item() * seqs.size(0)
            train_count += seqs.size(0)

        scheduler.step()
        train_loss = train_loss_sum / max(train_count, 1)
        history["train_loss"].append(train_loss)

        # --- Validate ---
        set_model_inference(model)
        val_loss_sum = 0.0
        val_correct = 0
        val_count = 0

        with torch.no_grad():
            for seqs, tgts in val_loader:
                seqs = seqs.to(device)
                tgts = tgts.to(device).squeeze(-1)

                target_seq = seqs.clone()
                target_seq[:, -1] = tgts

                batch = {
                    "inputs": seqs,
                    "targets": target_seq,
                    "puzzle_identifiers": torch.zeros(seqs.size(0), dtype=torch.long, device=device),
                }

                carry = model.initial_carry(batch)
                carry.inner_carry.z_H = carry.inner_carry.z_H.to(device)
                carry.inner_carry.z_L = carry.inner_carry.z_L.to(device)
                carry.steps = carry.steps.to(device)
                carry.halted = carry.halted.to(device)
                carry.current_data = {k: v.to(device) for k, v in carry.current_data.items()}

                new_carry, outputs = model(carry, batch)

                logits = outputs["logits"]
                last_logits = logits[:, -1, :]
                loss = F.cross_entropy(last_logits, tgts)

                val_loss_sum += loss.item() * seqs.size(0)
                preds = last_logits[:, 50:55].argmax(dim=-1) + 50
                val_correct += (preds == tgts).sum().item()
                val_count += seqs.size(0)

        val_loss = val_loss_sum / max(val_count, 1)
        val_acc = val_correct / max(val_count, 1) * 100.0
        history["val_loss"].append(val_loss)
        history["val_acc"].append(val_acc)

        lr_now = scheduler.get_last_lr()[0]
        logger.info(
            "Epoch %3d/%d  train_loss=%.4f  val_loss=%.4f  val_acc=%.1f%%  lr=%.2e",
            epoch, epochs, train_loss, val_loss, val_acc, lr_now,
        )

        if val_loss < best_val_loss:
            best_val_loss = val_loss
            best_epoch = epoch
            torch.save({"model_state_dict": model.state_dict()}, BEST_MODEL_PATH)
            logger.info("  -> New best model saved (epoch %d, val_loss=%.4f)", epoch, val_loss)

    logger.info("Training complete. Best epoch=%d, best_val_loss=%.4f", best_epoch, best_val_loss)
    return history


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main():
    t0 = time.time()
    logger.info("=" * 60)
    logger.info("HRM Real-Data Training Script")
    logger.info("=" * 60)

    # 1. Device
    device = "cuda" if torch.cuda.is_available() else "cpu"
    logger.info("Device: %s", device)
    if device == "cuda":
        logger.info("GPU: %s", torch.cuda.get_device_name(0))
        logger.info("VRAM: %.1f GB", torch.cuda.get_device_properties(0).total_memory / 1e9)

    # 2. Load candles
    candles = load_candles(DB_URL)
    if len(candles) < LOOKBACK + 10:
        logger.error("Not enough candles (%d). Need at least %d.", len(candles), LOOKBACK + 10)
        sys.exit(1)

    # 3. Build dataset
    logger.info("Computing features and building dataset ...")
    sequences, targets = build_dataset(candles)
    logger.info("Total samples: %d", len(sequences))

    # Distribution of action labels
    from collections import Counter
    dist = Counter(targets)
    for tok in sorted(dist.keys()):
        names = {50: "buy_strong", 51: "buy", 52: "hold", 53: "sell", 54: "sell_strong"}
        logger.info("  %s (%d): %d samples (%.1f%%)", names.get(tok, "?"), tok, dist[tok], dist[tok] / len(targets) * 100)

    # 4. Train/val split (80/20, time-based)
    split_idx = int(0.8 * len(sequences))
    train_seqs, train_tgts = sequences[:split_idx], targets[:split_idx]
    val_seqs, val_tgts = sequences[split_idx:], targets[split_idx:]
    logger.info("Train: %d samples, Val: %d samples", len(train_seqs), len(val_tgts))

    train_ds = HRMTradingDataset(train_seqs, train_tgts)
    val_ds = HRMTradingDataset(val_seqs, val_tgts)

    train_loader = DataLoader(train_ds, batch_size=BATCH_SIZE, shuffle=True, drop_last=True)
    val_loader = DataLoader(val_ds, batch_size=BATCH_SIZE, shuffle=False, drop_last=False)

    # 5. Initialize model
    logger.info("Initializing HRM model (hidden=%d, heads=%d, H=%d, L=%d) ...",
                HIDDEN_SIZE, NUM_HEADS, H_LAYERS, L_LAYERS)

    # Build config dict for HierarchicalReasoningModel_ACTV1
    from hrm.models.hrm.hrm_act_v1 import HierarchicalReasoningModel_ACTV1

    config_dict = {
        "batch_size": BATCH_SIZE,
        "seq_len": SEQ_LEN,
        "num_puzzle_identifiers": 1,
        "vocab_size": VOCAB_SIZE,
        "H_cycles": 2,
        "L_cycles": 2,
        "H_layers": H_LAYERS,
        "L_layers": L_LAYERS,
        "hidden_size": HIDDEN_SIZE,
        "expansion": 2.0,
        "num_heads": NUM_HEADS,
        "pos_encodings": "rope",
        "halt_max_steps": 4,
        "halt_exploration_prob": 0.0,
        "forward_dtype": "float32",
    }

    model = HierarchicalReasoningModel_ACTV1(config_dict)
    model.to(device)

    n_params = sum(p.numel() for p in model.parameters())
    logger.info("Model parameters: %d (%.2f M)", n_params, n_params / 1e6)
    if device == "cuda":
        logger.info("GPU memory after init: %.2f MB", torch.cuda.memory_allocated() / 1e6)

    # 6. Train
    history = train_model(model, train_loader, val_loader, device, epochs=EPOCHS, lr=LR)

    # 7. Load best and copy to final path
    if os.path.exists(BEST_MODEL_PATH):
        logger.info("Loading best model from %s", BEST_MODEL_PATH)
        checkpoint = torch.load(BEST_MODEL_PATH, map_location=device, weights_only=True)
        model.load_state_dict(checkpoint["model_state_dict"])

        os.makedirs(os.path.dirname(FINAL_MODEL_PATH), exist_ok=True)
        torch.save({"model_state_dict": model.state_dict()}, FINAL_MODEL_PATH)
        logger.info("Final model saved to %s", FINAL_MODEL_PATH)
    else:
        logger.warning("No best model checkpoint found!")

    # 8. Summary
    elapsed = time.time() - t0
    logger.info("=" * 60)
    logger.info("SUMMARY")
    logger.info("  Candles:     %d", len(candles))
    logger.info("  Samples:     %d (train=%d, val=%d)", len(sequences), len(train_seqs), len(val_tgts))
    logger.info("  Epochs:      %d", EPOCHS)
    logger.info("  Best val_loss: %.4f", min(history["val_loss"]) if history["val_loss"] else float("inf"))
    logger.info("  Best val_acc:  %.1f%%", max(history["val_acc"]) if history["val_acc"] else 0.0)
    logger.info("  Model path:  %s", FINAL_MODEL_PATH)
    logger.info("  Elapsed:     %.1f seconds", elapsed)
    logger.info("=" * 60)


if __name__ == "__main__":
    main()
