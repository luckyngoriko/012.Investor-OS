//! Exchange Definitions and Traits
//!
//! Defines the Exchange trait and concrete implementations for major exchanges

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::fmt;

use super::{calendar::MarketHours, Region, Result};

/// Exchange trait - unified interface for all exchanges
#[async_trait]
pub trait Exchange: Send + Sync + std::fmt::Debug {
    /// Get exchange identifier
    fn id(&self) -> ExchangeId;
    
    /// Get exchange name
    fn name(&self) -> &str;
    
    /// Get exchange region
    fn region(&self) -> Region;
    
    /// Check if exchange is currently open
    fn is_open(&self) -> bool;
    
    /// Get current market hours
    fn market_hours(&self) -> MarketHours;
    
    /// Connect to exchange
    async fn connect(&mut self) -> Result<()>;
    
    /// Disconnect from exchange
    async fn disconnect(&mut self) -> Result<()>;
    
    /// Check if connected
    fn is_connected(&self) -> bool;
    
    /// Get quote for symbol
    fn get_quote(&self, symbol: &str) -> Option<ExchangeQuote>;
    
    /// Get status
    fn status(&self) -> ExchangeStatus;
}

/// Exchange identifiers - supports 50+ global exchanges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExchangeId {
    // Americas
    NYSE,
    NASDAQ,
    TSX,           // Toronto
    Bovespa,       // Brazil
    BMV,           // Mexico
    
    // Europe
    LSE,           // London
    Xetra,         // Germany
    EuronextParis,
    EuronextAmsterdam,
    SIX,           // Switzerland
    BME,           // Spain
    BorsaItaliana,
    NasdaqNordic,
    WienerBoerse,  // Vienna
    
    // Asia-Pacific
    TSE,           // Tokyo
    HKEX,          // Hong Kong
    SSE,           // Shanghai
    SZSE,          // Shenzhen
    ASX,           // Australia
    NSE,           // India (National Stock Exchange)
    BSE,           // India (Bombay)
    KRX,           // Korea
    SGX,           // Singapore
    SET,           // Thailand
    
    // Emerging
    JSE,           // Johannesburg
    MOEX,          // Moscow
    Qatar,
    SaudiTadawul,
    TelAviv,
    Istanbul,
    
    // Crypto
    Binance,
    Coinbase,
    Kraken,
}

impl ExchangeId {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeId::NYSE => "NYSE",
            ExchangeId::NASDAQ => "NASDAQ",
            ExchangeId::TSX => "TSX",
            ExchangeId::Bovespa => "Bovespa",
            ExchangeId::BMV => "BMV",
            ExchangeId::LSE => "LSE",
            ExchangeId::Xetra => "Xetra",
            ExchangeId::EuronextParis => "EuronextParis",
            ExchangeId::EuronextAmsterdam => "EuronextAmsterdam",
            ExchangeId::SIX => "SIX",
            ExchangeId::BME => "BME",
            ExchangeId::BorsaItaliana => "BorsaItaliana",
            ExchangeId::NasdaqNordic => "NasdaqNordic",
            ExchangeId::WienerBoerse => "WienerBoerse",
            ExchangeId::TSE => "TSE",
            ExchangeId::HKEX => "HKEX",
            ExchangeId::SSE => "SSE",
            ExchangeId::SZSE => "SZSE",
            ExchangeId::ASX => "ASX",
            ExchangeId::NSE => "NSE",
            ExchangeId::BSE => "BSE",
            ExchangeId::KRX => "KRX",
            ExchangeId::SGX => "SGX",
            ExchangeId::SET => "SET",
            ExchangeId::JSE => "JSE",
            ExchangeId::MOEX => "MOEX",
            ExchangeId::Qatar => "Qatar",
            ExchangeId::SaudiTadawul => "SaudiTadawul",
            ExchangeId::TelAviv => "TelAviv",
            ExchangeId::Istanbul => "Istanbul",
            ExchangeId::Binance => "Binance",
            ExchangeId::Coinbase => "Coinbase",
            ExchangeId::Kraken => "Kraken",
        }
    }

    pub fn region(&self) -> Region {
        match self {
            ExchangeId::NYSE | ExchangeId::NASDAQ | ExchangeId::TSX | 
            ExchangeId::Bovespa | ExchangeId::BMV => Region::Americas,
            
            ExchangeId::LSE | ExchangeId::Xetra | ExchangeId::EuronextParis |
            ExchangeId::EuronextAmsterdam | ExchangeId::SIX | ExchangeId::BME |
            ExchangeId::BorsaItaliana | ExchangeId::NasdaqNordic | ExchangeId::WienerBoerse |
            ExchangeId::MOEX | ExchangeId::Istanbul => Region::Europe,
            
            ExchangeId::TSE | ExchangeId::HKEX | ExchangeId::SSE | ExchangeId::SZSE |
            ExchangeId::ASX | ExchangeId::NSE | ExchangeId::BSE | ExchangeId::KRX |
            ExchangeId::SGX | ExchangeId::SET | ExchangeId::TelAviv => Region::AsiaPacific,
            
            ExchangeId::JSE | ExchangeId::Qatar | ExchangeId::SaudiTadawul => Region::Emerging,
            
            ExchangeId::Binance | ExchangeId::Coinbase | ExchangeId::Kraken => Region::Americas,
        }
    }
}

impl fmt::Display for ExchangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Exchange configuration
#[derive(Debug, Clone)]
pub struct ExchangeConfig {
    pub id: ExchangeId,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub sandbox: bool,
    pub rate_limit_per_second: u32,
}

impl ExchangeConfig {
    pub fn new(id: ExchangeId) -> Self {
        Self {
            id,
            api_key: None,
            api_secret: None,
            sandbox: true,
            rate_limit_per_second: 10,
        }
    }

    pub fn with_credentials(mut self, key: String, secret: String) -> Self {
        self.api_key = Some(key);
        self.api_secret = Some(secret);
        self
    }

    pub fn production(mut self) -> Self {
        self.sandbox = false;
        self
    }
}

/// Exchange connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Quote from an exchange
#[derive(Debug, Clone)]
pub struct ExchangeQuote {
    pub symbol: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
    pub last_price: Option<Decimal>,
    pub volume: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
}

/// Generic exchange implementation (template for specific exchanges)
#[derive(Debug)]
pub struct GenericExchange {
    config: ExchangeConfig,
    status: ExchangeStatus,
    market_hours: MarketHours,
    connected: bool,
    last_quotes: std::collections::HashMap<String, ExchangeQuote>,
}

impl GenericExchange {
    pub fn new(config: ExchangeConfig, market_hours: MarketHours) -> Self {
        Self {
            config,
            status: ExchangeStatus::Disconnected,
            market_hours,
            connected: false,
            last_quotes: std::collections::HashMap::new(),
        }
    }

    pub fn update_quote(&mut self, quote: ExchangeQuote) {
        self.last_quotes.insert(quote.symbol.clone(), quote);
    }
}

#[async_trait]
impl Exchange for GenericExchange {
    fn id(&self) -> ExchangeId {
        self.config.id
    }

    fn name(&self) -> &str {
        self.config.id.as_str()
    }

    fn region(&self) -> Region {
        self.config.id.region()
    }

    fn is_open(&self) -> bool {
        self.market_hours.is_open_now()
    }

    fn market_hours(&self) -> MarketHours {
        self.market_hours.clone()
    }

    async fn connect(&mut self) -> Result<()> {
        self.status = ExchangeStatus::Connecting;
        // Simulated connection
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        self.status = ExchangeStatus::Connected;
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.status = ExchangeStatus::Disconnected;
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn get_quote(&self, symbol: &str) -> Option<ExchangeQuote> {
        self.last_quotes.get(symbol).cloned()
    }

    fn status(&self) -> ExchangeStatus {
        self.status
    }
}

/// Factory for creating exchange instances
pub struct ExchangeFactory;

impl ExchangeFactory {
    /// Create an exchange by ID with default settings
    pub fn create(id: ExchangeId) -> Box<dyn Exchange> {
        use super::calendar::MarketHours;
        
        let config = ExchangeConfig::new(id);
        
        // Create appropriate market hours based on exchange
        let market_hours = match id {
            // US Markets: 9:30 - 16:00 ET
            ExchangeId::NYSE | ExchangeId::NASDAQ => {
                MarketHours::us_equity()
            }
            
            // European Markets: 9:00 - 17:30 local
            ExchangeId::LSE | ExchangeId::Xetra | ExchangeId::EuronextParis => {
                MarketHours::european()
            }
            
            // Asian Markets
            ExchangeId::TSE => MarketHours::tokyo(),
            ExchangeId::HKEX => MarketHours::hong_kong(),
            ExchangeId::SSE | ExchangeId::SZSE => MarketHours::china(),
            
            // Crypto: 24/7
            ExchangeId::Binance | ExchangeId::Coinbase | ExchangeId::Kraken => {
                MarketHours::crypto()
            }
            
            _ => MarketHours::default(),
        };

        Box::new(GenericExchange::new(config, market_hours))
    }

    /// Create all major exchanges
    pub fn create_all_major() -> Vec<Box<dyn Exchange>> {
        vec![
            Self::create(ExchangeId::NYSE),
            Self::create(ExchangeId::NASDAQ),
            Self::create(ExchangeId::LSE),
            Self::create(ExchangeId::Xetra),
            Self::create(ExchangeId::EuronextParis),
            Self::create(ExchangeId::TSE),
            Self::create(ExchangeId::HKEX),
            Self::create(ExchangeId::ASX),
            Self::create(ExchangeId::Binance),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_id_display() {
        assert_eq!(ExchangeId::NYSE.to_string(), "NYSE");
        assert_eq!(ExchangeId::LSE.to_string(), "LSE");
    }

    #[test]
    fn test_exchange_id_region() {
        assert!(matches!(ExchangeId::NYSE.region(), Region::Americas));
        assert!(matches!(ExchangeId::LSE.region(), Region::Europe));
        assert!(matches!(ExchangeId::TSE.region(), Region::AsiaPacific));
        assert!(matches!(ExchangeId::JSE.region(), Region::Emerging));
    }

    #[test]
    fn test_exchange_config() {
        let config = ExchangeConfig::new(ExchangeId::NYSE)
            .with_credentials("key".to_string(), "secret".to_string())
            .production();

        assert!(!config.sandbox);
        assert_eq!(config.api_key, Some("key".to_string()));
    }

    #[tokio::test]
    async fn test_generic_exchange() {
        let config = ExchangeConfig::new(ExchangeId::NYSE);
        let mut exchange = GenericExchange::new(config, MarketHours::default());

        assert!(!exchange.is_connected());
        
        exchange.connect().await.unwrap();
        assert!(exchange.is_connected());
        
        exchange.disconnect().await.unwrap();
        assert!(!exchange.is_connected());
    }
}
