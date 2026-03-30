"""Chronos-2 zero-shot time series forecasting model (Sprint 129).

Amazon's encoder-only TSFM for univariate/multivariate zero-shot forecasting.
120M parameters, fits easily on RTX 3090, supports CPU inference.

HuggingFace: amazon/chronos-t5-base (or chronos-bolt-base for speed)
"""

import logging
import time

import numpy as np
import torch

logger = logging.getLogger(__name__)


class ChronosPredictor:
    """Chronos-2 zero-shot time series forecaster."""

    def __init__(self, model_id: str = "amazon/chronos-t5-base"):
        self.model_id = model_id
        self.model_version = "1.0.0"
        self.pipeline = None
        self._loaded = False
        self.device = "cuda" if torch.cuda.is_available() else "cpu"

    def load(self) -> bool:
        """Load Chronos pipeline from HuggingFace."""
        try:
            from chronos import ChronosPipeline

            logger.info("Loading Chronos-2 from %s on %s...", self.model_id, self.device)

            self.pipeline = ChronosPipeline.from_pretrained(
                self.model_id,
                device_map=self.device,
                torch_dtype=torch.float16 if self.device == "cuda" else torch.float32,
            )

            self._loaded = True
            vram = torch.cuda.memory_allocated() / 1e9 if torch.cuda.is_available() else 0
            logger.info("Chronos-2 loaded. VRAM used: %.1f GB", vram)
            return True

        except Exception as e:
            logger.error("Failed to load Chronos-2: %s", e)
            return False

    def predict(self, data: list[float], horizon: int = 12) -> dict:
        """Generate zero-shot forecast for any time series.

        Args:
            data: Historical values (prices, returns, or any numeric series).
            horizon: Number of future steps to forecast.

        Returns:
            Dict with point forecasts, confidence intervals, and metadata.
        """
        if not self._loaded or self.pipeline is None:
            raise RuntimeError("Chronos-2 not loaded")

        start = time.time()

        context = torch.tensor(data, dtype=torch.float32).unsqueeze(0)

        # Chronos returns quantile forecasts
        forecast = self.pipeline.predict(
            context,
            prediction_length=horizon,
            num_samples=20,
        )

        # forecast shape: [batch, num_samples, horizon]
        samples = forecast.numpy()[0]  # [num_samples, horizon]

        point_forecast = np.median(samples, axis=0).tolist()
        lower_bound = np.percentile(samples, 10, axis=0).tolist()
        upper_bound = np.percentile(samples, 90, axis=0).tolist()

        latency_ms = int((time.time() - start) * 1000)

        # Confidence from prediction interval width
        interval_width = np.mean(np.array(upper_bound) - np.array(lower_bound))
        data_range = max(data) - min(data) if len(data) > 1 else 1.0
        relative_width = interval_width / data_range if data_range > 0 else 1.0
        confidence = max(0.2, min(0.95, 1.0 - relative_width))

        return {
            "forecasts": [round(v, 4) for v in point_forecast],
            "lower_bound": [round(v, 4) for v in lower_bound],
            "upper_bound": [round(v, 4) for v in upper_bound],
            "confidence": round(confidence, 4),
            "latency_ms": latency_ms,
            "model_version": self.model_version,
            "device": self.device,
        }

    @property
    def is_loaded(self) -> bool:
        return self._loaded


_instance: ChronosPredictor | None = None

def get_chronos() -> ChronosPredictor:
    global _instance
    if _instance is None:
        _instance = ChronosPredictor()
    return _instance
