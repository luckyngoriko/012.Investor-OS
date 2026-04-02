"""Binance broker integration endpoints (Phase E1).

Provides read-only account access and paper trading capability.
Requires BINANCE_API_KEY and BINANCE_API_SECRET env vars for real account access.
Without keys, returns demo data.

Endpoints:
  GET /v1/broker/account — account balance + positions
  GET /v1/broker/prices — live prices for tracked symbols
  POST /v1/broker/paper-trade — execute paper trade (logged, not sent to exchange)
"""

import hashlib
import hmac
import logging
import os
import time
from datetime import datetime, timezone

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, Field

import requests

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/v1/broker", tags=["broker"])

BINANCE_API = "https://api.binance.com"
API_KEY = os.environ.get("BINANCE_API_KEY", "")
API_SECRET = os.environ.get("BINANCE_API_SECRET", "")

TRACKED_SYMBOLS = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT"]


def _binance_signed_request(endpoint: str, params: dict = None) -> dict:
    """Make an authenticated Binance API request."""
    if not API_KEY or not API_SECRET:
        raise HTTPException(status_code=503, detail="Binance API keys not configured")

    params = params or {}
    params["timestamp"] = int(time.time() * 1000)

    query = "&".join(f"{k}={v}" for k, v in sorted(params.items()))
    signature = hmac.new(API_SECRET.encode(), query.encode(), hashlib.sha256).hexdigest()
    query += f"&signature={signature}"

    resp = requests.get(
        f"{BINANCE_API}{endpoint}?{query}",
        headers={"X-MBX-APIKEY": API_KEY},
        timeout=10,
    )
    resp.raise_for_status()
    return resp.json()


@router.get("/account")
async def get_account():
    """Get account balance and positions."""
    if not API_KEY:
        # Demo mode — return simulated account
        return {
            "mode": "demo",
            "balances": [
                {"asset": "USDT", "free": "10000.00", "locked": "0.00"},
                {"asset": "BTC", "free": "0.15", "locked": "0.00"},
                {"asset": "ETH", "free": "2.50", "locked": "0.00"},
            ],
            "note": "Set BINANCE_API_KEY and BINANCE_API_SECRET for real account data",
        }

    try:
        data = _binance_signed_request("/api/v3/account")
        # Filter non-zero balances
        balances = [
            {"asset": b["asset"], "free": b["free"], "locked": b["locked"]}
            for b in data.get("balances", [])
            if float(b["free"]) > 0 or float(b["locked"]) > 0
        ]
        return {"mode": "live", "balances": balances}
    except HTTPException:
        raise
    except Exception as e:
        logger.error("Binance account request failed: %s", e)
        raise HTTPException(status_code=502, detail=f"Binance API error: {e}")


@router.get("/prices")
async def get_live_prices():
    """Get live prices for tracked symbols."""
    try:
        symbols_param = "[" + ",".join(f'"{s}"' for s in TRACKED_SYMBOLS) + "]"
        resp = requests.get(
            f"{BINANCE_API}/api/v3/ticker/24hr?symbols={symbols_param}",
            timeout=5,
        )
        resp.raise_for_status()

        prices = []
        for t in resp.json():
            prices.append({
                "symbol": t["symbol"],
                "price": float(t["lastPrice"]),
                "change_24h_pct": float(t["priceChangePercent"]),
                "volume_24h": float(t["quoteVolume"]),
                "high_24h": float(t["highPrice"]),
                "low_24h": float(t["lowPrice"]),
            })

        return {"prices": prices, "timestamp": datetime.now(timezone.utc).isoformat()}
    except Exception as e:
        logger.error("Price fetch failed: %s", e)
        raise HTTPException(status_code=502, detail=str(e))


class PaperTradeRequest(BaseModel):
    """Paper trade request."""
    symbol: str
    side: str = Field(..., description="BUY or SELL")
    quantity: float
    reason: str = ""


@router.post("/paper-trade")
async def execute_paper_trade(body: PaperTradeRequest):
    """Execute a paper trade (logged but not sent to exchange)."""
    # Get current price
    try:
        resp = requests.get(
            f"{BINANCE_API}/api/v3/ticker/price?symbol={body.symbol}",
            timeout=5,
        )
        price_data = resp.json()
        current_price = float(price_data["price"])
    except Exception:
        current_price = 0.0

    trade = {
        "id": f"paper_{int(time.time())}",
        "symbol": body.symbol,
        "side": body.side.upper(),
        "quantity": body.quantity,
        "price": current_price,
        "value": round(current_price * body.quantity, 2),
        "reason": body.reason,
        "status": "FILLED_PAPER",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "mode": "paper",
    }

    logger.info("Paper trade: %s %s %.4f %s @ %.2f = $%.2f",
                body.side, body.symbol, body.quantity, body.symbol,
                current_price, trade["value"])

    return {"success": True, "trade": trade}
