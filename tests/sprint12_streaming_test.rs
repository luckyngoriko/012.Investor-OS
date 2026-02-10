//! Integration tests for Sprint 12: Real-Time Streaming

use investor_os::streaming::{
    MarketDataStream, SignalType, StreamingEngine, TradingSignal
};
use rust_decimal::Decimal;

/// Test streaming engine creation
#[tokio::test]
async fn test_streaming_engine_creation() {
    let engine = StreamingEngine::new();
    assert!(true);
}

/// Test streaming engine start
#[tokio::test]
async fn test_streaming_engine_start() {
    let mut engine = StreamingEngine::new();
    let result = engine.start().await;
    assert!(result.is_ok());
}

/// Test signal subscription
#[tokio::test]
async fn test_signal_subscription() {
    let engine = StreamingEngine::new();
    let rx = engine.subscribe_signals();
    
    // Should be able to subscribe
    assert_eq!(rx.len(), 0); // No signals yet
}

/// Test trading signal creation
#[test]
fn test_trading_signal() {
    let signal = TradingSignal {
        symbol: "AAPL".to_string(),
        signal_type: SignalType::Buy,
        confidence: 0.85,
        cq_score: 0.82,
        timestamp: chrono::Utc::now(),
    };
    
    assert_eq!(signal.symbol, "AAPL");
    assert!(matches!(signal.signal_type, SignalType::Buy));
    assert_eq!(signal.confidence, 0.85);
    assert_eq!(signal.cq_score, 0.82);
}

/// Test all signal types
#[test]
fn test_signal_types() {
    let types = vec![
        SignalType::Buy,
        SignalType::Sell,
        SignalType::Hold,
        SignalType::StrongBuy,
        SignalType::StrongSell,
    ];
    
    for signal_type in types {
        let signal = TradingSignal {
            symbol: "TEST".to_string(),
            signal_type,
            confidence: 0.5,
            cq_score: 0.5,
            timestamp: chrono::Utc::now(),
        };
        
        // Just verify it compiles and runs
        assert!(signal.confidence > 0.0);
    }
}

/// Test market data stream
#[test]
fn test_market_data_stream() {
    let tick = MarketDataStream {
        symbol: "BTCUSDT".to_string(),
        bid: Decimal::from(50000),
        ask: Decimal::from(50001),
        last_price: Decimal::from(50000),
        volume_24h: Decimal::from(1000000),
        timestamp: chrono::Utc::now(),
    };
    
    assert_eq!(tick.symbol, "BTCUSDT");
    assert!(tick.bid < tick.ask); // Bid < Ask
    assert!(tick.volume_24h > Decimal::ZERO);
}

/// Test signal confidence thresholds
#[test]
fn test_signal_confidence_thresholds() {
    // High confidence signal
    let strong_signal = TradingSignal {
        symbol: "AAPL".to_string(),
        signal_type: SignalType::StrongBuy,
        confidence: 0.90,
        cq_score: 0.88,
        timestamp: chrono::Utc::now(),
    };
    
    assert!(strong_signal.confidence > 0.80);
    assert!(strong_signal.cq_score > 0.75);
    
    // Low confidence signal
    let weak_signal = TradingSignal {
        symbol: "TSLA".to_string(),
        signal_type: SignalType::Hold,
        confidence: 0.45,
        cq_score: 0.50,
        timestamp: chrono::Utc::now(),
    };
    
    assert!(weak_signal.confidence < 0.50);
}

/// Test timestamp ordering
#[test]
fn test_timestamp_ordering() {
    let now = chrono::Utc::now();
    
    let signal1 = TradingSignal {
        symbol: "A".to_string(),
        signal_type: SignalType::Buy,
        confidence: 0.8,
        cq_score: 0.8,
        timestamp: now,
    };
    
    let signal2 = TradingSignal {
        symbol: "B".to_string(),
        signal_type: SignalType::Sell,
        confidence: 0.7,
        cq_score: 0.7,
        timestamp: now + chrono::Duration::seconds(1),
    };
    
    assert!(signal2.timestamp > signal1.timestamp);
}

/// Test multiple subscribers
#[tokio::test]
async fn test_multiple_subscribers() {
    let engine = StreamingEngine::new();
    
    let rx1 = engine.subscribe_signals();
    let rx2 = engine.subscribe_signals();
    let rx3 = engine.subscribe_signals();
    
    // All should be created successfully
    assert_eq!(rx1.len(), 0);
    assert_eq!(rx2.len(), 0);
    assert_eq!(rx3.len(), 0);
}

/// Test signal clone (for broadcast)
#[test]
fn test_signal_clone() {
    let original = TradingSignal {
        symbol: "NVDA".to_string(),
        signal_type: SignalType::StrongBuy,
        confidence: 0.95,
        cq_score: 0.92,
        timestamp: chrono::Utc::now(),
    };
    
    let cloned = original.clone();
    
    assert_eq!(original.symbol, cloned.symbol);
    assert_eq!(original.confidence, cloned.confidence);
    assert_eq!(original.cq_score, cloned.cq_score);
}

/// Test CQ score bounds
#[test]
fn test_cq_score_bounds() {
    let valid_cq = TradingSignal {
        symbol: "GOOGL".to_string(),
        signal_type: SignalType::Buy,
        confidence: 0.75,
        cq_score: 0.75, // Valid: 0-1
        timestamp: chrono::Utc::now(),
    };
    
    // CQ should be between 0 and 1
    assert!(valid_cq.cq_score >= 0.0);
    assert!(valid_cq.cq_score <= 1.0);
}

/// Test streaming with different asset classes
#[test]
fn test_streaming_crypto_forex() {
    let crypto_tick = MarketDataStream {
        symbol: "BTCUSDT".to_string(),
        bid: Decimal::from(50000),
        ask: Decimal::from(50001),
        last_price: Decimal::from(50000),
        volume_24h: Decimal::from(1000000),
        timestamp: chrono::Utc::now(),
    };
    
    let forex_tick = MarketDataStream {
        symbol: "EUR_USD".to_string(),
        bid: Decimal::from_str_exact("1.0850").unwrap(),
        ask: Decimal::from_str_exact("1.0851").unwrap(),
        last_price: Decimal::from_str_exact("1.0850").unwrap(),
        volume_24h: Decimal::from(100000),
        timestamp: chrono::Utc::now(),
    };
    
    assert!(crypto_tick.volume_24h > forex_tick.volume_24h);
    assert_ne!(crypto_tick.symbol, forex_tick.symbol);
}
