#!/usr/bin/env python3
"""Backfill historical OHLCV data from Binance REST API into prices table.

Fetches kline (candlestick) data and inserts into TimescaleDB prices hypertable.
No API key required for public market data.

Usage:
    python scripts/ml/backfill_prices.py [--symbol BTCUSDT] [--interval 1h] [--days 30]
    python scripts/ml/backfill_prices.py --symbol ETHUSDT --interval 15m --days 7
"""

import argparse
import sys
import time
from datetime import datetime, timezone

import psycopg2
import requests

BINANCE_API = "https://api.binance.com/api/v3/klines"

# Interval to milliseconds mapping
INTERVAL_MS = {
    "1m": 60_000,
    "5m": 300_000,
    "15m": 900_000,
    "1h": 3_600_000,
    "4h": 14_400_000,
    "1d": 86_400_000,
}


def fetch_klines(symbol: str, interval: str, start_ms: int, end_ms: int) -> list:
    """Fetch klines from Binance API. Returns list of OHLCV data."""
    params = {
        "symbol": symbol,
        "interval": interval,
        "startTime": start_ms,
        "endTime": end_ms,
        "limit": 1000,
    }

    resp = requests.get(BINANCE_API, params=params, timeout=10)
    resp.raise_for_status()
    return resp.json()


def kline_to_row(symbol: str, kline: list) -> tuple:
    """Convert Binance kline array to a database row tuple."""
    # Binance kline format: [open_time, open, high, low, close, volume, close_time, ...]
    open_time_ms = kline[0]
    open_price = float(kline[1])
    high = float(kline[2])
    low = float(kline[3])
    close = float(kline[4])
    volume = float(kline[5])
    trades = int(kline[8])

    timestamp = datetime.fromtimestamp(open_time_ms / 1000, tz=timezone.utc)

    return (timestamp, symbol, open_price, high, low, close, volume, trades, "binance")


def backfill(
    db_url: str,
    symbol: str,
    interval: str,
    days: int,
) -> int:
    """Fetch historical data and insert into prices table."""
    now_ms = int(time.time() * 1000)
    interval_ms = INTERVAL_MS.get(interval, 3_600_000)
    start_ms = now_ms - (days * 24 * 3600 * 1000)

    print(f"Backfilling {symbol} {interval} for {days} days...")
    print(f"  Start: {datetime.fromtimestamp(start_ms/1000, tz=timezone.utc)}")
    print(f"  End:   {datetime.fromtimestamp(now_ms/1000, tz=timezone.utc)}")

    conn = psycopg2.connect(db_url)
    cur = conn.cursor()

    total_inserted = 0
    current_ms = start_ms

    while current_ms < now_ms:
        batch_end = min(current_ms + 1000 * interval_ms, now_ms)

        try:
            klines = fetch_klines(symbol, interval, current_ms, batch_end)
        except Exception as e:
            print(f"  Error fetching klines: {e}")
            time.sleep(1)
            continue

        if not klines:
            break

        rows = [kline_to_row(symbol, k) for k in klines]

        # Batch insert with ON CONFLICT DO NOTHING
        insert_sql = """
            INSERT INTO prices (time, symbol, open, high, low, close, volume, trades, source)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s)
            ON CONFLICT DO NOTHING
        """

        cur.executemany(insert_sql, rows)
        conn.commit()

        inserted = len(rows)
        total_inserted += inserted

        last_time = rows[-1][0] if rows else "?"
        print(f"  Fetched {inserted} candles up to {last_time}")

        # Move cursor past last kline
        current_ms = klines[-1][0] + interval_ms

        # Rate limit: Binance allows 1200 req/min
        time.sleep(0.1)

    cur.close()
    conn.close()

    print(f"\nDone. Total inserted: {total_inserted} candles for {symbol}")
    return total_inserted


def main():
    parser = argparse.ArgumentParser(description="Backfill historical prices from Binance")
    parser.add_argument("--symbol", default="BTCUSDT", help="Trading pair (default: BTCUSDT)")
    parser.add_argument("--interval", default="1h", help="Candle interval (1m,5m,15m,1h,4h,1d)")
    parser.add_argument("--days", type=int, default=30, help="Days of history (default: 30)")
    parser.add_argument(
        "--db-url",
        default="postgresql://investor:investor@192.168.1.123:15432/investor_os",
        help="PostgreSQL connection URL",
    )
    args = parser.parse_args()

    if args.interval not in INTERVAL_MS:
        print(f"Error: invalid interval '{args.interval}'. Use: {list(INTERVAL_MS.keys())}")
        sys.exit(1)

    total = backfill(args.db_url, args.symbol, args.interval, args.days)

    if total == 0:
        print("WARNING: No data inserted. Check network and symbol name.")
        sys.exit(1)


if __name__ == "__main__":
    main()
