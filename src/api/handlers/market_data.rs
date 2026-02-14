//! Market Data API Handlers
//!
//! Real-time and historical market data endpoints

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::api::handlers::ApiResponse;

/// Price quote response
#[derive(Serialize, Deserialize)]
pub struct QuoteResponse {
    pub ticker: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub last_price: Decimal,
    pub volume: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Historical price bar
#[derive(Serialize, Deserialize)]
pub struct PriceBar {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

/// GET /api/market-data/quote/:ticker - Get current quote
pub async fn get_quote(
    Path(ticker): Path<String>,
) -> Result<Json<ApiResponse<QuoteResponse>>, StatusCode> {
    // Generate simulated quote
    // In production: fetch from market data provider (Polygon, Alpha Vantage)
    let quote = generate_simulated_quote(&ticker);
    
    Ok(Json(ApiResponse::success(quote)))
}

/// Historical data request
#[derive(Deserialize)]
pub struct HistoricalRequest {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub interval: Option<String>, // "1d", "1h", "15m"
}

/// GET /api/market-data/historical/:ticker - Get historical prices
pub async fn get_historical(
    Path(ticker): Path<String>,
    Query(req): Query<HistoricalRequest>,
) -> Result<Json<ApiResponse<Vec<PriceBar>>>, StatusCode> {
    // Generate simulated historical data
    let bars = generate_simulated_historical(&ticker, req.from, req.to);
    
    Ok(Json(ApiResponse::success(bars)))
}

/// Market status response
#[derive(Serialize, Deserialize)]
pub struct MarketStatusResponse {
    pub market_open: bool,
    pub next_open: Option<DateTime<Utc>>,
    pub next_close: Option<DateTime<Utc>>,
    pub session: String,
}

/// GET /api/market-data/status - Get market status
pub async fn get_market_status(
) -> Result<Json<ApiResponse<MarketStatusResponse>>, StatusCode> {
    let now = Utc::now();
    
    // Simple check: US markets open 9:30-16:00 ET, Mon-Fri
    // For demo, always return open
    let response = MarketStatusResponse {
        market_open: true,
        next_open: None,
        next_close: Some(now + chrono::Duration::hours(4)),
        session: "Regular".to_string(),
    };
    
    Ok(Json(ApiResponse::success(response)))
}

/// Order book level
#[derive(Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}

/// Order book response
#[derive(Serialize, Deserialize)]
pub struct OrderBookResponse {
    pub ticker: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: DateTime<Utc>,
}

/// GET /api/market-data/orderbook/:ticker - Get order book
pub async fn get_order_book(
    Path(ticker): Path<String>,
) -> Result<Json<ApiResponse<OrderBookResponse>>, StatusCode> {
    // Generate simulated order book
    let book = generate_simulated_order_book(&ticker);
    
    Ok(Json(ApiResponse::success(book)))
}

// ============ Simulated Data Generation ============

fn generate_simulated_quote(ticker: &str) -> QuoteResponse {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    // Generate realistic price based on ticker first letter
    let base_price: f64 = match ticker.chars().next() {
        Some('A') => 150.0,
        Some('G') => 140.0,
        Some('M') => 380.0,
        Some('T') => 220.0,
        _ => 100.0,
    };
    
    let variation: f64 = rng.gen_range(-0.02..0.02);
    let last_price = base_price * (1.0 + variation);
    let bid = last_price * 0.999;
    let ask = last_price * 1.001;
    
    QuoteResponse {
        ticker: ticker.to_string(),
        bid: Decimal::try_from(bid).unwrap_or(Decimal::ZERO),
        ask: Decimal::try_from(ask).unwrap_or(Decimal::ZERO),
        last_price: Decimal::try_from(last_price).unwrap_or(Decimal::ZERO),
        volume: Decimal::from(rng.gen_range(1000000..10000000)),
        timestamp: Utc::now(),
    }
}

fn generate_simulated_historical(
    _ticker: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Vec<PriceBar> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let mut bars = Vec::new();
    let mut current = from;
    let mut price = 100.0; // Starting price
    
    while current < to {
        // Generate OHLC
        let change: f64 = rng.gen_range(-0.02..0.02);
        let open = price;
        let close = price * (1.0 + change);
        let high = open.max(close) * (1.0 + rng.gen_range(0.0..0.01));
        let low = open.min(close) * (1.0 - rng.gen_range(0.0..0.01));
        let volume = rng.gen_range(1000000..10000000) as i64;
        
        bars.push(PriceBar {
            timestamp: current,
            open: Decimal::try_from(open).unwrap_or(Decimal::ZERO),
            high: Decimal::try_from(high).unwrap_or(Decimal::ZERO),
            low: Decimal::try_from(low).unwrap_or(Decimal::ZERO),
            close: Decimal::try_from(close).unwrap_or(Decimal::ZERO),
            volume: Decimal::from(volume),
        });
        
        price = close;
        current += chrono::Duration::days(1);
    }
    
    bars
}

fn generate_simulated_order_book(ticker: &str) -> OrderBookResponse {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    let base_price: f64 = 100.0;
    
    let mut bids = Vec::new();
    let mut asks = Vec::new();
    
    // Generate 5 levels on each side
    for i in 0..5 {
        let bid_price = base_price * (1.0 - 0.001 * (i as f64 + 1.0));
        let ask_price = base_price * (1.0 + 0.001 * (i as f64 + 1.0));
        
        bids.push(OrderBookLevel {
            price: Decimal::try_from(bid_price).unwrap_or(Decimal::ZERO),
            quantity: Decimal::from(rng.gen_range(100..1000)),
        });
        
        asks.push(OrderBookLevel {
            price: Decimal::try_from(ask_price).unwrap_or(Decimal::ZERO),
            quantity: Decimal::from(rng.gen_range(100..1000)),
        });
    }
    
    OrderBookResponse {
        ticker: ticker.to_string(),
        bids,
        asks,
        timestamp: Utc::now(),
    }
}
