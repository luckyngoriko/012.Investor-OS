"""FinGPT advanced financial NLP model (Sprint 131).

Open-source financial LLM with QLoRA 4-bit quantization.
Capabilities: sentiment, entity extraction, event classification, summarization.

Uses PEFT + bitsandbytes for memory-efficient inference on RTX 3090.
"""

import logging
import time

import torch

logger = logging.getLogger(__name__)


class FinGPTPredictor:
    """FinGPT with QLoRA for advanced financial text analysis."""

    def __init__(self, model_id: str = "FinGPT/fingpt-sentiment_llama2-13b_lora"):
        self.model_id = model_id
        self.model_version = "1.0.0"
        self.model = None
        self.tokenizer = None
        self._loaded = False
        self.device = "cuda" if torch.cuda.is_available() else "cpu"

    def load(self) -> bool:
        """Load FinGPT with 4-bit quantization."""
        try:
            from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig

            logger.info("Loading FinGPT from %s with 4-bit quantization...", self.model_id)

            quantization_config = BitsAndBytesConfig(
                load_in_4bit=True,
                bnb_4bit_compute_dtype=torch.float16,
                bnb_4bit_use_double_quant=True,
                bnb_4bit_quant_type="nf4",
            )

            self.tokenizer = AutoTokenizer.from_pretrained(
                self.model_id, trust_remote_code=True
            )
            if self.tokenizer.pad_token is None:
                self.tokenizer.pad_token = self.tokenizer.eos_token

            self.model = AutoModelForCausalLM.from_pretrained(
                self.model_id,
                quantization_config=quantization_config,
                device_map="auto",
                trust_remote_code=True,
            )

            self.model.eval()
            self._loaded = True

            vram = torch.cuda.memory_allocated() / 1e9 if torch.cuda.is_available() else 0
            logger.info("FinGPT loaded. VRAM used: %.1f GB", vram)
            return True

        except Exception as e:
            logger.error("Failed to load FinGPT: %s", e)
            return False

    def predict(self, texts: list[str], task: str = "sentiment") -> tuple[list[dict], int]:
        """Analyze financial texts.

        Args:
            texts: List of financial text strings (max 8).
            task: Analysis task — sentiment, summarize, entities, or qa.

        Returns:
            Tuple of (results list, latency_ms).
        """
        if not self._loaded or self.model is None:
            raise RuntimeError("FinGPT not loaded")

        texts = texts[:8]  # batch limit
        start = time.time()

        prompts = [self._build_prompt(text, task) for text in texts]
        results = []

        for i, prompt in enumerate(prompts):
            inputs = self.tokenizer(
                prompt,
                return_tensors="pt",
                truncation=True,
                max_length=512,
            ).to(self.device)

            with torch.no_grad():
                outputs = self.model.generate(
                    **inputs,
                    max_new_tokens=128,
                    do_sample=False,
                    temperature=1.0,
                )

            response = self.tokenizer.decode(
                outputs[0][inputs["input_ids"].shape[1]:],
                skip_special_tokens=True,
            ).strip()

            results.append({
                "text": texts[i][:200],
                "task": task,
                "response": response[:500],
                "confidence": 0.7,
            })

        latency_ms = int((time.time() - start) * 1000)
        return results, latency_ms

    def _build_prompt(self, text: str, task: str) -> str:
        """Build task-specific prompt for FinGPT."""
        sentiment_choices = "negative, neutral, positive"
        prompts = {
            "sentiment": (
                f"Instruction: What is the sentiment of this financial news? "
                f"Please choose an answer from [{sentiment_choices}].\n"
                f"Input: {text}\nAnswer: "
            ),
            "summarize": (
                f"Instruction: Summarize this financial text in one sentence.\n"
                f"Input: {text}\nSummary: "
            ),
            "entities": (
                f"Instruction: Extract financial entities (companies, tickers, amounts) from this text.\n"
                f"Input: {text}\nEntities: "
            ),
            "qa": (
                f"Instruction: Answer the following financial question.\n"
                f"Input: {text}\nAnswer: "
            ),
        }
        return prompts.get(task, prompts["sentiment"])

    @property
    def is_loaded(self) -> bool:
        return self._loaded


_instance: FinGPTPredictor | None = None

def get_fingpt() -> FinGPTPredictor:
    global _instance
    if _instance is None:
        _instance = FinGPTPredictor()
    return _instance
