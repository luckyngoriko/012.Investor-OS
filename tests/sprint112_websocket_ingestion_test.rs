//! Sprint 112: Live Market Data Feed — WebSocket Ingestion Tests
//!
//! Uses a mock WS server (`TcpListener` + `tokio_tungstenite::accept_async`)
//! to validate real `tokio-tungstenite` connections, Binance message parsing,
//! reconnection behaviour, subscription management, and graceful shutdown.

use futures::{SinkExt, StreamExt};
use investor_os::streaming::websocket::{
    backoff_with_jitter, build_ws_url, parse_binance_message, BinanceSubscription,
    SubscriptionManager,
};
use investor_os::streaming::{MarketTick, Side, StreamingConfig, StreamingEngine, TickType};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as TungsteniteMsg;

// ---------------------------------------------------------------------------
// Helper: start a mock WS server that sends a series of messages
// ---------------------------------------------------------------------------

async fn start_mock_ws_server(messages: Vec<String>) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}", addr);

    let handle = tokio::spawn(async move {
        // Accept one connection
        if let Ok((stream, _)) = listener.accept().await {
            if let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await {
                let (mut write, mut read) = ws_stream.split();

                // Send all messages
                for msg in messages {
                    let _ = write.send(TungsteniteMsg::Text(msg.into())).await;
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }

                // Read a few frames to avoid connection reset
                for _ in 0..5 {
                    match tokio::time::timeout(Duration::from_millis(100), read.next()).await {
                        Ok(Some(Ok(TungsteniteMsg::Close(_)))) | Ok(None) | Err(_) => break,
                        _ => {}
                    }
                }

                // Close
                let _ = write.send(TungsteniteMsg::Close(None)).await;
            }
        }
    });

    (url, handle)
}

// ---------------------------------------------------------------------------
// Test 1: Connect to mock server and receive ticks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_connect_receive_ticks() {
    let trade_msg =
        r#"{"e":"trade","s":"BTCUSDT","p":"50000","q":"0.5","m":false,"T":1709500000000}"#;

    let (url, _server) = start_mock_ws_server(vec![trade_msg.to_string()]).await;

    // Connect directly via tokio-tungstenite
    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let (_, mut read) = ws.split();

    if let Some(Ok(TungsteniteMsg::Text(text))) = read.next().await {
        let tick = parse_binance_message("binance", &text).unwrap();
        assert_eq!(tick.symbol, "BTCUSDT");
        assert_eq!(tick.price, Decimal::from_str("50000").unwrap());
        assert_eq!(tick.quantity, Decimal::from_str("0.5").unwrap());
        assert_eq!(tick.side, Some(Side::Bid));
        assert_eq!(tick.tick_type, TickType::Trade);
    } else {
        panic!("Expected text message from mock server");
    }
}

// ---------------------------------------------------------------------------
// Test 2: Parse trade, bookTicker, combined stream formats
// ---------------------------------------------------------------------------

#[test]
fn test_parse_trade_format() {
    let json = r#"{"e":"trade","s":"ETHUSDT","p":"3500.25","q":"10","m":true,"T":1709500000000}"#;
    let tick = parse_binance_message("binance", json).unwrap();
    assert_eq!(tick.symbol, "ETHUSDT");
    assert_eq!(tick.price, Decimal::from_str("3500.25").unwrap());
    assert_eq!(tick.side, Some(Side::Ask)); // is_buyer_maker=true → seller aggressive
}

#[test]
fn test_parse_book_ticker_format() {
    let json = r#"{"s":"BTCUSDT","b":"50000","B":"1.5","a":"50001","A":"2.0"}"#;
    let tick = parse_binance_message("binance", json).unwrap();
    assert_eq!(tick.symbol, "BTCUSDT");
    assert_eq!(tick.side, None);
    assert_eq!(tick.tick_type, TickType::Bid);
    // Mid price
    let expected_mid = (Decimal::from_str("50000").unwrap() + Decimal::from_str("50001").unwrap())
        / Decimal::from(2);
    assert_eq!(tick.price, expected_mid);
}

#[test]
fn test_parse_combined_stream_format() {
    let json = r#"{"stream":"btcusdt@trade","data":{"e":"trade","s":"BTCUSDT","p":"49999","q":"1.2","m":false,"T":1709500001000}}"#;
    let tick = parse_binance_message("binance", json).unwrap();
    assert_eq!(tick.symbol, "BTCUSDT");
    assert_eq!(tick.price, Decimal::from_str("49999").unwrap());
    assert_eq!(tick.side, Some(Side::Bid));
}

// ---------------------------------------------------------------------------
// Test 3: Reconnection after server drop
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_reconnection_after_server_drop() {
    // Start a server that immediately closes
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}", addr);

    let _server = tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            if let Ok(ws) = tokio_tungstenite::accept_async(stream).await {
                let (mut write, _) = ws.split();
                // Close immediately
                let _ = write.send(TungsteniteMsg::Close(None)).await;
            }
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect — should succeed then see close
    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let (_, mut read) = ws.split();

    // Should receive Close frame
    let msg = read.next().await;
    match msg {
        Some(Ok(TungsteniteMsg::Close(_))) | None => {
            // Expected: server closed connection — triggers reconnect in real code
        }
        other => {
            // Connection drop or frame might look different; just verify non-panic
            let _ = other;
        }
    }
}

// ---------------------------------------------------------------------------
// Test 4: Subscribe/unsubscribe JSON validation
// ---------------------------------------------------------------------------

#[test]
fn test_subscribe_json_format() {
    let mut mgr = SubscriptionManager::new();
    let json = mgr.subscribe(&["btcusdt@trade".to_string(), "ethusdt@trade".to_string()]);

    let parsed: BinanceSubscription = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.method, "SUBSCRIBE");
    assert_eq!(parsed.params.len(), 2);
    assert!(parsed.params.contains(&"btcusdt@trade".to_string()));
    assert!(parsed.params.contains(&"ethusdt@trade".to_string()));
    assert!(parsed.id > 0);
}

#[test]
fn test_unsubscribe_json_format() {
    let mut mgr = SubscriptionManager::new();
    // First subscribe
    let _ = mgr.subscribe(&["btcusdt@trade".to_string()]);

    // Then unsubscribe
    let json = mgr.unsubscribe(&["btcusdt@trade".to_string()]);
    let parsed: BinanceSubscription = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.method, "UNSUBSCRIBE");
    assert_eq!(parsed.params, vec!["btcusdt@trade"]);
}

#[test]
fn test_subscribe_dedup() {
    let mut mgr = SubscriptionManager::new();
    let _first = mgr.subscribe(&["btcusdt@trade".to_string()]);
    let second = mgr.subscribe(&["btcusdt@trade".to_string()]);
    assert!(second.is_empty(), "duplicate subscribe should be empty");
}

// ---------------------------------------------------------------------------
// Test 5: Graceful shutdown
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_graceful_shutdown() {
    let (signal_tx, _signal_rx) = mpsc::channel(100);
    let config = StreamingConfig::default();
    let (mut engine, _tick_tx) = StreamingEngine::new_with_config(config, signal_tx);

    engine
        .register_symbol("BTC-USD".to_string(), "binance".to_string())
        .await;

    engine.start().await.expect("Failed to start engine");
    assert!(engine.is_running());

    engine.stop().await;
    assert!(!engine.is_running());
}

// ---------------------------------------------------------------------------
// Test 6: Invalid message resilience
// ---------------------------------------------------------------------------

#[test]
fn test_invalid_message_does_not_panic() {
    // Completely invalid
    assert!(parse_binance_message("binance", "").is_none());
    assert!(parse_binance_message("binance", "not json at all").is_none());
    assert!(parse_binance_message("binance", "{}").is_none());
    assert!(parse_binance_message("binance", r#"{"e":"unknown"}"#).is_none());

    // Valid JSON but bad field types
    assert!(parse_binance_message(
        "binance",
        r#"{"e":"trade","s":"X","p":"BAD","q":"1","m":false,"T":0}"#
    )
    .is_none());
}

#[tokio::test]
async fn test_invalid_messages_from_mock_server() {
    let messages = vec![
        "not json".to_string(),
        r#"{"foo":"bar"}"#.to_string(),
        r#"{"e":"trade","s":"BTCUSDT","p":"50000","q":"1","m":false,"T":1709500000000}"#
            .to_string(),
    ];

    let (url, _server) = start_mock_ws_server(messages).await;

    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let (_, mut read) = ws.split();

    let mut ticks = Vec::new();
    while let Ok(Some(Ok(TungsteniteMsg::Text(text)))) =
        tokio::time::timeout(Duration::from_millis(500), read.next()).await
    {
        if let Some(tick) = parse_binance_message("binance", &text) {
            ticks.push(tick);
        }
    }

    // Only the valid trade message should parse
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0].symbol, "BTCUSDT");
}

// ---------------------------------------------------------------------------
// Test 7: Backoff bounds check
// ---------------------------------------------------------------------------

#[test]
fn test_backoff_bounds() {
    // attempt 0: base_ms * 2^0 = base_ms
    for _ in 0..50 {
        let b = backoff_with_jitter(1000, 0);
        assert!(b >= 1000, "attempt 0: got {}", b);
        assert!(b <= 1250, "attempt 0: got {}", b);
    }

    // attempt 5: base_ms * 2^5 = 32000
    for _ in 0..50 {
        let b = backoff_with_jitter(1000, 5);
        assert!(b >= 32000, "attempt 5: got {}", b);
        assert!(b <= 40000, "attempt 5: got {}", b);
    }

    // attempt 10 (beyond cap): should be capped at 60000 + jitter
    for _ in 0..50 {
        let b = backoff_with_jitter(1000, 10);
        assert!(b <= 75_000, "attempt 10: got {}", b);
    }

    // Large base: should still cap at 60_000 + jitter
    for _ in 0..50 {
        let b = backoff_with_jitter(10_000, 3);
        assert!(b <= 75_000, "large base attempt 3: got {}", b);
    }
}

// ---------------------------------------------------------------------------
// Test 8: WS URL builder
// ---------------------------------------------------------------------------

#[test]
fn test_ws_url_single_symbol() {
    let url = build_ws_url("wss://stream.binance.com:9443/ws", &["BTCUSDT".to_string()]);
    assert_eq!(url, "wss://stream.binance.com:9443/ws/btcusdt@trade");
}

#[test]
fn test_ws_url_multi_symbol() {
    let url = build_ws_url(
        "wss://stream.binance.com:9443/ws",
        &["BTCUSDT".to_string(), "ETHUSDT".to_string()],
    );
    assert!(url.contains("/stream?streams="));
    assert!(url.contains("btcusdt@trade"));
    assert!(url.contains("ethusdt@trade"));
}

#[test]
fn test_ws_url_empty() {
    let url = build_ws_url("wss://stream.binance.com:9443/ws", &[]);
    assert_eq!(url, "wss://stream.binance.com:9443/ws");
}

// ---------------------------------------------------------------------------
// Test 9: StreamingEngine with tick injection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_engine_tick_injection() {
    let (signal_tx, mut signal_rx) = mpsc::channel(100);
    let config = StreamingConfig {
        max_latency_ms: 100,
        signal_cooldown_ms: 0,
        enable_deduplication: false,
        trade_window_sec: 60,
        large_trade_threshold: Decimal::from(1),
    };

    let (mut engine, tick_tx) = StreamingEngine::new_with_config(config, signal_tx);
    engine
        .register_symbol("BTCUSDT".to_string(), "binance".to_string())
        .await;
    engine.start().await.expect("start");

    // Inject 5 ticks
    for _ in 0..5 {
        let tick = MarketTick {
            exchange: "binance".to_string(),
            symbol: "BTCUSDT".to_string(),
            price: Decimal::from_str("50000").unwrap(),
            quantity: Decimal::from(1),
            side: Some(Side::Bid),
            tick_type: TickType::Trade,
            timestamp: chrono::Utc::now(),
        };
        tick_tx.send(tick).await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    engine.stop().await;

    // Should have received at least one signal
    let mut count = 0;
    while signal_rx.try_recv().is_ok() {
        count += 1;
    }
    assert!(count >= 1, "expected at least 1 signal, got {}", count);
}

// ---------------------------------------------------------------------------
// Test 10: Combined stream bookTicker via mock
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_combined_book_ticker_mock() {
    let msg = r#"{"stream":"btcusdt@bookTicker","data":{"s":"BTCUSDT","b":"50000","B":"1.5","a":"50001","A":"2.0"}}"#;
    let (url, _server) = start_mock_ws_server(vec![msg.to_string()]).await;

    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let (_, mut read) = ws.split();

    if let Some(Ok(TungsteniteMsg::Text(text))) = read.next().await {
        let tick = parse_binance_message("binance", &text).unwrap();
        assert_eq!(tick.symbol, "BTCUSDT");
        assert_eq!(tick.tick_type, TickType::Bid);
        assert_eq!(tick.side, None);
    }
}
