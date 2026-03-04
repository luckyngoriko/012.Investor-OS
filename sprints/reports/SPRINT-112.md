# SPRINT-112 Report: Live Market Data Feed — WebSocket Ingestion

**Status:** COMPLETE
**Date:** 2026-03-04

## Summary

Replaced stubbed WebSocket implementation with real `tokio-tungstenite`
connections. The streaming module now connects to Binance public WebSocket
streams, deserializes trade and bookTicker messages, supports dynamic
subscription management, and reconnects with exponential backoff + jitter.

## Changes

### `src/streaming/mod.rs`

- `MarketTick.side` changed from `Side` to `Option<Side>`
- Added `StreamingConfig` struct (latency, cooldown, dedup, window, threshold)
- Added `StreamingEngine::new_with_config()` returning `(engine, tick_sender)`
- Added `register_symbol()`, `get_orderbook()`, `stop()`
- Background tick processing loop: updates orderbooks, runs TradeAnalyzer, emits signals
- Re-exported `WebSocketManager`, `ConnectionConfig`, `ConnectionState`, etc.

### `src/streaming/websocket.rs`

- Added `BinanceTrade`, `BinanceBookTicker`, `BinanceCombinedStream`, `BinanceSubscription` serde structs
- Added `parse_binance_message()` — tries combined envelope, then individual trade, then bookTicker
- Real `connect()` using `tokio_tungstenite::connect_async()` with timeout
- Message read loop: Text → parse → tick_tx, Ping → Pong, Close → break
- `SubscriptionManager` — tracks subscribed streams, generates sub/unsub JSON
- `backoff_with_jitter()` — exponential backoff capped at 60s with fastrand jitter
- `build_ws_url()` — single-symbol direct path, multi-symbol combined stream

### `src/streaming/trade_analyzer.rs`

- Updated test helper to use `side: Some(side)`

### `src/config/mod.rs`

- Added `WsStreamingConfig` to `AppConfig` (enabled, exchange, symbols, reconnect, heartbeat, timeout, latency)

### `tests/sprint112_websocket_ingestion_test.rs` (new)

- 17 tests with mock WS server via `TcpListener` + `tokio_tungstenite::accept_async()`
- Covers: connect+receive, parse formats, reconnection, subscribe/unsubscribe JSON, graceful shutdown, invalid message resilience, backoff bounds, URL builder, tick injection, combined bookTicker

## Gate Results

| Gate                                 | Result     |
| ------------------------------------ | ---------- |
| `cargo clippy -- -D warnings`        | 0 warnings |
| `cargo test --lib`                   | 356 passed |
| `sprint24_streaming_test`            | 14 passed  |
| `sprint112_websocket_ingestion_test` | 17 passed  |
