//! Free Data Sources Module
//!
//! Integrates free-tier APIs for market data:
//! - Yahoo Finance (unofficial)
//! - Alpha Vantage (free tier)
//! - Finnhub (free tier)
//! - CryptoCompare (free tier)
//! - Binance Public API (free)
//! - Twelve Data (free tier)
//!
//! AI Pattern Discovery:
//! - Cross-source price validation
//! - Pattern detection across sources
//! - Data quality scoring
//! - Free vs Paid accuracy comparison

pub mod aggregator;

pub use aggregator::{FreeDataAggregator, DataSourceQuality, CrossSourceValidation};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Free data source error
#[derive(Error, Debug, Clone)]
pub enum FreeDataError {
    #[error("API rate limit exceeded: {0}")]
    RateLimit(String),
    
    #[error("API error {0}: {1}")]
    ApiError(String, String),
    
    #[error("Data not available: {0}@{1}")]
    DataNotAvailable(String, String),
    
    #[error("Validation failed: {0}")]
    ValidationError(String),
}

/// Available free data sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataSource {
    YahooFinance,
    AlphaVantage,
    Finnhub,
    CryptoCompare,
    BinancePublic,
    TwelveData,
}

impl DataSource {
    pub fn name(&self) -> &'static str {
        match self {
            DataSource::YahooFinance => "Yahoo Finance",
            DataSource::AlphaVantage => "Alpha Vantage",
            DataSource::Finnhub => "Finnhub",
            DataSource::CryptoCompare => "CryptoCompare",
            DataSource::BinancePublic => "Binance Public",
            DataSource::TwelveData => "Twelve Data",
        }
    }
    
    pub fn rate_limit_per_minute(&self) -> u32 {
        match self {
            DataSource::YahooFinance => 100,
            DataSource::AlphaVantage => 5,
            DataSource::Finnhub => 60,
            DataSource::CryptoCompare => 100,
            DataSource::BinancePublic => 1200,
            DataSource::TwelveData => 8,
        }
    }
    
    pub fn supports_stocks(&self) -> bool {
        matches!(self, 
            DataSource::YahooFinance | 
            DataSource::AlphaVantage | 
            DataSource::Finnhub |
            DataSource::TwelveData
        )
    }
    
    pub fn supports_crypto(&self) -> bool {
        matches!(self,
            DataSource::YahooFinance |
            DataSource::CryptoCompare |
            DataSource::BinancePublic
        )
    }
}

/// Standardized market data from any free source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeMarketData {
    pub symbol: String,
    pub source: DataSource,
    pub price: Decimal,
    pub change_24h: Option<Decimal>,
    pub volume_24h: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub bid: Option<Decimal>,
    pub ask: Option<Decimal>,
}

/// Data quality metrics for AI pattern discovery
#[derive(Debug, Clone, Default)]
pub struct DataQualityMetrics {
    pub accuracy_vs_paid: f32,
    pub avg_latency_ms: u64,
    pub freshness_secs: u64,
    pub uptime_pct: f32,
    pub sample_count: u64,
    pub error_rate: f32,
}

/// Cross-source price comparison
#[derive(Debug, Clone)]
pub struct CrossSourcePrice {
    pub symbol: String,
    pub prices: HashMap<DataSource, Decimal>,
    pub consensus_price: Decimal,
    pub spread: Decimal,
    pub spread_pct: f64,
    pub outlier_sources: Vec<DataSource>,
}

impl CrossSourcePrice {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            prices: HashMap::new(),
            consensus_price: Decimal::ZERO,
            spread: Decimal::ZERO,
            spread_pct: 0.0,
            outlier_sources: Vec::new(),
        }
    }
    
    pub fn add_price(&mut self, source: DataSource, price: Decimal) {
        self.prices.insert(source, price);
    }
    
    pub fn calculate_consensus(&mut self) {
        let mut price_values: Vec<Decimal> = self.prices.values().copied().collect();
        price_values.sort();
        
        let len = price_values.len();
        if len == 0 {
            return;
        }
        
        self.consensus_price = if len.is_multiple_of(2) {
            (price_values[len/2 - 1] + price_values[len/2]) / Decimal::from(2)
        } else {
            price_values[len/2]
        };
        
        let min = price_values[0];
        let max = price_values[len - 1];
        self.spread = max - min;
        
        if !self.consensus_price.is_zero() {
            let spread_f64: f64 = self.spread.try_into().unwrap_or(0.0);
            let consensus_f64: f64 = self.consensus_price.try_into().unwrap_or(1.0);
            self.spread_pct = (spread_f64 / consensus_f64) * 100.0;
        }
        
        self.outlier_sources.clear();
        for (source, price) in &self.prices {
            let deviation = (*price - self.consensus_price).abs();
            if !self.consensus_price.is_zero() {
                let dev_pct: f64 = (deviation / self.consensus_price).try_into().unwrap_or(0.0);
                if dev_pct > 0.01 {
                    self.outlier_sources.push(*source);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_source_capabilities() {
        assert!(DataSource::YahooFinance.supports_stocks());
        assert!(DataSource::YahooFinance.supports_crypto());
        assert!(!DataSource::BinancePublic.supports_stocks());
        assert!(DataSource::BinancePublic.supports_crypto());
    }

    #[test]
    fn test_cross_source_consensus() {
        let mut cross = CrossSourcePrice::new("AAPL");
        
        cross.add_price(DataSource::YahooFinance, Decimal::from(150));
        cross.add_price(DataSource::AlphaVantage, Decimal::try_from(150.50).unwrap());
        cross.add_price(DataSource::Finnhub, Decimal::try_from(149.80).unwrap());
        
        cross.calculate_consensus();
        
        assert!(cross.consensus_price > Decimal::ZERO);
        assert!(cross.spread > Decimal::ZERO);
    }

    #[test]
    fn test_outlier_detection() {
        let mut cross = CrossSourcePrice::new("AAPL");
        
        cross.add_price(DataSource::YahooFinance, Decimal::from(150));
        cross.add_price(DataSource::AlphaVantage, Decimal::try_from(150.20).unwrap());
        cross.add_price(DataSource::Finnhub, Decimal::try_from(149.90).unwrap());
        cross.add_price(DataSource::TwelveData, Decimal::from(160)); // Outlier
        
        cross.calculate_consensus();
        
        assert!(!cross.outlier_sources.is_empty());
        assert!(cross.outlier_sources.contains(&DataSource::TwelveData));
    }
}
