//! Integration tests for Sprint 11: Multi-Asset Expansion

use investor_os::broker::binance::{BinanceClient, TOP_CRYPTOS};
use investor_os::broker::multi_asset::{
    AssetClass, MultiAssetPortfolio, OrderRouter, OrderSide, OrderType, UnifiedOrder,
};
use investor_os::broker::oanda::{OandaClient, MAJOR_PAIRS};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Test Binance client creation
#[test]
fn test_binance_client_creation() {
    let client = BinanceClient::new("test_key".to_string(), "test_secret".to_string());

    assert_eq!(client.base_url(), "https://api.binance.com");
}

/// Test OANDA client creation
#[test]
fn test_oanda_client_creation() {
    let practice = OandaClient::new("test_key".to_string(), "account_123".to_string(), true);
    assert_eq!(practice.base_url(), "https://api-fxpractice.oanda.com");

    let live = OandaClient::new("test_key".to_string(), "account_123".to_string(), false);
    assert_eq!(live.base_url(), "https://api-fxtrade.oanda.com");
}

/// Test top crypto symbols
#[test]
fn test_top_crypto_symbols() {
    assert!(TOP_CRYPTOS.contains(&"BTCUSDT"));
    assert!(TOP_CRYPTOS.contains(&"ETHUSDT"));
    assert!(TOP_CRYPTOS.contains(&"SOLUSDT"));
    assert_eq!(TOP_CRYPTOS.len(), 5);
}

/// Test major forex pairs
#[test]
fn test_major_forex_pairs() {
    assert!(MAJOR_PAIRS.contains(&"EUR_USD"));
    assert!(MAJOR_PAIRS.contains(&"GBP_USD"));
    assert!(MAJOR_PAIRS.contains(&"USD_JPY"));
    assert_eq!(MAJOR_PAIRS.len(), 7);
}

/// Test multi-asset portfolio
#[test]
fn test_multi_asset_portfolio() {
    let mut portfolio = MultiAssetPortfolio::new();

    // Add some crypto positions
    let balances = vec![
        investor_os::broker::binance::Balance {
            asset: "BTC".to_string(),
            free: Decimal::from(1),
            locked: Decimal::ZERO,
        },
        investor_os::broker::binance::Balance {
            asset: "ETH".to_string(),
            free: Decimal::from(10),
            locked: Decimal::ZERO,
        },
    ];

    let mut prices = HashMap::new();
    prices.insert("BTCUSDT".to_string(), Decimal::from(50000));
    prices.insert("ETHUSDT".to_string(), Decimal::from(3000));

    portfolio.add_crypto_positions(&balances, &prices);
    portfolio.calculate_total_value();

    // Total should be: 1*50000 + 10*3000 = 80000
    assert_eq!(portfolio.total_value_usd, Decimal::from(80000));

    // Should have 2 positions
    assert_eq!(portfolio.positions.len(), 2);
}

/// Test asset class display
#[test]
fn test_asset_class_display() {
    assert_eq!(format!("{}", AssetClass::Crypto), "Crypto");
    assert_eq!(format!("{}", AssetClass::Forex), "Forex");
    assert_eq!(format!("{}", AssetClass::Equity), "Equity");
}

/// Test portfolio allocation
#[test]
fn test_portfolio_allocation() {
    let mut portfolio = MultiAssetPortfolio::new();

    // Manually add positions
    portfolio
        .positions
        .push(investor_os::broker::multi_asset::MultiAssetPosition {
            symbol: "BTC".to_string(),
            asset_class: AssetClass::Crypto,
            quantity: Decimal::from(1),
            avg_cost: Decimal::from(40000),
            current_price: Decimal::from(50000),
            market_value: Decimal::from(50000),
            unrealized_pnl: Decimal::from(10000),
            currency: "USDT".to_string(),
        });

    portfolio.total_value_usd = Decimal::from(50000);

    let allocation = portfolio.get_allocation();
    let crypto_pct = allocation.get(&AssetClass::Crypto).unwrap();

    // Should be 100%
    assert_eq!(*crypto_pct, Decimal::from(100));
}

/// Test top positions
#[test]
fn test_top_positions() {
    let mut portfolio = MultiAssetPortfolio::new();

    portfolio
        .positions
        .push(investor_os::broker::multi_asset::MultiAssetPosition {
            symbol: "BTC".to_string(),
            asset_class: AssetClass::Crypto,
            quantity: Decimal::ONE,
            avg_cost: Decimal::from(40000),
            current_price: Decimal::from(50000),
            market_value: Decimal::from(50000),
            unrealized_pnl: Decimal::ZERO,
            currency: "USDT".to_string(),
        });

    portfolio
        .positions
        .push(investor_os::broker::multi_asset::MultiAssetPosition {
            symbol: "ETH".to_string(),
            asset_class: AssetClass::Crypto,
            quantity: Decimal::ONE,
            avg_cost: Decimal::from(2000),
            current_price: Decimal::from(3000),
            market_value: Decimal::from(3000),
            unrealized_pnl: Decimal::ZERO,
            currency: "USDT".to_string(),
        });

    let top = portfolio.get_top_positions(1);
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].symbol, "BTC");
}

/// Test unified order creation
#[test]
fn test_unified_order() {
    let order = UnifiedOrder {
        symbol: "BTC".to_string(),
        asset_class: AssetClass::Crypto,
        side: OrderSide::Buy,
        quantity: Decimal::from(1),
        order_type: OrderType::Market,
        price: None,
        stop_price: None,
    };

    assert_eq!(order.symbol, "BTC");
    assert_eq!(order.asset_class, AssetClass::Crypto);
    assert!(matches!(order.side, OrderSide::Buy));
    assert!(matches!(order.order_type, OrderType::Market));
}

/// Test empty portfolio
#[test]
fn test_empty_portfolio() {
    let portfolio = MultiAssetPortfolio::new();

    assert!(portfolio.positions.is_empty());
    assert!(portfolio.cash_balances.is_empty());
    assert_eq!(portfolio.total_value_usd, Decimal::ZERO);
}

/// Test portfolio with cash only
#[test]
fn test_cash_only_portfolio() {
    let mut portfolio = MultiAssetPortfolio::new();

    portfolio
        .cash_balances
        .insert("USD".to_string(), Decimal::from(10000));
    portfolio.calculate_total_value();

    assert_eq!(portfolio.total_value_usd, Decimal::from(10000));
    assert!(portfolio.positions.is_empty());
}
