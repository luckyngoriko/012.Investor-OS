"""Trading adapter for sapientinc/HRM (Apache 2.0).

Converts OHLCV market data into tokenized sequences for HRM inference,
and maps HRM output tokens back to trading actions.
"""

import logging
import time

import numpy as np
import torch

from hrm.models.hrm.hrm_act_v1 import (
    HierarchicalReasoningModel_ACTV1,
    HierarchicalReasoningModel_ACTV1Config,
)

logger = logging.getLogger(__name__)

# Trading action vocabulary (tokens 50-56)
ACTION_TOKENS = {
    50: "buy_strong",
    51: "buy",
    52: "hold",
    53: "sell",
    54: "sell_strong",
    55: "stop_loss",
    56: "take_profit",
}

# Feature tokenization bins
PRICE_CHANGE_BINS = [-0.05, -0.02, -0.005, 0.0, 0.005, 0.02, 0.05]
PRICE_CHANGE_TOKENS = list(range(0, 8))

RSI_TOKENS = {"oversold": 8, "neutral": 9, "overbought": 10}
MACD_TOKENS = {"bullish": 11, "bearish": 12}
VOLUME_TOKENS = list(range(13, 18))
BB_TOKENS = {"below": 18, "inside": 19, "above": 20}
TREND_TOKENS = {"down": 21, "flat": 22, "up": 23}

VOCAB_SIZE = 64
SEQ_LEN = 48


def tokenize_candle(pct_change, rsi, macd_hist, volume_zscore, bb_position, trend):
    """Convert a single candle's features to tokens."""
    tokens = []

    idx = int(np.searchsorted(PRICE_CHANGE_BINS, pct_change))
    tokens.append(PRICE_CHANGE_TOKENS[min(idx, len(PRICE_CHANGE_TOKENS) - 1)])

    if rsi < 30:
        tokens.append(RSI_TOKENS["oversold"])
    elif rsi > 70:
        tokens.append(RSI_TOKENS["overbought"])
    else:
        tokens.append(RSI_TOKENS["neutral"])

    tokens.append(MACD_TOKENS["bullish"] if macd_hist > 0 else MACD_TOKENS["bearish"])

    vol_idx = int(np.clip((volume_zscore + 2) / 4 * 5, 0, 4))
    tokens.append(VOLUME_TOKENS[vol_idx])

    if bb_position < 0.2:
        tokens.append(BB_TOKENS["below"])
    elif bb_position > 0.8:
        tokens.append(BB_TOKENS["above"])
    else:
        tokens.append(BB_TOKENS["inside"])

    if trend < -0.01:
        tokens.append(TREND_TOKENS["down"])
    elif trend > 0.01:
        tokens.append(TREND_TOKENS["up"])
    else:
        tokens.append(TREND_TOKENS["flat"])

    return tokens


def ohlcv_to_sequence(features_list, seq_len=SEQ_LEN):
    """Convert a list of feature dicts to a padded token sequence."""
    all_tokens = []
    for f in features_list:
        tokens = tokenize_candle(
            pct_change=f.get("price_change_pct_5", 0.0),
            rsi=f.get("rsi_14", 50.0),
            macd_hist=f.get("macd_histogram", 0.0),
            volume_zscore=f.get("volume_change_pct", 0.0),
            bb_position=f.get("bb_position", 0.5),
            trend=f.get("price_change_pct_20", 0.0),
        )
        all_tokens.extend(tokens)

    if len(all_tokens) > seq_len:
        all_tokens = all_tokens[-seq_len:]
    elif len(all_tokens) < seq_len:
        all_tokens = [0] * (seq_len - len(all_tokens)) + all_tokens

    return torch.LongTensor(all_tokens)


def decode_action(token):
    return ACTION_TOKENS.get(token, "hold")


def action_to_direction(action):
    if action in ("buy_strong", "buy"):
        return "long"
    elif action in ("sell_strong", "sell"):
        return "short"
    return "neutral"


class HRMTradingModel:
    """Wrapper around sapientinc/HRM for trading inference."""

    def __init__(self, device="cuda"):
        self.device = device if torch.cuda.is_available() else "cpu"
        self.model = None
        self.config = None
        self.model_version = "sapientinc-v1"
        self._loaded = False

    def init_model(self, hidden_size=256, num_heads=8, h_layers=3, l_layers=3, h_cycles=2, l_cycles=2):
        """Initialize HRM model with trading config."""
        self.config = HierarchicalReasoningModel_ACTV1Config(
            batch_size=1,
            seq_len=SEQ_LEN,
            num_puzzle_identifiers=1,
            vocab_size=VOCAB_SIZE,
            H_cycles=h_cycles,
            L_cycles=l_cycles,
            H_layers=h_layers,
            L_layers=l_layers,
            hidden_size=hidden_size,
            expansion=2.0,
            num_heads=num_heads,
            pos_encodings="rope",
            halt_max_steps=4,
            halt_exploration_prob=0.0,
            forward_dtype="float32",
        )

        self.model = HierarchicalReasoningModel_ACTV1(self.config)
        self.model.to(self.device)
        self.model.eval()
        self._loaded = True

        n_params = sum(p.numel() for p in self.model.parameters())
        vram_gb = torch.cuda.memory_allocated() / 1e9 if torch.cuda.is_available() else 0
        logger.info("HRM initialized: %d params, device=%s, VRAM=%.2fGB", n_params, self.device, vram_gb)
        return n_params

    def load(self, path):
        """Load trained weights."""
        try:
            checkpoint = torch.load(path, map_location=self.device, weights_only=True)
            self.model.load_state_dict(checkpoint["model_state_dict"])
            self.model.eval()
            logger.info("HRM weights loaded from %s", path)
            return True
        except Exception as e:
            logger.warning("HRM load failed: %s — using random weights", e)
            return False

    def save(self, path):
        """Save model weights."""
        torch.save({"model_state_dict": self.model.state_dict()}, path)
        logger.info("HRM saved to %s", path)

    def predict(self, features_list):
        """Run inference on a list of feature dicts."""
        if not self._loaded:
            raise RuntimeError("HRM not initialized")

        start = time.time()

        input_seq = ohlcv_to_sequence(features_list, SEQ_LEN)
        inputs = input_seq.unsqueeze(0).to(self.device)

        batch = {
            "inputs": inputs,
            "targets": inputs.clone(),
            "puzzle_identifiers": torch.zeros(1, dtype=torch.long, device=self.device),
        }

        with torch.no_grad():
            outputs = self.model(batch)

        logits = outputs["logits"]
        last_logits = logits[0, -1, :]

        action_logits = last_logits[50:57]
        action_idx = action_logits.argmax().item()
        action_token = 50 + action_idx
        action = decode_action(action_token)
        direction = action_to_direction(action)

        action_probs = torch.softmax(action_logits, dim=0)
        confidence = float(action_probs[action_idx])

        latency_ms = int((time.time() - start) * 1000)

        return {
            "action": action,
            "direction": direction,
            "confidence": round(confidence, 4),
            "action_probs": {
                decode_action(50 + i): round(float(p), 4)
                for i, p in enumerate(action_probs)
            },
            "latency_ms": latency_ms,
            "model_version": self.model_version,
            "device": self.device,
        }

    @property
    def is_loaded(self):
        return self._loaded
