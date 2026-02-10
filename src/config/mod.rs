//! Configuration management for Investor OS
//!
//! Environment-based configuration with validation

mod trading_mode;

pub use trading_mode::{TradingMode, TradingModeConfig, ModeNotifications};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;

/// Configuration errors
#[derive(Error, Debug, Clone)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    
    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Result type for configuration operations
pub type Result<T> = std::result::Result<T, ConfigError>;

/// Application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    /// Application environment
    pub environment: Environment,
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Redis configuration
    pub redis: RedisConfig,
    /// Trading configuration
    pub trading: TradingConfig,
    /// Trading mode configuration
    pub trading_mode: TradingModeConfig,
    /// Risk configuration
    pub risk: RiskConfig,
    /// Broker configuration
    pub brokers: HashMap<String, BrokerConfig>,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Environment type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    /// Check if production
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
    
    /// Check if development
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Development
    }
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Host to bind to
    pub host: String,
    /// Port to listen on
    pub port: u16,
    /// Request timeout
    pub request_timeout_secs: u64,
    /// Graceful shutdown timeout
    pub shutdown_timeout_secs: u64,
    /// Rate limit requests per minute
    pub rate_limit_per_minute: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            request_timeout_secs: 30,
            shutdown_timeout_secs: 15,
            rate_limit_per_minute: 100,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    /// Maximum connections in pool
    pub max_connections: u32,
    /// Connection timeout
    pub connect_timeout_secs: u64,
    /// Enable query logging
    pub log_queries: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost:5432/investor_os".to_string(),
            max_connections: 10,
            connect_timeout_secs: 5,
            log_queries: false,
        }
    }
}

/// Redis configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    /// Redis URL
    pub url: String,
    /// Connection timeout
    pub connect_timeout_secs: u64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            connect_timeout_secs: 5,
        }
    }
}

/// Trading configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingConfig {
    /// Paper trading mode
    pub paper_trading: bool,
    /// Default commission rate
    pub commission_rate: Decimal,
    /// Maximum position size (as % of portfolio)
    pub max_position_pct: Decimal,
    /// Minimum CQ threshold for trading
    pub min_cq_threshold: f64,
    /// Rebalance frequency in days
    pub rebalance_frequency_days: i64,
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            paper_trading: true,
            commission_rate: Decimal::from(1) / Decimal::from(1000), // 0.1%
            max_position_pct: Decimal::from(10) / Decimal::from(100), // 10%
            min_cq_threshold: 0.65,
            rebalance_frequency_days: 1,
        }
    }
}

/// Risk configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskConfig {
    /// Maximum portfolio VaR (95%)
    pub max_var_95: Decimal,
    /// Maximum daily loss limit
    pub max_daily_loss: Decimal,
    /// Maximum drawdown limit
    pub max_drawdown: Decimal,
    /// Kill switch enabled
    pub kill_switch_enabled: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_var_95: Decimal::from(5) / Decimal::from(100), // 5%
            max_daily_loss: Decimal::from(10) / Decimal::from(100), // 10%
            max_drawdown: Decimal::from(20) / Decimal::from(100), // 20%
            kill_switch_enabled: true,
        }
    }
}

/// Broker configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrokerConfig {
    /// Broker type
    pub broker_type: BrokerType,
    /// API key (or reference to secret)
    pub api_key: String,
    /// API secret (or reference to secret)
    pub api_secret: String,
    /// Paper trading for this broker
    pub paper_trading: bool,
    /// Rate limit per second
    pub rate_limit_per_second: u32,
}

/// Broker type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BrokerType {
    Alpaca,
    InteractiveBrokers,
    Binance,
    Oanda,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open
    pub failure_threshold: u32,
    /// Success threshold to close
    pub success_threshold: u32,
    /// Timeout before half-open (seconds)
    pub timeout_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_secs: 30,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Enable JSON formatting
    pub json_format: bool,
    /// Enable OpenTelemetry
    pub opentelemetry_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            opentelemetry_enabled: false,
        }
    }
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let environment = std::env::var("INVESTOR_ENV")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "production" | "prod" => Some(Environment::Production),
                "staging" | "stage" => Some(Environment::Staging),
                "development" | "dev" => Some(Environment::Development),
                _ => None,
            })
            .unwrap_or_default();

        let server = ServerConfig {
            host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("SERVER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000),
            request_timeout_secs: std::env::var("REQUEST_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            shutdown_timeout_secs: std::env::var("SHUTDOWN_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(15),
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
        };

        let database = DatabaseConfig {
            url: std::env::var("DATABASE_URL")
                .map_err(|_| ConfigError::MissingEnvVar("DATABASE_URL".to_string()))?,
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            connect_timeout_secs: std::env::var("DB_CONNECT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            log_queries: std::env::var("DB_LOG_QUERIES")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(false),
        };

        let redis = RedisConfig {
            url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            connect_timeout_secs: std::env::var("REDIS_CONNECT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        };

        let trading = TradingConfig {
            paper_trading: std::env::var("PAPER_TRADING")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(true),
            commission_rate: std::env::var("COMMISSION_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .map(|f: f64| Decimal::try_from(f).ok())
                .flatten()
                .unwrap_or_else(|| Decimal::from(1) / Decimal::from(1000)),
            max_position_pct: std::env::var("MAX_POSITION_PCT")
                .ok()
                .and_then(|s| s.parse().ok())
                .map(|f: f64| Decimal::try_from(f).ok())
                .flatten()
                .unwrap_or_else(|| Decimal::from(10) / Decimal::from(100)),
            min_cq_threshold: std::env::var("MIN_CQ_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.65),
            rebalance_frequency_days: std::env::var("REBALANCE_FREQUENCY_DAYS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1),
        };

        let trading_mode = TradingModeConfig {
            mode: std::env::var("TRADING_MODE")
                .ok()
                .and_then(|s| match s.to_lowercase().as_str() {
                    "manual" => Some(TradingMode::Manual),
                    "semi_auto" | "semi-auto" | "semiauto" => Some(TradingMode::SemiAuto),
                    "fully_auto" | "fully-auto" | "fullyauto" | "auto" => Some(TradingMode::FullyAuto),
                    _ => None,
                })
                .unwrap_or_default(),
            auto_execute_cq_threshold: std::env::var("AUTO_EXECUTE_CQ_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(80),
            max_auto_trade_value: std::env::var("MAX_AUTO_TRADE_VALUE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000.0),
            notifications: ModeNotifications::default(),
        };

        let risk = RiskConfig {
            max_var_95: std::env::var("MAX_VAR_95")
                .ok()
                .and_then(|s| s.parse().ok())
                .map(|f: f64| Decimal::try_from(f).ok())
                .flatten()
                .unwrap_or_else(|| Decimal::from(5) / Decimal::from(100)),
            max_daily_loss: std::env::var("MAX_DAILY_LOSS")
                .ok()
                .and_then(|s| s.parse().ok())
                .map(|f: f64| Decimal::try_from(f).ok())
                .flatten()
                .unwrap_or_else(|| Decimal::from(10) / Decimal::from(100)),
            max_drawdown: std::env::var("MAX_DRAWDOWN")
                .ok()
                .and_then(|s| s.parse().ok())
                .map(|f: f64| Decimal::try_from(f).ok())
                .flatten()
                .unwrap_or_else(|| Decimal::from(20) / Decimal::from(100)),
            kill_switch_enabled: std::env::var("KILL_SWITCH_ENABLED")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(true),
        };

        let circuit_breaker = CircuitBreakerConfig {
            failure_threshold: std::env::var("CB_FAILURE_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            success_threshold: std::env::var("CB_SUCCESS_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            timeout_secs: std::env::var("CB_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        };

        let logging = LoggingConfig {
            level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            json_format: std::env::var("LOG_JSON_FORMAT")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(false),
            opentelemetry_enabled: std::env::var("OTEL_ENABLED")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(false),
        };

        Ok(Self {
            environment,
            server,
            database,
            redis,
            trading,
            trading_mode,
            risk,
            brokers: HashMap::new(),
            circuit_breaker,
            logging,
        })
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate database URL format
        if !self.database.url.starts_with("postgres://") {
            return Err(ConfigError::InvalidValue(
                "DATABASE_URL must start with postgres://".to_string()
            ));
        }

        // Validate Redis URL format
        if !self.redis.url.starts_with("redis://") {
            return Err(ConfigError::InvalidValue(
                "REDIS_URL must start with redis://".to_string()
            ));
        }

        // Validate port range
        if self.server.port == 0 || self.server.port > 65535 {
            return Err(ConfigError::InvalidValue(
                "SERVER_PORT must be between 1 and 65535".to_string()
            ));
        }

        // Validate CQ threshold
        if self.trading.min_cq_threshold < 0.0 || self.trading.min_cq_threshold > 1.0 {
            return Err(ConfigError::InvalidValue(
                "MIN_CQ_THRESHOLD must be between 0 and 1".to_string()
            ));
        }

        Ok(())
    }

    /// Load and validate from environment
    pub fn load() -> Result<Self> {
        let config = Self::from_env()?;
        config.validate()?;
        Ok(config)
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            environment: Environment::Development,
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            trading: TradingConfig::default(),
            trading_mode: TradingModeConfig::default(),
            risk: RiskConfig::default(),
            brokers: HashMap::new(),
            circuit_breaker: CircuitBreakerConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 3000);
        assert!(config.trading.paper_trading);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AppConfig::default();
        
        // Valid config
        assert!(config.validate().is_ok());
        
        // Invalid database URL
        config.database.url = "invalid".to_string();
        assert!(config.validate().is_err());
        
        // Reset and test invalid CQ threshold
        config.database.url = "postgres://localhost".to_string();
        config.trading.min_cq_threshold = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_environment_detection() {
        assert!(Environment::Production.is_production());
        assert!(!Environment::Production.is_development());
        assert!(Environment::Development.is_development());
    }
}
