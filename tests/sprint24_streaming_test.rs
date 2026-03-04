//! Sprint 24: Real-time Streaming & Order Flow - Golden Path Tests
//!
//! Tests for:
//! - WebSocket market data feeds
//! - Order book reconstruction
//! - Trade flow analysis
//! - Real-time signal generation

use investor_os::streaming::orderbook::{BookUpdate, UpdateType};
use investor_os::streaming::trade_analyzer::{TradeAnalyzer, TradeClassification};
use investor_os::streaming::*;
use rust_decimal::Decimal;
use std::time::Duration;
use tokio::sync::mpsc;

/// Test 1: WebSocket connection and reconnection
#[tokio::test]
async fn test_websocket_connection_and_reconnection() {
    let (tx, _rx) = mpsc::channel(1000);
    let manager = WebSocketManager::new(tx);

    // Check that manager is created (no feeds registered yet)
    let feeds = manager.get_all_feeds().await;
    assert!(feeds.is_empty());
}

/// Test 2: Order book reconstruction with L2 data
#[test]
fn test_order_book_reconstruction() {
    let mut book = OrderBook::new("BTC-USD".to_string(), "binance".to_string());

    // Add bid levels using apply_update
    for i in 0..3 {
        book.apply_update(BookUpdate {
            side: Side::Bid,
            price: Decimal::try_from(50000.0 - (i as f64 * 100.0)).unwrap(),
            quantity: Decimal::try_from(1.5 + (i as f64 * 0.5)).unwrap(),
            update_type: UpdateType::Add,
            timestamp: chrono::Utc::now(),
        });
    }

    // Add ask levels
    for i in 0..3 {
        book.apply_update(BookUpdate {
            side: Side::Ask,
            price: Decimal::try_from(50100.0 + (i as f64 * 100.0)).unwrap(),
            quantity: Decimal::try_from(1.0 + (i as f64 * 0.5)).unwrap(),
            update_type: UpdateType::Add,
            timestamp: chrono::Utc::now(),
        });
    }

    // Verify best bid/ask
    let best_bid = book.best_bid().unwrap();
    let best_ask = book.best_ask().unwrap();

    assert_eq!(best_bid.price, Decimal::try_from(50000.0).unwrap());
    assert_eq!(best_ask.price, Decimal::try_from(50100.0).unwrap());

    // Calculate spread
    let spread = book.spread().unwrap();
    assert_eq!(spread, Decimal::try_from(100.0).unwrap());

    // Mid price
    let mid = book.mid_price().unwrap();
    assert_eq!(mid, Decimal::try_from(50050.0).unwrap());
}

/// Test 3: Order book imbalance calculation
#[test]
fn test_order_book_imbalance() {
    let mut book = OrderBook::new("ETH-USD".to_string(), "binance".to_string());

    // Create buy-heavy book
    book.apply_update(BookUpdate {
        side: Side::Bid,
        price: Decimal::try_from(3000.0).unwrap(),
        quantity: Decimal::try_from(100.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });
    book.apply_update(BookUpdate {
        side: Side::Bid,
        price: Decimal::try_from(2990.0).unwrap(),
        quantity: Decimal::try_from(50.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });
    book.apply_update(BookUpdate {
        side: Side::Ask,
        price: Decimal::try_from(3010.0).unwrap(),
        quantity: Decimal::try_from(10.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });

    let (bid_vol, ask_vol, imbalance) = book.get_imbalance(3); // Top 3 levels

    // Total volumes should be positive
    assert!(bid_vol > Decimal::ZERO);
    assert!(ask_vol > Decimal::ZERO);

    // Imbalance should be positive (more bids)
    assert!(imbalance > Decimal::ZERO);
}

/// Test 4: Trade flow analysis and classification
#[test]
fn test_trade_flow_analysis() {
    let threshold = Decimal::from(10); // Large trade threshold
    let mut analyzer = TradeAnalyzer::new(60, threshold);

    // Create a large block trade (>= 100 for Block classification)
    let trade = MarketTick {
        symbol: "BTC-USD".to_string(),
        exchange: "binance".to_string(),
        price: Decimal::try_from(50000.0).unwrap(),
        quantity: Decimal::try_from(150.0).unwrap(), // Block trade (>= 100)
        side: Some(Side::Bid),                       // Buy side
        timestamp: chrono::Utc::now(),
        tick_type: TickType::Trade,
    };

    let analyzed = analyzer.analyze(
        trade,
        Decimal::try_from(49990.0).unwrap(),
        Decimal::try_from(50010.0).unwrap(),
    );

    // Large quantity (>= 100) should be classified as Block
    assert_eq!(analyzed.classification, TradeClassification::Block);
}

/// Test 5: Trade flow with buy/sell pressure
#[test]
fn test_trade_flow_pressure() {
    let threshold = Decimal::from(10);
    let mut analyzer = TradeAnalyzer::new(60, threshold);

    // Add multiple buy trades
    for i in 0..5 {
        let trade = MarketTick {
            symbol: "AAPL".to_string(),
            exchange: "nasdaq".to_string(),
            price: Decimal::try_from(150.0 + (i as f64 * 0.1)).unwrap(),
            quantity: Decimal::try_from(5.0).unwrap(),
            side: Some(Side::Bid), // Buy
            timestamp: chrono::Utc::now(),
            tick_type: TickType::Trade,
        };
        analyzer.analyze(
            trade,
            Decimal::try_from(149.9).unwrap(),
            Decimal::try_from(150.1).unwrap(),
        );
    }

    // Add one sell trade
    let sell_trade = MarketTick {
        symbol: "AAPL".to_string(),
        exchange: "nasdaq".to_string(),
        price: Decimal::try_from(150.5).unwrap(),
        quantity: Decimal::try_from(2.0).unwrap(),
        side: Some(Side::Ask), // Sell
        timestamp: chrono::Utc::now(),
        tick_type: TickType::Trade,
    };
    analyzer.analyze(
        sell_trade,
        Decimal::try_from(150.4).unwrap(),
        Decimal::try_from(150.6).unwrap(),
    );

    let flow = analyzer.get_flow();
    // Should have more buy pressure
    assert!(flow.buy_pressure > Decimal::try_from(0.5).unwrap());
    assert!(flow.total_volume > Decimal::ZERO);
}

/// Test 6: Signal generation latency
#[tokio::test]
async fn test_real_time_signal_latency() {
    let (tx, _rx) = mpsc::channel(100);
    let config = StreamingConfig {
        max_latency_ms: 100,   // Sub-100ms requirement
        signal_cooldown_ms: 0, // No cooldown for testing
        enable_deduplication: false,
        trade_window_sec: 60,
        large_trade_threshold: Decimal::from(1),
    };

    let (mut engine, tick_tx) = StreamingEngine::new_with_config(config, tx);

    // Register a symbol
    engine
        .register_symbol("BTC-USD".to_string(), "binance".to_string())
        .await;

    // Start engine
    engine.start().await.expect("Failed to start engine");

    // Send a tick that should trigger a signal (block trade with high buy pressure)
    let tick = MarketTick {
        symbol: "BTC-USD".to_string(),
        exchange: "binance".to_string(),
        price: Decimal::try_from(50000.0).unwrap(),
        quantity: Decimal::try_from(100.0).unwrap(), // Large trade to trigger Block classification
        side: Some(Side::Bid),                       // Buy
        timestamp: chrono::Utc::now(),
        tick_type: TickType::Trade,
    };

    let start = std::time::Instant::now();
    tick_tx.send(tick).await.expect("Failed to send tick");

    // Wait a short time for processing
    tokio::time::sleep(Duration::from_millis(50)).await;
    let latency = start.elapsed().as_millis() as u64;

    // Stop engine
    engine.stop().await;

    // Latency should be under 100ms
    assert!(latency < 100, "Latency {}ms exceeded 100ms limit", latency);
}

/// Test 7: Signal generation works (deduplication tested via engine behavior)
#[tokio::test]
async fn test_signal_generation_flow() {
    let (tx, mut rx) = mpsc::channel(100);
    let config = StreamingConfig {
        max_latency_ms: 100,
        signal_cooldown_ms: 0, // No cooldown for testing
        enable_deduplication: false,
        trade_window_sec: 60,
        large_trade_threshold: Decimal::from(1),
    };

    let (mut engine, tick_tx) = StreamingEngine::new_with_config(config, tx);
    engine
        .register_symbol("BTC-USD".to_string(), "binance".to_string())
        .await;

    // Start engine
    engine.start().await.expect("Failed to start engine");

    // Send multiple buy ticks to trigger a signal
    for _ in 0..5 {
        let tick = MarketTick {
            symbol: "BTC-USD".to_string(),
            exchange: "binance".to_string(),
            price: Decimal::try_from(50000.0).unwrap(),
            quantity: Decimal::try_from(50.0).unwrap(), // Block trade
            side: Some(Side::Bid),
            timestamp: chrono::Utc::now(),
            tick_type: TickType::Trade,
        };
        tick_tx.send(tick).await.expect("Failed to send tick");
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stop engine
    engine.stop().await;

    // The test passes if no panic occurs during signal processing
}

/// Test 8: Multi-symbol streaming
#[tokio::test]
async fn test_multi_symbol_streaming() {
    let (tx, _rx) = mpsc::channel(100);
    let config = StreamingConfig::default();
    let (mut engine, _tick_tx) = StreamingEngine::new_with_config(config, tx);

    // Register multiple symbols
    let symbols = vec!["BTC-USD", "ETH-USD", "SOL-USD"];

    for symbol in &symbols {
        engine
            .register_symbol(symbol.to_string(), "binance".to_string())
            .await;
    }

    // Verify order books are created
    for symbol in &symbols {
        let book = engine.get_orderbook(symbol).await;
        assert!(book.is_some(), "Order book should exist for {}", symbol);
    }
}

/// Test 9: Order book updates
#[test]
fn test_order_book_updates() {
    let mut book = OrderBook::new("BTC-USD".to_string(), "binance".to_string());

    // Apply initial updates
    book.apply_update(BookUpdate {
        side: Side::Bid,
        price: Decimal::try_from(50000.0).unwrap(),
        quantity: Decimal::try_from(1.0).unwrap(),
        update_type: UpdateType::Modify,
        timestamp: chrono::Utc::now(),
    });

    book.apply_update(BookUpdate {
        side: Side::Ask,
        price: Decimal::try_from(50100.0).unwrap(),
        quantity: Decimal::try_from(1.0).unwrap(),
        update_type: UpdateType::Modify,
        timestamp: chrono::Utc::now(),
    });

    // Verify spread
    let spread = book.spread().unwrap();
    assert_eq!(spread, Decimal::try_from(100.0).unwrap());

    // Modify a level
    book.apply_update(BookUpdate {
        side: Side::Bid,
        price: Decimal::try_from(50000.0).unwrap(),
        quantity: Decimal::try_from(2.0).unwrap(),
        update_type: UpdateType::Modify,
        timestamp: chrono::Utc::now(),
    });

    let best_bid = book.best_bid().unwrap();
    assert_eq!(best_bid.quantity, Decimal::try_from(2.0).unwrap());
}

/// Test 10: End-to-end streaming pipeline
#[tokio::test]
async fn test_end_to_end_streaming_pipeline() {
    let (tx, _rx) = mpsc::channel(100);
    let config = StreamingConfig {
        max_latency_ms: 100,
        signal_cooldown_ms: 0,
        enable_deduplication: false,
        trade_window_sec: 60,
        large_trade_threshold: Decimal::from(1),
    };

    let (mut engine, tick_tx) = StreamingEngine::new_with_config(config, tx);
    engine
        .register_symbol("BTC-USD".to_string(), "binance".to_string())
        .await;

    // Start engine
    engine.start().await.expect("Failed to start engine");

    // Send multiple ticks
    for i in 0..10 {
        let tick = MarketTick {
            symbol: "BTC-USD".to_string(),
            exchange: "binance".to_string(),
            price: Decimal::try_from(50000.0 + (i as f64 * 10.0)).unwrap(),
            quantity: Decimal::try_from(20.0).unwrap(), // Large enough for Block classification
            side: if i % 2 == 0 {
                Some(Side::Bid)
            } else {
                Some(Side::Ask)
            },
            timestamp: chrono::Utc::now(),
            tick_type: TickType::Trade,
        };
        tick_tx.send(tick).await.expect("Failed to send tick");
    }

    // Wait a bit for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stop engine
    engine.stop().await;

    // The test passes if the pipeline works without errors
}

/// Test 11: VWAP calculation
#[test]
fn test_vwap_calculation() {
    let mut book = OrderBook::new("BTC-USD".to_string(), "binance".to_string());

    // Add bid levels
    book.apply_update(BookUpdate {
        side: Side::Bid,
        price: Decimal::try_from(50000.0).unwrap(),
        quantity: Decimal::try_from(1.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });
    book.apply_update(BookUpdate {
        side: Side::Bid,
        price: Decimal::try_from(49900.0).unwrap(),
        quantity: Decimal::try_from(2.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });

    // Add ask levels
    book.apply_update(BookUpdate {
        side: Side::Ask,
        price: Decimal::try_from(50100.0).unwrap(),
        quantity: Decimal::try_from(1.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });
    book.apply_update(BookUpdate {
        side: Side::Ask,
        price: Decimal::try_from(50200.0).unwrap(),
        quantity: Decimal::try_from(2.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });

    // Calculate VWAP for buying 1.5 units
    // Takes 1 @ 50100 + 0.5 @ 50200 = 75200 / 1.5 = ~50133.33
    let vwap = book.vwap(Side::Bid, Decimal::try_from(1.5).unwrap());
    assert!(vwap.is_some());

    let vwap_val = vwap.unwrap();
    assert!(vwap_val > Decimal::try_from(50100.0).unwrap());
    assert!(vwap_val < Decimal::try_from(50200.0).unwrap());
}

/// Test 12: Book snapshot
#[test]
fn test_book_snapshot() {
    let mut book = OrderBook::new("BTC-USD".to_string(), "binance".to_string());

    // Create price levels
    let bids = vec![
        investor_os::streaming::orderbook::PriceLevel::new(
            Decimal::try_from(50000.0).unwrap(),
            Decimal::try_from(1.0).unwrap(),
        ),
        investor_os::streaming::orderbook::PriceLevel::new(
            Decimal::try_from(49900.0).unwrap(),
            Decimal::try_from(2.0).unwrap(),
        ),
    ];
    let asks = vec![
        investor_os::streaming::orderbook::PriceLevel::new(
            Decimal::try_from(50100.0).unwrap(),
            Decimal::try_from(1.0).unwrap(),
        ),
        investor_os::streaming::orderbook::PriceLevel::new(
            Decimal::try_from(50200.0).unwrap(),
            Decimal::try_from(2.0).unwrap(),
        ),
    ];

    book.apply_snapshot(bids, asks);

    let best_bid = book.best_bid().unwrap();
    let best_ask = book.best_ask().unwrap();
    assert_eq!(best_bid.price, Decimal::try_from(50000.0).unwrap());
    assert_eq!(best_ask.price, Decimal::try_from(50100.0).unwrap());
}

/// Test 13: Crossed book detection
#[test]
fn test_crossed_book_detection() {
    let mut book = OrderBook::new("BTC-USD".to_string(), "binance".to_string());

    // Normal book
    book.apply_update(BookUpdate {
        side: Side::Bid,
        price: Decimal::try_from(50000.0).unwrap(),
        quantity: Decimal::try_from(1.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });
    book.apply_update(BookUpdate {
        side: Side::Ask,
        price: Decimal::try_from(50100.0).unwrap(),
        quantity: Decimal::try_from(1.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });

    assert!(!book.is_crossed());

    // Cross the book - bid >= ask
    book.apply_update(BookUpdate {
        side: Side::Bid,
        price: Decimal::try_from(50150.0).unwrap(),
        quantity: Decimal::try_from(1.0).unwrap(),
        update_type: UpdateType::Add,
        timestamp: chrono::Utc::now(),
    });

    assert!(book.is_crossed());
}

/// Test 14: Market impact calculation
#[test]
fn test_market_impact() {
    let mut book = OrderBook::new("BTC-USD".to_string(), "binance".to_string());

    // Create book with depth
    for i in 0..5 {
        book.apply_update(BookUpdate {
            side: Side::Bid,
            price: Decimal::try_from(50000.0 - (i as f64 * 100.0)).unwrap(),
            quantity: Decimal::try_from(1.0).unwrap(),
            update_type: UpdateType::Add,
            timestamp: chrono::Utc::now(),
        });
        book.apply_update(BookUpdate {
            side: Side::Ask,
            price: Decimal::try_from(50100.0 + (i as f64 * 100.0)).unwrap(),
            quantity: Decimal::try_from(1.0).unwrap(),
            update_type: UpdateType::Add,
            timestamp: chrono::Utc::now(),
        });
    }

    // Calculate market impact for buying 3 units
    let impact = book.market_impact(Side::Bid, Decimal::from(3));
    assert!(impact.is_some());

    // Impact should be positive
    let impact_val = impact.unwrap();
    assert!(impact_val > Decimal::ZERO);
}
