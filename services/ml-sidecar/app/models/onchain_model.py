"""On-chain analytics model — whale tracking + exchange flow (Wave 2 Task 11).

Combines free public data sources:
1. Binance Futures funding rates — market sentiment from perpetual swaps
2. Binance Futures open interest — total outstanding contracts
3. Blockchain.com — large BTC transactions (whale detection)

All APIs are free and require no API keys.
"""

import logging
import time
from datetime import datetime, timezone

import requests

logger = logging.getLogger(__name__)

# Binance Futures endpoints (no auth required)
BINANCE_FUNDING_URL = "https://fapi.binance.com/fapi/v1/fundingRate"
BINANCE_OI_URL = "https://fapi.binance.com/fapi/v1/openInterest"
BINANCE_TICKER_URL = "https://fapi.binance.com/fapi/v1/ticker/24hr"

# Blockchain.com endpoints (no auth required)
BLOCKCHAIN_LATEST_BLOCK_URL = "https://blockchain.info/latestblock"
BLOCKCHAIN_RAW_BLOCK_URL = "https://blockchain.info/rawblock"

# HTTP timeout in seconds
HTTP_TIMEOUT = 10

# BTC satoshi conversion
SATOSHI_PER_BTC = 1e8


class OnChainAnalyzer:
    """Whale tracking + exchange flow analytics using free public APIs."""

    def __init__(self):
        self.model_version = "1.0.0"
        self._session = requests.Session()
        self._session.headers.update({
            "User-Agent": "InvestorOS/1.0",
            "Accept": "application/json",
        })

    # ------------------------------------------------------------------
    # Funding rates
    # ------------------------------------------------------------------

    def get_funding_rates(self, symbol: str = "BTCUSDT", limit: int = 10) -> dict:
        """Fetch recent perpetual futures funding rates from Binance.

        Positive funding rate = longs pay shorts (bullish bias in market).
        Negative funding rate = shorts pay longs (bearish bias in market).

        Args:
            symbol: Futures trading pair (e.g. BTCUSDT).
            limit: Number of recent funding rate entries (max 1000).

        Returns:
            Dict with rates list, current rate, and sentiment interpretation.
        """
        try:
            resp = self._session.get(
                BINANCE_FUNDING_URL,
                params={"symbol": symbol, "limit": limit},
                timeout=HTTP_TIMEOUT,
            )
            resp.raise_for_status()
            raw_rates = resp.json()
        except Exception as e:
            logger.warning("Failed to fetch funding rates: %s", e)
            return {
                "symbol": symbol,
                "rates": [],
                "current_rate": 0.0,
                "avg_rate": 0.0,
                "sentiment": "unknown",
                "error": str(e),
            }

        rates = []
        for entry in raw_rates:
            rate_val = float(entry.get("fundingRate", 0))
            rates.append({
                "time": entry.get("fundingTime", 0),
                "rate": rate_val,
                "rate_pct": round(rate_val * 100, 4),
            })

        current_rate = rates[-1]["rate"] if rates else 0.0
        avg_rate = sum(r["rate"] for r in rates) / len(rates) if rates else 0.0

        # Interpret sentiment
        if current_rate > 0.0005:
            sentiment = "very_bullish"
        elif current_rate > 0.0001:
            sentiment = "bullish"
        elif current_rate < -0.0005:
            sentiment = "very_bearish"
        elif current_rate < -0.0001:
            sentiment = "bearish"
        else:
            sentiment = "neutral"

        return {
            "symbol": symbol,
            "rates": rates,
            "current_rate": round(current_rate, 6),
            "avg_rate": round(avg_rate, 6),
            "sentiment": sentiment,
        }

    # ------------------------------------------------------------------
    # Open interest (proxy for exchange flow)
    # ------------------------------------------------------------------

    def get_exchange_netflow(self, symbol: str = "BTCUSDT") -> dict:
        """Estimate exchange flow using Binance futures open interest + 24h volume.

        Rising OI + rising price = new longs entering (bullish inflow).
        Rising OI + falling price = new shorts entering (bearish inflow).
        Falling OI = positions closing (outflow).

        This is a simplified proxy; true on-chain netflow requires
        paid APIs like Glassnode or CryptoQuant.

        Args:
            symbol: Futures trading pair.

        Returns:
            Dict with open_interest, 24h volume, and flow interpretation.
        """
        oi_data = {}
        ticker_data = {}

        # Fetch open interest
        try:
            resp = self._session.get(
                BINANCE_OI_URL,
                params={"symbol": symbol},
                timeout=HTTP_TIMEOUT,
            )
            resp.raise_for_status()
            oi_data = resp.json()
        except Exception as e:
            logger.warning("Failed to fetch open interest: %s", e)

        # Fetch 24h ticker for price change context
        try:
            resp = self._session.get(
                BINANCE_TICKER_URL,
                params={"symbol": symbol},
                timeout=HTTP_TIMEOUT,
            )
            resp.raise_for_status()
            ticker_data = resp.json()
        except Exception as e:
            logger.warning("Failed to fetch 24h ticker: %s", e)

        open_interest = float(oi_data.get("openInterest", 0))
        oi_value = open_interest * float(ticker_data.get("lastPrice", 0)) if ticker_data else 0

        price_change_pct = float(ticker_data.get("priceChangePercent", 0))
        volume_24h = float(ticker_data.get("quoteVolume", 0))
        last_price = float(ticker_data.get("lastPrice", 0))

        # Interpret flow direction
        if open_interest > 0 and price_change_pct > 1.0:
            flow_direction = "inflow_bullish"
            interpretation = "Rising OI + rising price: new long positions entering"
        elif open_interest > 0 and price_change_pct < -1.0:
            flow_direction = "inflow_bearish"
            interpretation = "Rising OI + falling price: new short positions entering"
        else:
            flow_direction = "neutral"
            interpretation = "Stable OI: no significant directional flow"

        return {
            "symbol": symbol,
            "open_interest_contracts": round(open_interest, 4),
            "open_interest_value_usd": round(oi_value, 2),
            "last_price": round(last_price, 2),
            "price_change_24h_pct": round(price_change_pct, 2),
            "volume_24h_usd": round(volume_24h, 2),
            "flow_direction": flow_direction,
            "interpretation": interpretation,
        }

    # ------------------------------------------------------------------
    # Whale transactions
    # ------------------------------------------------------------------

    def get_whale_transactions(self, min_btc: float = 100.0) -> dict:
        """Detect large BTC transactions from the latest blockchain block.

        Scans the most recent Bitcoin block for transactions where any
        single output exceeds min_btc threshold.

        Args:
            min_btc: Minimum BTC value per output to qualify as a whale tx.

        Returns:
            Dict with whale transactions, block info, and summary.
        """
        min_satoshi = min_btc * SATOSHI_PER_BTC

        # Step 1: Get latest block hash
        try:
            resp = self._session.get(
                BLOCKCHAIN_LATEST_BLOCK_URL,
                timeout=HTTP_TIMEOUT,
            )
            resp.raise_for_status()
            latest = resp.json()
            block_hash = latest.get("hash", "")
            block_height = latest.get("height", 0)
            block_time = latest.get("time", 0)
        except Exception as e:
            logger.warning("Failed to fetch latest block: %s", e)
            return {
                "block_height": 0,
                "block_hash": "",
                "whale_transactions": [],
                "total_whale_volume_btc": 0.0,
                "whale_tx_count": 0,
                "error": str(e),
            }

        # Step 2: Get raw block data
        whale_txs = []
        try:
            resp = self._session.get(
                f"{BLOCKCHAIN_RAW_BLOCK_URL}/{block_hash}",
                timeout=HTTP_TIMEOUT * 2,
            )
            resp.raise_for_status()
            block = resp.json()

            for tx in block.get("tx", []):
                tx_hash = tx.get("hash", "")
                total_output = 0
                large_outputs = []

                for out in tx.get("out", []):
                    value = out.get("value", 0)
                    if value >= min_satoshi:
                        btc_value = value / SATOSHI_PER_BTC
                        large_outputs.append({
                            "address": out.get("addr", "unknown"),
                            "value_btc": round(btc_value, 8),
                            "value_satoshi": value,
                        })
                        total_output += btc_value

                if large_outputs:
                    whale_txs.append({
                        "tx_hash": tx_hash,
                        "total_large_output_btc": round(total_output, 8),
                        "large_outputs": large_outputs,
                        "n_large_outputs": len(large_outputs),
                    })

        except Exception as e:
            logger.warning("Failed to fetch raw block %s: %s", block_hash[:16], e)
            return {
                "block_height": block_height,
                "block_hash": block_hash,
                "block_time": block_time,
                "whale_transactions": [],
                "total_whale_volume_btc": 0.0,
                "whale_tx_count": 0,
                "error": str(e),
            }

        total_whale_btc = sum(tx["total_large_output_btc"] for tx in whale_txs)

        return {
            "block_height": block_height,
            "block_hash": block_hash,
            "block_time": block_time,
            "min_btc_threshold": min_btc,
            "whale_transactions": whale_txs[:50],  # Cap at 50 to limit payload size
            "total_whale_volume_btc": round(total_whale_btc, 8),
            "whale_tx_count": len(whale_txs),
        }

    # ------------------------------------------------------------------
    # Combined analysis
    # ------------------------------------------------------------------

    def analyze(self, symbol: str = "BTCUSDT", min_whale_btc: float = 100.0) -> dict:
        """Combine all on-chain signals into a bullish/bearish/neutral score.

        Scoring system (range -100 to +100):
          Funding rate contribution: up to +/- 40 points
          Exchange flow (OI direction): up to +/- 30 points
          Whale activity: up to +/- 30 points

        Args:
            symbol: Futures trading pair for Binance data.
            min_whale_btc: Minimum BTC for whale detection.

        Returns:
            Dict with individual signals, combined score, and verdict.
        """
        start = time.time()

        # Gather all signals
        funding = self.get_funding_rates(symbol=symbol, limit=10)
        netflow = self.get_exchange_netflow(symbol=symbol)
        whales = self.get_whale_transactions(min_btc=min_whale_btc)

        # Score 1: Funding rate (-40 to +40)
        funding_score = 0.0
        current_rate = funding.get("current_rate", 0.0)
        if current_rate > 0.001:
            funding_score = 40.0  # Extremely bullish funding
        elif current_rate > 0.0005:
            funding_score = 30.0
        elif current_rate > 0.0001:
            funding_score = 15.0
        elif current_rate < -0.001:
            funding_score = -40.0  # Extremely bearish funding
        elif current_rate < -0.0005:
            funding_score = -30.0
        elif current_rate < -0.0001:
            funding_score = -15.0

        # Score 2: Exchange flow / OI direction (-30 to +30)
        flow_score = 0.0
        flow_direction = netflow.get("flow_direction", "neutral")
        price_change = netflow.get("price_change_24h_pct", 0.0)
        if flow_direction == "inflow_bullish":
            flow_score = min(30.0, abs(price_change) * 6)
        elif flow_direction == "inflow_bearish":
            flow_score = max(-30.0, -abs(price_change) * 6)

        # Score 3: Whale activity (-30 to +30)
        # High whale volume during price increase = bullish accumulation
        # High whale volume during price decrease = bearish distribution
        whale_score = 0.0
        whale_count = whales.get("whale_tx_count", 0)
        whale_volume = whales.get("total_whale_volume_btc", 0.0)

        if whale_count > 0:
            # Scale whale activity (normalized by typical range)
            whale_intensity = min(1.0, whale_volume / 5000.0)  # 5000 BTC = max intensity
            if price_change > 0:
                whale_score = whale_intensity * 30.0  # Bullish accumulation
            elif price_change < 0:
                whale_score = -whale_intensity * 30.0  # Bearish distribution
            else:
                whale_score = whale_intensity * 10.0  # Neutral but active

        # Combined score
        total_score = funding_score + flow_score + whale_score
        total_score = max(-100.0, min(100.0, total_score))

        # Verdict
        if total_score >= 40:
            verdict = "strongly_bullish"
        elif total_score >= 15:
            verdict = "bullish"
        elif total_score <= -40:
            verdict = "strongly_bearish"
        elif total_score <= -15:
            verdict = "bearish"
        else:
            verdict = "neutral"

        # Confidence based on data availability
        data_sources_ok = 0
        if "error" not in funding:
            data_sources_ok += 1
        if netflow.get("open_interest_contracts", 0) > 0:
            data_sources_ok += 1
        if "error" not in whales:
            data_sources_ok += 1
        confidence = round(data_sources_ok / 3.0, 2)

        latency_ms = int((time.time() - start) * 1000)

        return {
            "symbol": symbol,
            "score": round(total_score, 1),
            "verdict": verdict,
            "confidence": confidence,
            "signals": {
                "funding": {
                    "score": round(funding_score, 1),
                    "current_rate": funding.get("current_rate", 0.0),
                    "avg_rate": funding.get("avg_rate", 0.0),
                    "sentiment": funding.get("sentiment", "unknown"),
                },
                "exchange_flow": {
                    "score": round(flow_score, 1),
                    "direction": flow_direction,
                    "open_interest_contracts": netflow.get("open_interest_contracts", 0),
                    "open_interest_value_usd": netflow.get("open_interest_value_usd", 0),
                    "price_change_24h_pct": price_change,
                    "volume_24h_usd": netflow.get("volume_24h_usd", 0),
                },
                "whale_activity": {
                    "score": round(whale_score, 1),
                    "whale_tx_count": whale_count,
                    "total_whale_volume_btc": whale_volume,
                    "block_height": whales.get("block_height", 0),
                },
            },
            "latency_ms": latency_ms,
            "model_version": self.model_version,
            "timestamp": datetime.now(timezone.utc).isoformat(),
        }
