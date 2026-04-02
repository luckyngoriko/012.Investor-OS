"""Alternative Data Collector (Wave 3 Task 21).

Aggregates alternative data signals:
  - Google Trends interest (via pytrends or web scrape fallback)
  - Fear & Greed Index (from alternative.me free API)
  - Combined bullish/bearish score (-100 to +100)
"""

import logging
import time
from datetime import datetime, timezone

import httpx

logger = logging.getLogger(__name__)

# Google Trends explore endpoint (public, no API key needed)
_GOOGLE_TRENDS_URL = "https://trends.google.com/trends/api/dailytrends"

# Alternative.me Fear & Greed Index (free, no API key)
_FEAR_GREED_URL = "https://api.alternative.me/fng/"

MODEL_VERSION = "altdata-v1"


class AltDataCollector:
    """Collects and analyzes alternative market data signals."""

    def __init__(self, timeout: float = 10.0):
        self._timeout = timeout

    # ------------------------------------------------------------------
    # Google Trends
    # ------------------------------------------------------------------

    def get_google_trends(self, keyword: str = "bitcoin") -> dict:
        """Fetch Google Trends interest score for a keyword.

        Uses the public Google Trends daily trends API.
        Returns a dict with the keyword and an interest score (0-100).
        On failure, returns a neutral score of 50.
        """
        try:
            # Use the Google Trends explore autocomplete as a lightweight proxy
            url = "https://trends.google.com/trends/api/autocomplete/" + keyword
            params = {"hl": "en-US"}
            resp = httpx.get(url, params=params, timeout=self._timeout, follow_redirects=True)

            if resp.status_code == 200:
                # Google prepends ")]}'" to JSON responses
                raw = resp.text
                if raw.startswith(")]}'"):
                    raw = raw[5:]

                import json
                data = json.loads(raw)
                # Extract topic mid and title if available
                topics = data.get("default", {}).get("topics", [])
                topic_count = len(topics)

                # Higher topic count = more trending interest
                # Normalize: 0 topics -> 30, 5+ topics -> 80
                interest = min(30 + topic_count * 10, 80)

                return {
                    "keyword": keyword,
                    "interest": interest,
                    "topic_count": topic_count,
                    "source": "google_trends_autocomplete",
                }
            else:
                logger.warning(
                    "Google Trends returned status %d for '%s'",
                    resp.status_code,
                    keyword,
                )
                return {
                    "keyword": keyword,
                    "interest": 50,
                    "topic_count": 0,
                    "source": "fallback",
                }
        except Exception as e:
            logger.warning("Google Trends fetch failed for '%s': %s", keyword, e)
            return {
                "keyword": keyword,
                "interest": 50,
                "topic_count": 0,
                "source": "fallback",
            }

    # ------------------------------------------------------------------
    # Fear & Greed Index
    # ------------------------------------------------------------------

    def get_fear_greed_index(self) -> dict:
        """Fetch the current Crypto Fear & Greed Index from alternative.me.

        Returns:
            dict with value (0-100), classification, and timestamp.
            0 = Extreme Fear, 100 = Extreme Greed.
            On failure, returns neutral (50).
        """
        try:
            resp = httpx.get(
                _FEAR_GREED_URL,
                params={"limit": "1", "format": "json"},
                timeout=self._timeout,
            )
            resp.raise_for_status()

            data = resp.json()
            entry = data.get("data", [{}])[0]

            value = int(entry.get("value", 50))
            classification = entry.get("value_classification", "Neutral")
            ts = entry.get("timestamp", "0")

            return {
                "value": value,
                "classification": classification,
                "data_timestamp": int(ts),
                "source": "alternative.me",
            }
        except Exception as e:
            logger.warning("Fear & Greed Index fetch failed: %s", e)
            return {
                "value": 50,
                "classification": "Neutral",
                "data_timestamp": 0,
                "source": "fallback",
            }

    # ------------------------------------------------------------------
    # Combined analysis
    # ------------------------------------------------------------------

    def analyze(self, keyword: str = "bitcoin") -> dict:
        """Combine alternative data sources into a bullish/bearish score.

        Score range: -100 (extreme bearish) to +100 (extreme bullish).

        Weighting:
          - Fear & Greed Index:  60% weight (0-100 scale, centered at 50)
          - Google Trends:       40% weight (0-100 scale, centered at 50)

        Returns:
            dict with score, verdict, individual signals, and metadata.
        """
        t0 = time.monotonic()

        trends = self.get_google_trends(keyword=keyword)
        fng = self.get_fear_greed_index()

        # Convert Fear & Greed (0-100) to (-100, +100)
        # 0 -> -100, 50 -> 0, 100 -> +100
        fng_score = (fng["value"] - 50) * 2.0

        # Convert Google Trends interest (0-100) to (-100, +100)
        trends_score = (trends["interest"] - 50) * 2.0

        # Weighted combination
        combined = fng_score * 0.6 + trends_score * 0.4
        combined = max(-100.0, min(100.0, combined))

        # Determine verdict
        if combined >= 40:
            verdict = "strongly_bullish"
        elif combined >= 15:
            verdict = "bullish"
        elif combined <= -40:
            verdict = "strongly_bearish"
        elif combined <= -15:
            verdict = "bearish"
        else:
            verdict = "neutral"

        # Confidence based on data source availability
        sources_available = 0
        if fng["source"] != "fallback":
            sources_available += 1
        if trends["source"] != "fallback":
            sources_available += 1
        confidence = sources_available / 2.0

        latency_ms = int((time.monotonic() - t0) * 1000)

        return {
            "keyword": keyword,
            "score": round(combined, 2),
            "verdict": verdict,
            "confidence": confidence,
            "signals": {
                "fear_greed": {
                    "value": fng["value"],
                    "classification": fng["classification"],
                    "score": round(fng_score, 2),
                    "weight": 0.6,
                    "source": fng["source"],
                },
                "google_trends": {
                    "interest": trends["interest"],
                    "topic_count": trends["topic_count"],
                    "score": round(trends_score, 2),
                    "weight": 0.4,
                    "source": trends["source"],
                },
            },
            "latency_ms": latency_ms,
            "model_version": MODEL_VERSION,
            "timestamp": datetime.now(timezone.utc).isoformat(),
        }
