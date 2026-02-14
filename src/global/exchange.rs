//! Exchange definitions and configurations

use super::TradingHours;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Exchange identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExchangeId(pub String);

/// Geographic region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    NorthAmerica,
    Europe,
    AsiaPacific,
    LatinAmerica,
    MiddleEast,
    Africa,
}

/// Asset class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetClass {
    Stocks,
    Bonds,
    Options,
    Futures,
    Forex,
    Crypto,
    Commodities,
}

/// Exchange status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeStatus {
    Open,
    Closed,
    PreMarket,
    AfterHours,
    Holiday,
}

/// Exchange configuration
#[derive(Debug, Clone)]
pub struct Exchange {
    pub id: ExchangeId,
    pub name: String,
    pub region: Region,
    pub country: String,
    pub time_zone: String,
    pub trading_hours: TradingHours,
    pub supported_assets: Vec<AssetClass>,
    pub currency: String,
}

impl Exchange {
    /// Check if exchange is open
    pub fn is_open(&self, datetime: DateTime<Utc>, holidays: Option<&super::HolidayCalendar>) -> bool {
        self.trading_hours.is_open(datetime, holidays)
    }
    
    /// Check if symbol is supported
    pub fn supports_symbol(&self, _symbol: &str) -> bool {
        // Simplified - in reality would check symbol against exchange listings
        true
    }
    
    // Factory methods for major exchanges
    pub fn nyse() -> Self {
        Self {
            id: ExchangeId("NYSE".to_string()),
            name: "New York Stock Exchange".to_string(),
            region: Region::NorthAmerica,
            country: "US".to_string(),
            time_zone: "America/New_York".to_string(),
            trading_hours: TradingHours::us_equity(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds, AssetClass::Options],
            currency: "USD".to_string(),
        }
    }
    
    pub fn nasdaq() -> Self {
        Self {
            id: ExchangeId("NASDAQ".to_string()),
            name: "NASDAQ".to_string(),
            region: Region::NorthAmerica,
            country: "US".to_string(),
            time_zone: "America/New_York".to_string(),
            trading_hours: TradingHours::us_equity(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Options],
            currency: "USD".to_string(),
        }
    }
    
    pub fn tsx() -> Self {
        Self {
            id: ExchangeId("TSX".to_string()),
            name: "Toronto Stock Exchange".to_string(),
            region: Region::NorthAmerica,
            country: "CA".to_string(),
            time_zone: "America/Toronto".to_string(),
            trading_hours: TradingHours::us_equity(),
            supported_assets: vec![AssetClass::Stocks],
            currency: "CAD".to_string(),
        }
    }
    
    pub fn lse() -> Self {
        Self {
            id: ExchangeId("LSE".to_string()),
            name: "London Stock Exchange".to_string(),
            region: Region::Europe,
            country: "GB".to_string(),
            time_zone: "Europe/London".to_string(),
            trading_hours: TradingHours::lse(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds],
            currency: "GBP".to_string(),
        }
    }
    
    pub fn xetra() -> Self {
        Self {
            id: ExchangeId("XETRA".to_string()),
            name: "Xetra".to_string(),
            region: Region::Europe,
            country: "DE".to_string(),
            time_zone: "Europe/Berlin".to_string(),
            trading_hours: TradingHours::european_equity(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds],
            currency: "EUR".to_string(),
        }
    }
    
    pub fn euronext_paris() -> Self {
        Self {
            id: ExchangeId("EURONEXT_PARIS".to_string()),
            name: "Euronext Paris".to_string(),
            region: Region::Europe,
            country: "FR".to_string(),
            time_zone: "Europe/Paris".to_string(),
            trading_hours: TradingHours::european_equity(),
            supported_assets: vec![AssetClass::Stocks],
            currency: "EUR".to_string(),
        }
    }
    
    pub fn six_swiss() -> Self {
        Self {
            id: ExchangeId("SIX".to_string()),
            name: "SIX Swiss Exchange".to_string(),
            region: Region::Europe,
            country: "CH".to_string(),
            time_zone: "Europe/Zurich".to_string(),
            trading_hours: TradingHours::european_equity(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds],
            currency: "CHF".to_string(),
        }
    }
    
    pub fn omx_stockholm() -> Self {
        Self {
            id: ExchangeId("OMX".to_string()),
            name: "NASDAQ OMX Stockholm".to_string(),
            region: Region::Europe,
            country: "SE".to_string(),
            time_zone: "Europe/Stockholm".to_string(),
            trading_hours: TradingHours::european_equity(),
            supported_assets: vec![AssetClass::Stocks],
            currency: "SEK".to_string(),
        }
    }
    
    pub fn borsa_italiana() -> Self {
        Self {
            id: ExchangeId("BIT".to_string()),
            name: "Borsa Italiana".to_string(),
            region: Region::Europe,
            country: "IT".to_string(),
            time_zone: "Europe/Rome".to_string(),
            trading_hours: TradingHours::european_equity(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds],
            currency: "EUR".to_string(),
        }
    }
    
    pub fn tse() -> Self {
        Self {
            id: ExchangeId("TSE".to_string()),
            name: "Tokyo Stock Exchange".to_string(),
            region: Region::AsiaPacific,
            country: "JP".to_string(),
            time_zone: "Asia/Tokyo".to_string(),
            trading_hours: TradingHours::tse(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds],
            currency: "JPY".to_string(),
        }
    }
    
    pub fn hkex() -> Self {
        Self {
            id: ExchangeId("HKEX".to_string()),
            name: "Hong Kong Stock Exchange".to_string(),
            region: Region::AsiaPacific,
            country: "HK".to_string(),
            time_zone: "Asia/Hong_Kong".to_string(),
            trading_hours: TradingHours::hkex(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds, AssetClass::Futures],
            currency: "HKD".to_string(),
        }
    }
    
    pub fn sgx() -> Self {
        Self {
            id: ExchangeId("SGX".to_string()),
            name: "Singapore Exchange".to_string(),
            region: Region::AsiaPacific,
            country: "SG".to_string(),
            time_zone: "Asia/Singapore".to_string(),
            trading_hours: TradingHours::asian_equity(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Futures],
            currency: "SGD".to_string(),
        }
    }
    
    pub fn asx() -> Self {
        Self {
            id: ExchangeId("ASX".to_string()),
            name: "Australian Securities Exchange".to_string(),
            region: Region::AsiaPacific,
            country: "AU".to_string(),
            time_zone: "Australia/Sydney".to_string(),
            trading_hours: TradingHours::asx(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds, AssetClass::Options],
            currency: "AUD".to_string(),
        }
    }
    
    pub fn nse() -> Self {
        Self {
            id: ExchangeId("NSE".to_string()),
            name: "National Stock Exchange of India".to_string(),
            region: Region::AsiaPacific,
            country: "IN".to_string(),
            time_zone: "Asia/Kolkata".to_string(),
            trading_hours: TradingHours::nse(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Futures, AssetClass::Options],
            currency: "INR".to_string(),
        }
    }
    
    pub fn b3() -> Self {
        Self {
            id: ExchangeId("B3".to_string()),
            name: "B3 - Brasil Bolsa Balcão".to_string(),
            region: Region::LatinAmerica,
            country: "BR".to_string(),
            time_zone: "America/Sao_Paulo".to_string(),
            trading_hours: TradingHours::b3(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Futures, AssetClass::Options],
            currency: "BRL".to_string(),
        }
    }
    
    pub fn bmv() -> Self {
        Self {
            id: ExchangeId("BMV".to_string()),
            name: "Mexican Stock Exchange".to_string(),
            region: Region::LatinAmerica,
            country: "MX".to_string(),
            time_zone: "America/Mexico_City".to_string(),
            trading_hours: TradingHours::us_equity(),
            supported_assets: vec![AssetClass::Stocks, AssetClass::Bonds],
            currency: "MXN".to_string(),
        }
    }
}
