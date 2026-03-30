"""FinBERT financial sentiment analysis model (Sprint 121).

Uses HuggingFace transformers with ProsusAI/finbert.
Supports batch inference and optional ONNX quantization for CPU deployment.
"""

import logging
import time
from functools import lru_cache

logger = logging.getLogger(__name__)

# Sentiment label mapping
LABELS = ["positive", "negative", "neutral"]


class FinBERTPredictor:
    """FinBERT sentiment analysis for financial text."""

    def __init__(self, model_name: str = "ProsusAI/finbert", use_onnx: bool = False):
        self.model_name = model_name
        self.use_onnx = use_onnx
        self.model_version = "1.0.0"
        self._pipeline = None
        self._loaded = False

    def load(self) -> bool:
        """Load the FinBERT model. Downloads on first use (~500MB)."""
        try:
            if self.use_onnx:
                return self._load_onnx()
            return self._load_transformers()
        except Exception as e:
            logger.error("Failed to load FinBERT: %s", e)
            return False

    def _load_transformers(self) -> bool:
        """Load via HuggingFace transformers pipeline."""
        from transformers import pipeline

        self._pipeline = pipeline(
            "text-classification",
            model=self.model_name,
            top_k=None,  # return all label scores
            truncation=True,
            max_length=512,
        )
        self._loaded = True
        logger.info("FinBERT loaded via transformers: %s", self.model_name)
        return True

    def _load_onnx(self) -> bool:
        """Load quantized ONNX model for faster CPU inference."""
        try:
            from optimum.onnxruntime import ORTModelForSequenceClassification
            from transformers import AutoTokenizer, pipeline

            tokenizer = AutoTokenizer.from_pretrained(self.model_name)
            model = ORTModelForSequenceClassification.from_pretrained(
                self.model_name, export=True
            )
            self._pipeline = pipeline(
                "text-classification",
                model=model,
                tokenizer=tokenizer,
                top_k=None,
                truncation=True,
                max_length=512,
            )
            self._loaded = True
            self.model_version = "1.0.0-onnx"
            logger.info("FinBERT loaded via ONNX: %s", self.model_name)
            return True
        except ImportError:
            logger.warning("optimum not available, falling back to transformers")
            return self._load_transformers()

    def predict(self, texts: list[str]) -> list[dict]:
        """Analyze sentiment for a batch of texts.

        Args:
            texts: List of financial text strings (max 16 per batch).

        Returns:
            List of dicts with sentiment, score, confidence per text.
        """
        if not self._loaded or self._pipeline is None:
            raise RuntimeError("FinBERT not loaded — call load() first")

        if len(texts) > 16:
            texts = texts[:16]  # enforce batch limit

        start = time.time()
        raw_results = self._pipeline(texts)
        latency_ms = int((time.time() - start) * 1000)

        results = []
        for i, text_scores in enumerate(raw_results):
            # text_scores is a list of {label, score} dicts
            # Find the dominant sentiment
            best = max(text_scores, key=lambda x: x["score"])
            sentiment = best["label"].lower()

            # Convert to -1..+1 score
            score_map = {s["label"].lower(): s["score"] for s in text_scores}
            pos = score_map.get("positive", 0)
            neg = score_map.get("negative", 0)
            sentiment_score = pos - neg  # -1 to +1

            results.append({
                "text": texts[i][:200],  # truncate for response
                "sentiment": sentiment,
                "score": round(sentiment_score, 4),
                "confidence": round(best["score"], 4),
                "scores": {k: round(v, 4) for k, v in score_map.items()},
            })

        return results, latency_ms

    @property
    def is_loaded(self) -> bool:
        return self._loaded


# Lazy singleton
_instance: FinBERTPredictor | None = None


def get_finbert(use_onnx: bool = False) -> FinBERTPredictor:
    """Get or create the FinBERT singleton."""
    global _instance
    if _instance is None:
        _instance = FinBERTPredictor(use_onnx=use_onnx)
    return _instance
