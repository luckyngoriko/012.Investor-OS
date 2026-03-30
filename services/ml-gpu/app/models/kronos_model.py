"""Kronos financial K-line prediction model (Sprint 128).

First foundation model pre-trained specifically on financial OHLCV data.
12 billion K-line records from 45 global exchanges.
Accepted at AAAI 2026.

HuggingFace: NeoQuasar/Kronos-base
"""

import logging
import time

import torch

logger = logging.getLogger(__name__)


class KronosPredictor:
    """Kronos OHLCV K-line prediction model."""

    def __init__(self, model_id: str = "NeoQuasar/Kronos-base"):
        self.model_id = model_id
        self.model_version = "1.0.0"
        self.model = None
        self.tokenizer = None
        self._loaded = False
        self.device = "cuda" if torch.cuda.is_available() else "cpu"

    def load(self) -> bool:
        """Load Kronos model from HuggingFace Hub."""
        try:
            from transformers import AutoModelForCausalLM, AutoTokenizer

            logger.info("Loading Kronos from %s on %s...", self.model_id, self.device)

            self.tokenizer = AutoTokenizer.from_pretrained(
                self.model_id, trust_remote_code=True
            )
            self.model = AutoModelForCausalLM.from_pretrained(
                self.model_id,
                trust_remote_code=True,
                torch_dtype=torch.float16 if self.device == "cuda" else torch.float32,
            ).to(self.device)

            self.model.eval()
            self._loaded = True

            vram = torch.cuda.memory_allocated() / 1e9 if torch.cuda.is_available() else 0
            logger.info("Kronos loaded. VRAM used: %.1f GB", vram)
            return True

        except Exception as e:
            logger.error("Failed to load Kronos: %s", e)
            return False

    def predict(self, price_series: list[float], horizon: int = 12) -> dict:
        """Predict next K-line values from price history.

        Args:
            price_series: Historical close prices.
            horizon: Number of future candles to predict.

        Returns:
            Dict with forecasts, confidence, and metadata.
        """
        if not self._loaded:
            raise RuntimeError("Kronos not loaded")

        start = time.time()

        # Normalize input prices
        base_price = price_series[-1]
        normalized = [p / base_price for p in price_series]

        # Prepare input as text (Kronos uses tokenized K-line sequences)
        input_text = " ".join(f"{v:.6f}" for v in normalized[-128:])

        with torch.no_grad():
            inputs = self.tokenizer(input_text, return_tensors="pt").to(self.device)
            outputs = self.model.generate(
                **inputs,
                max_new_tokens=horizon * 4,
                do_sample=False,
                temperature=1.0,
            )

        # Decode output tokens back to price values
        output_text = self.tokenizer.decode(outputs[0], skip_special_tokens=True)
        tokens = output_text.split()

        forecasts = []
        for t in tokens[-horizon:]:
            try:
                val = float(t) * base_price
                forecasts.append(round(val, 2))
            except ValueError:
                continue

        # Pad if insufficient forecasts
        while len(forecasts) < horizon:
            forecasts.append(forecasts[-1] if forecasts else base_price)

        latency_ms = int((time.time() - start) * 1000)

        # Confidence based on forecast stability
        if len(forecasts) > 1:
            pct_changes = [abs(forecasts[i] - forecasts[i - 1]) / forecasts[i - 1]
                           for i in range(1, len(forecasts)) if forecasts[i - 1] != 0]
            avg_change = sum(pct_changes) / len(pct_changes) if pct_changes else 0
            confidence = max(0.3, min(0.95, 1.0 - avg_change * 10))
        else:
            confidence = 0.5

        return {
            "forecasts": forecasts[:horizon],
            "confidence": round(confidence, 4),
            "latency_ms": latency_ms,
            "model_version": self.model_version,
            "device": self.device,
        }

    @property
    def is_loaded(self) -> bool:
        return self._loaded


_instance: KronosPredictor | None = None

def get_kronos() -> KronosPredictor:
    global _instance
    if _instance is None:
        _instance = KronosPredictor()
    return _instance
