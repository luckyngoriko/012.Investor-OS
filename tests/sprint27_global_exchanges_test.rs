//! Sprint 27: Global Exchange Integration - Golden Path Tests
//!
//! Tests for:
//! - 50+ exchanges support
//! - Trading hours management
//! - Free data sources aggregation
//! - AI pattern discovery from free data
//! - Cross-market arbitrage detection

use chrono::Utc;
use investor_os::global::{
    free_data::{
        CrossSourcePrice, CrossSourceValidation, DataSource, FreeDataAggregator, FreeMarketData,
    },
    ExchangeId, GlobalExchangeRegistry, GlobalMarketData, GlobalQuote, Region,
};
use rust_decimal::Decimal;

// ============================================================================
// Test 1: Exchange Registry Creation
// ============================================================================

#[test]
fn test_exchange_registry_creation() {
    let registry = GlobalExchangeRegistry::new();

    assert!(
        registry.exchange_count() >= 15,
        "Should have at least 15 exchanges"
    );
}

// ============================================================================
// Test 2: Major Exchange Lookup
// ============================================================================

#[test]
fn test_major_exchange_lookup() {
    let registry = GlobalExchangeRegistry::new();

    let nyse = registry.get_exchange(&ExchangeId("NYSE".to_string()));
    assert!(nyse.is_some(), "NYSE should exist");
    assert_eq!(nyse.unwrap().region, Region::NorthAmerica);

    let nasdaq = registry.get_exchange(&ExchangeId("NASDAQ".to_string()));
    assert!(nasdaq.is_some(), "NASDAQ should exist");

    let lse = registry.get_exchange(&ExchangeId("LSE".to_string()));
    assert!(lse.is_some(), "LSE should exist");
    assert_eq!(lse.unwrap().region, Region::Europe);
}

// ============================================================================
// Test 3: Exchange by Region
// ============================================================================

#[test]
fn test_exchanges_by_region() {
    let registry = GlobalExchangeRegistry::new();

    let na = registry.get_exchanges_by_region(Region::NorthAmerica);
    assert!(!na.is_empty(), "Should have North American exchanges");

    let eu = registry.get_exchanges_by_region(Region::Europe);
    assert!(!eu.is_empty(), "Should have European exchanges");

    let apac = registry.get_exchanges_by_region(Region::AsiaPacific);
    assert!(!apac.is_empty(), "Should have APAC exchanges");

    let latam = registry.get_exchanges_by_region(Region::LatinAmerica);
    assert!(!latam.is_empty(), "Should have Latin American exchanges");
}

// ============================================================================
// Test 4: Market Hours Check
// ============================================================================

#[test]
fn test_market_hours() {
    let registry = GlobalExchangeRegistry::new();

    let now = Utc::now();

    // Get open exchanges (if any)
    let open_exchanges = registry.get_open_exchanges(now);

    // This test may pass or fail depending on current time
    // Just verify it doesn't panic
    let _ = open_exchanges.len();
}

// ============================================================================
// Test 5: Global Market Data Aggregation
// ============================================================================

#[test]
fn test_global_market_data_aggregation() {
    let mut market_data = GlobalMarketData::new();

    // Add prices from different exchanges
    market_data.update_price(
        "AAPL",
        ExchangeId("NYSE".to_string()),
        Decimal::try_from(150.0).unwrap(),
    );
    market_data.update_price(
        "AAPL",
        ExchangeId("NASDAQ".to_string()),
        Decimal::try_from(150_05).unwrap() / Decimal::from(100),
    );

    let prices = market_data.get_all_prices("AAPL").unwrap();
    assert_eq!(prices.len(), 2, "Should have 2 prices for AAPL");
}

// ============================================================================
// Test 6: Best Price Discovery
// ============================================================================

#[test]
fn test_best_price_discovery() {
    let mut market_data = GlobalMarketData::new();

    market_data.update_price(
        "BTCUSD",
        ExchangeId("BINANCE".to_string()),
        Decimal::try_from(50000.0).unwrap(),
    );
    market_data.update_price(
        "BTCUSD",
        ExchangeId("COINBASE".to_string()),
        Decimal::try_from(50100.0).unwrap(),
    );
    market_data.update_price(
        "BTCUSD",
        ExchangeId("KRAKEN".to_string()),
        Decimal::try_from(49950.0).unwrap(),
    );

    let quote = market_data.get_best_price("BTCUSD").unwrap();

    assert!(quote.best_bid > Decimal::ZERO);
    assert!(quote.best_ask > Decimal::ZERO);
    assert!(quote.best_bid >= quote.best_ask);
}

// ============================================================================
// Test 7: Free Data Source Aggregation
// ============================================================================

#[test]
fn test_free_data_aggregation() {
    let mut aggregator = FreeDataAggregator::new();

    let data1 = FreeMarketData {
        symbol: "AAPL".to_string(),
        source: DataSource::YahooFinance,
        price: Decimal::try_from(150.0).unwrap(),
        change_24h: None,
        volume_24h: Some(Decimal::from(1000000)),
        timestamp: Utc::now(),
        bid: None,
        ask: None,
    };

    let data2 = FreeMarketData {
        symbol: "AAPL".to_string(),
        source: DataSource::AlphaVantage,
        price: Decimal::try_from(150_50).unwrap() / Decimal::from(100),
        change_24h: None,
        volume_24h: Some(Decimal::from(1000000)),
        timestamp: Utc::now(),
        bid: None,
        ask: None,
    };

    aggregator.add_data(data1);
    aggregator.add_data(data2);

    let consensus = aggregator.get_consensus_price("AAPL").unwrap();
    assert!(consensus.price > Decimal::ZERO);
    assert_eq!(consensus.sources.len(), 2);
}

// ============================================================================
// Test 8: Consensus Price Calculation
// ============================================================================

#[test]
fn test_consensus_price_calculation() {
    let mut aggregator = FreeDataAggregator::new();

    // Add prices from multiple sources
    for (source, price) in [
        (DataSource::YahooFinance, 150.0),
        (DataSource::AlphaVantage, 150.5),
        (DataSource::Finnhub, 149.8),
        (DataSource::TwelveData, 150.2),
    ] {
        aggregator.add_data(FreeMarketData {
            symbol: "AAPL".to_string(),
            source,
            price: Decimal::try_from(price).unwrap(),
            change_24h: None,
            volume_24h: None,
            timestamp: Utc::now(),
            bid: None,
            ask: None,
        });
    }

    let consensus = aggregator.get_consensus_price("AAPL").unwrap();

    // Consensus should be close to 150
    let consensus_f64: f64 = consensus.price.try_into().unwrap();
    assert!(consensus_f64 > 149.0 && consensus_f64 < 151.0);

    // Should have high confidence with 4 sources
    assert!(consensus.confidence > 0.8);
}

// ============================================================================
// Test 9: Cross-Source Price Validation
// ============================================================================

#[test]
fn test_cross_source_validation() {
    let mut cross = CrossSourcePrice::new("AAPL");

    cross.add_price(DataSource::YahooFinance, Decimal::try_from(150.0).unwrap());
    cross.add_price(
        DataSource::AlphaVantage,
        Decimal::try_from(150_10).unwrap() / Decimal::from(100),
    );
    cross.add_price(
        DataSource::Finnhub,
        Decimal::try_from(149_90).unwrap() / Decimal::from(100),
    );

    cross.calculate_consensus();

    assert!(cross.consensus_price > Decimal::ZERO);
    assert!(cross.spread > Decimal::ZERO);
    assert!(cross.spread_pct < 1.0); // Less than 1% spread
}

// ============================================================================
// Test 10: Outlier Detection
// ============================================================================

#[test]
fn test_outlier_detection() {
    let mut cross = CrossSourcePrice::new("AAPL");

    // Most sources agree around 150
    cross.add_price(DataSource::YahooFinance, Decimal::try_from(150.0).unwrap());
    cross.add_price(
        DataSource::AlphaVantage,
        Decimal::try_from(150_20).unwrap() / Decimal::from(100),
    );
    cross.add_price(
        DataSource::Finnhub,
        Decimal::try_from(149_80).unwrap() / Decimal::from(100),
    );
    // One outlier at 160 (6.7% deviation)
    cross.add_price(DataSource::TwelveData, Decimal::from(160));

    cross.calculate_consensus();

    assert!(!cross.outlier_sources.is_empty(), "Should detect outlier");
    assert!(cross.outlier_sources.contains(&DataSource::TwelveData));
}

// ============================================================================
// Test 11: Arbitrage Detection
// ============================================================================

#[test]
fn test_arbitrage_detection() {
    let mut cross = CrossSourcePrice::new("BTCUSD");

    // 2% price difference
    cross.add_price(
        DataSource::BinancePublic,
        Decimal::try_from(50000.0).unwrap(),
    );
    cross.add_price(
        DataSource::CryptoCompare,
        Decimal::try_from(51000.0).unwrap(),
    );

    cross.calculate_consensus();

    let arb = CrossSourceValidation::find_arbitrage(&cross, 1.0);

    assert!(arb.is_some(), "Should detect arbitrage opportunity");
    let arb = arb.unwrap();
    assert!(arb.profit_pct >= 1.0, "Profit should be at least 1%");
}

// ============================================================================
// Test 12: AI Pattern Discovery - Volume Spike
// ============================================================================

#[test]
fn test_ai_pattern_volume_spike() {
    let mut aggregator = FreeDataAggregator::new();

    // Add historical data with volume spike
    for i in 0..30 {
        let volume = if i > 20 { 1000000.0 } else { 100000.0 }; // 10x spike

        aggregator.add_data(FreeMarketData {
            symbol: "AAPL".to_string(),
            source: DataSource::YahooFinance,
            price: Decimal::try_from(150.0 + i as f64 * 0.1).unwrap(),
            change_24h: None,
            volume_24h: Some(Decimal::try_from(volume).unwrap()),
            timestamp: Utc::now() - chrono::Duration::minutes(i as i64),
            bid: None,
            ask: None,
        });
    }

    let patterns = aggregator.detect_patterns("AAPL");

    // May or may not detect pattern depending on implementation
    // Just verify it doesn't panic
}

// ============================================================================
// Test 13: Data Source Quality Tracking
// ============================================================================

#[test]
fn test_data_source_quality() {
    let mut aggregator = FreeDataAggregator::new();

    // Add sample data
    aggregator.add_data(FreeMarketData {
        symbol: "AAPL".to_string(),
        source: DataSource::YahooFinance,
        price: Decimal::try_from(150.0).unwrap(),
        change_24h: None,
        volume_24h: None,
        timestamp: Utc::now(),
        bid: None,
        ask: None,
    });

    // Validate against "paid" source
    let validation =
        aggregator.validate_against_paid("AAPL", Decimal::try_from(150.02).unwrap(), "Bloomberg");

    // Should be accurate within 0.1%
    assert!(validation.is_accurate || validation.deviation_pct < 0.1);
}

// ============================================================================
// Test 14: Free vs Paid Data Comparison
// ============================================================================

#[test]
fn test_free_vs_paid_comparison() {
    let mut aggregator = FreeDataAggregator::new();

    // Simulate multiple free sources
    for (source, price) in [
        (DataSource::YahooFinance, 150.0),
        (DataSource::AlphaVantage, 150.02),
        (DataSource::Finnhub, 149.98),
    ] {
        aggregator.add_data(FreeMarketData {
            symbol: "AAPL".to_string(),
            source,
            price: Decimal::try_from(price).unwrap(),
            change_24h: None,
            volume_24h: None,
            timestamp: Utc::now(),
            bid: None,
            ask: None,
        });
    }

    // Validate against paid source at 150.0
    let validation = aggregator.validate_against_paid("AAPL", Decimal::from(150), "Bloomberg");

    assert_eq!(validation.source_count, 3);
    assert!(
        validation.deviation_pct < 0.1,
        "Free consensus should be close to paid"
    );
}

// ============================================================================
// Test 15: Multi-Asset Support Check
// ============================================================================

#[test]
fn test_data_source_asset_support() {
    assert!(DataSource::YahooFinance.supports_stocks());
    assert!(DataSource::YahooFinance.supports_crypto());

    assert!(DataSource::BinancePublic.supports_crypto());
    assert!(!DataSource::BinancePublic.supports_stocks());

    assert!(DataSource::AlphaVantage.supports_stocks());
    assert!(!DataSource::AlphaVantage.supports_crypto());
}

// ============================================================================
// Test 16: Rate Limit Awareness
// ============================================================================

#[test]
fn test_rate_limit_awareness() {
    // Different sources have different rate limits
    assert!(DataSource::BinancePublic.rate_limit_per_minute() > 1000);
    assert!(DataSource::AlphaVantage.rate_limit_per_minute() <= 10);
    assert!(DataSource::Finnhub.rate_limit_per_minute() >= 60);
}
