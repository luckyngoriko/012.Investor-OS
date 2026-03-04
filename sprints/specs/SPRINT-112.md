# SPRINT-112: Live Market Data Feed — WebSocket Ingestion

## Objective

Replace stubbed WebSocket connection with real `tokio-tungstenite` connections
targeting Binance public streams. Add Binance message deserialization,
reconnection with exponential backoff + jitter, dynamic subscription
management, and a mock WS server for testing.

## Work Packages

| WP  | Title                            | Files Changed                               |
| --- | -------------------------------- | ------------------------------------------- |
| 1   | MarketTick.side → Option\<Side\> | streaming/mod.rs, trade_analyzer.rs         |
| 2   | Binance Message Types + Parsing  | streaming/websocket.rs                      |
| 3   | Real WebSocket Connection        | streaming/websocket.rs                      |
| 4   | Reconnection + Subscription Mgmt | streaming/websocket.rs                      |
| 5   | StreamingEngine Extension        | streaming/mod.rs                            |
| 6   | Streaming Configuration          | config/mod.rs                               |
| 7   | Integration Tests                | tests/sprint112_websocket_ingestion_test.rs |
| 8   | Gate Verification                | —                                           |

## Acceptance Criteria

- `cargo clippy -- -D warnings` → 0 warnings
- `cargo test --lib` → all pass (356+)
- `cargo test --test sprint24_streaming_test` → 14 tests pass
- `cargo test --test sprint112_websocket_ingestion_test` → 17 tests pass

## Dependencies

- `tokio-tungstenite` 0.21
- `fastrand` 2.3
- `futures` 0.3
- `serde_json` 1.0

## Sprint Context

- **Prior state:** Streaming module had stubbed `connect()` (sleeps 100ms) and
  naive `process_message()`. Sprint 24 tests could not compile.
- **After:** Real WS connections with Binance message parsing, exponential
  backoff reconnection, `StreamingConfig` / `StreamingEngine` API for tests.
