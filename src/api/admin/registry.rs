//! Integration Registry
//!
//! Централизиран регистър на всички API интеграции и техния статус

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Тип на интеграцията
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationType {
    ExternalApi,      // Външно API (broker, market data)
    InternalModule,   // Вътрешен модул (analytics, ML)
    Database,         // База данни
    MessageQueue,     // Message queue (Redis, Kafka)
}

/// Статус на интеграцията
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationStatus {
    Connected,        // Свързано и работи
    Disconnected,     // Изключено/недостъпно
    Hardcoded,        // Hardcoded стойности (както е сега analytics)
    Stub,             // Stub имплементация
    NotImplemented,   // Не е имплементирано
    RequiresLicense,  // Изисква лиценз (FiatGateway)
}

/// Приоритет на интеграцията
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationPriority {
    Critical,         // Блокер за production
    High,             // Важно
    Medium,           // Нормално
    Low,              // Ниско
    Deprecated,       // Не се поддържа вече
}

/// Статус на имплементация на endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImplementationStatus {
    Implemented,      // Имплементирано и работи
    Stub,             // Stub/мок
    Hardcoded,        // Hardcoded стойности
    NotImplemented,   // Не е имплементирано
}

/// Поле за конфигурация
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub description: String,
    pub current_value: Option<String>,
}

/// Endpoint дефиниция
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub description: String,
    pub implementation_status: ImplementationStatus,
}

/// Интеграция дефиниция
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    pub id: String,
    pub name: String,
    pub description: String,
    pub integration_type: IntegrationType,
    pub status: IntegrationStatus,
    pub priority: IntegrationPriority,
    pub endpoints: Vec<ApiEndpoint>,
    pub config_fields: Vec<ConfigField>,
    pub connection_steps: Vec<String>,
    pub example_config: serde_json::Value,
    pub testing_commands: Vec<String>,
    pub last_check: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub documentation_url: String,
}

/// Регистър на всички интеграции
pub struct IntegrationRegistry;

impl IntegrationRegistry {
    pub fn new() -> Self {
        Self
    }
    
    /// Връща всички интеграции
    pub fn get_all_integrations(&self) -> Vec<Integration> {
        vec![
            self.analytics_integration(),
            self.risk_metrics_integration(),
            self.attribution_integration(),
            self.ml_prediction_integration(),
            self.broker_integration(),
            self.market_data_integration(),
            self.rag_integration(),
            self.treasury_integration(),
            self.fiat_gateway_integration(),
            self.fx_converter_integration(),
            self.database_integration(),
            self.redis_integration(),
        ]
    }
    
    /// Намира интеграция по ID
    pub fn get_integration(&self, id: &str) -> Option<Integration> {
        self.get_all_integrations().into_iter().find(|i| i.id == id)
    }
    
    // ==================== ANALYTICS INTEGRATIONS ====================
    
    fn analytics_integration(&self) -> Integration {
        Integration {
            id: "analytics-backtest".to_string(),
            name: "Analytics - Backtest Engine".to_string(),
            description: "Бекетстинг двигател за тестване на стратегии".to_string(),
            integration_type: IntegrationType::InternalModule,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::Critical,
            endpoints: vec![
                ApiEndpoint {
                    method: "POST".to_string(),
                    path: "/api/analytics/backtest".to_string(),
                    description: "Run backtest with strategy".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
            ],
            config_fields: vec![],
            connection_steps: vec![
                "✅ Свързано с BacktestEngine модула".to_string(),
                "✅ Изпълнява реални backtest симулации".to_string(),
                "⚠️ Използва mock данни (за production: свържи Market Data API)".to_string(),
            ],
            example_config: serde_json::json!({}),
            testing_commands: vec![
                "cargo test backtest".to_string(),
                "curl -X POST /api/analytics/backtest -d '{...}'".to_string(),
            ],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/analytics.md".to_string(),
        }
    }
    
    fn risk_metrics_integration(&self) -> Integration {
        Integration {
            id: "analytics-risk".to_string(),
            name: "Analytics - Risk Metrics".to_string(),
            description: "VaR, Sharpe, Sortino, Max Drawdown изчисления".to_string(),
            integration_type: IntegrationType::InternalModule,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::Critical,
            endpoints: vec![
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/analytics/risk".to_string(),
                    description: "Get portfolio risk metrics".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
            ],
            config_fields: vec![],
            connection_steps: vec![
                "✅ Свързано с RiskAnalyzer модула".to_string(),
                "✅ Използва реални VaR изчисления".to_string(),
                "⚠️ Връща симулирани returns (за production: свържи с портфолио БД)".to_string(),
            ],
            example_config: serde_json::json!({}),
            testing_commands: vec![
                "cargo test risk".to_string(),
                "curl '/api/analytics/risk?portfolio_id=test&lookback_days=252'".to_string(),
            ],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/risk.md".to_string(),
        }
    }
    
    fn attribution_integration(&self) -> Integration {
        Integration {
            id: "analytics-attribution".to_string(),
            name: "Analytics - Performance Attribution".to_string(),
            description: "Brinson-Fachler модел за декомпозиция на returns".to_string(),
            integration_type: IntegrationType::InternalModule,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::High,
            endpoints: vec![
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/analytics/attribution".to_string(),
                    description: "Get performance attribution by sector".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
            ],
            config_fields: vec![],
            connection_steps: vec![
                "✅ Свързано с AttributionAnalyzer модула".to_string(),
                "✅ Използва Brinson-Fachler модел".to_string(),
                "✅ Разделя returns на: allocation, selection, interaction".to_string(),
                "⚠️ Използва симулирани портфолио данни (за production: свържи с БД)".to_string(),
            ],
            example_config: serde_json::json!({}),
            testing_commands: vec![
                "cargo test attribution".to_string(),
                "curl '/api/analytics/attribution?portfolio_id=test&start_date=...'".to_string(),
            ],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/attribution.md".to_string(),
        }
    }
    
    fn ml_prediction_integration(&self) -> Integration {
        Integration {
            id: "ml-prediction".to_string(),
            name: "ML - Prediction Engine".to_string(),
            description: "Машинно обучение за предсказване на Composite Quality".to_string(),
            integration_type: IntegrationType::InternalModule,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::High,
            endpoints: vec![
                ApiEndpoint {
                    method: "POST".to_string(),
                    path: "/api/analytics/predict".to_string(),
                    description: "Get ML prediction for ticker".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
            ],
            config_fields: vec![],
            connection_steps: vec![
                "✅ Свързано с CQPredictor модула".to_string(),
                "✅ Използва FeaturePipeline за feature extraction".to_string(),
                "✅ Sigmoid активация за [0,1] score".to_string(),
                "⚠️ Използва симулирани сигнали (за production: свържи с Signals БД)".to_string(),
            ],
            example_config: serde_json::json!({
                "model_type": "CQPredictor",
                "features": 19,
                "threshold": 0.65
            }),
            testing_commands: vec![
                "cargo test ml".to_string(),
                "curl -X POST /api/analytics/predict -d '{\"ticker\":\"AAPL\"}'".to_string(),
            ],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/ml.md".to_string(),
        }
    }
    
    // ==================== BROKER INTEGRATIONS ====================
    
    fn broker_integration(&self) -> Integration {
        Integration {
            id: "broker-ibkr".to_string(),
            name: "Broker - Paper Trading".to_string(),
            description: "Paper Trading Broker за симулирани поръчки и позиции".to_string(),
            integration_type: IntegrationType::InternalModule,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::Critical,
            endpoints: vec![
                ApiEndpoint {
                    method: "POST".to_string(),
                    path: "/api/broker/orders".to_string(),
                    description: "Place order (Market/Limit)".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
                ApiEndpoint {
                    method: "DELETE".to_string(),
                    path: "/api/broker/orders/:id".to_string(),
                    description: "Cancel order".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/broker/positions".to_string(),
                    description: "Get all positions".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/broker/account".to_string(),
                    description: "Get account info".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
            ],
            config_fields: vec![],
            connection_steps: vec![
                "✅ PaperBroker е активен".to_string(),
                "✅ Поддържа Market и Limit поръчки".to_string(),
                "✅ Проследява позиции, P&L, комисионни".to_string(),
                "⚠️ За реални поръчки: свържи IBKR API".to_string(),
            ],
            example_config: serde_json::json!({
                "mode": "paper_trading",
                "initial_balance": 100000,
                "commission_rate": 0.001
            }),
            testing_commands: vec![
                "cargo test paper".to_string(),
                "curl /api/broker/account".to_string(),
                "curl -X POST /api/broker/orders -d '{\"ticker\":\"AAPL\",\"side\":\"buy\",\"quantity\":100,\"order_type\":\"market\"}'".to_string(),
            ],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/paper-trading.md".to_string(),
        }
    }
    
    fn market_data_integration(&self) -> Integration {
        Integration {
            id: "market-data".to_string(),
            name: "Market Data Provider".to_string(),
            description: "Пазарни данни от Polygon, Alpha Vantage, или Yahoo".to_string(),
            integration_type: IntegrationType::ExternalApi,
            status: IntegrationStatus::Stub,
            priority: IntegrationPriority::High,
            endpoints: vec![
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/market-data/quote/:ticker".to_string(),
                    description: "Get real-time price quote".to_string(),
                    implementation_status: ImplementationStatus::Stub,
                },
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/market-data/historical/:ticker".to_string(),
                    description: "Get historical OHLC prices".to_string(),
                    implementation_status: ImplementationStatus::Stub,
                },
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/market-data/status".to_string(),
                    description: "Get market open/closed status".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/market-data/orderbook/:ticker".to_string(),
                    description: "Get order book (bids/asks)".to_string(),
                    implementation_status: ImplementationStatus::Stub,
                },
            ],
            config_fields: vec![
                ConfigField {
                    name: "POLYGON_API_KEY".to_string(),
                    field_type: "secret".to_string(),
                    required: false,
                    description: "Polygon.io API Key".to_string(),
                    current_value: None,
                },
                ConfigField {
                    name: "ALPHA_VANTAGE_KEY".to_string(),
                    field_type: "secret".to_string(),
                    required: false,
                    description: "Alpha Vantage API Key".to_string(),
                    current_value: None,
                },
            ],
            connection_steps: vec![
                "✅ API endpoints са създадени".to_string(),
                "⚠️ Сега: stub данни (симулирани quotes)".to_string(),
                "1. За реални данни: регистрирай се в Polygon.io или Alpha Vantage".to_string(),
                "2. Конфигурирай API ключа в админ панела".to_string(),
            ],
            example_config: serde_json::json!({
                "provider": "polygon",
                "api_key": "your_key_here",
                "rate_limit": 5
            }),
            testing_commands: vec![
                "curl '/api/market-data/quote/AAPL'".to_string(),
                "curl '/api/market-data/historical/BTC-USD?from=2024-01-01&to=2024-12-31'".to_string(),
                "curl '/api/market-data/status'".to_string(),
            ],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://polygon.io/docs/".to_string(),
        }
    }
    
    // ==================== INTERNAL MODULES ====================
    
    fn rag_integration(&self) -> Integration {
        Integration {
            id: "rag".to_string(),
            name: "RAG - Document Search".to_string(),
            description: "Semantic search по SEC filings и earnings calls".to_string(),
            integration_type: IntegrationType::InternalModule,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::Medium,
            endpoints: vec![
                ApiEndpoint {
                    method: "POST".to_string(),
                    path: "/api/rag/search".to_string(),
                    description: "Semantic search".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
                ApiEndpoint {
                    method: "POST".to_string(),
                    path: "/api/rag/summarize".to_string(),
                    description: "Summarize documents".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
                ApiEndpoint {
                    method: "POST".to_string(),
                    path: "/api/rag/sec-filings".to_string(),
                    description: "Process SEC filing".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
            ],
            config_fields: vec![],
            connection_steps: vec!["Работи - няма нужда от конфигурация".to_string()],
            example_config: serde_json::json!({}),
            testing_commands: vec!["cargo test rag".to_string()],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/rag.md".to_string(),
        }
    }
    
    fn treasury_integration(&self) -> Integration {
        Integration {
            id: "treasury".to_string(),
            name: "Treasury Module".to_string(),
            description: "Управление на капитал - депозити, тегления, yield".to_string(),
            integration_type: IntegrationType::InternalModule,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::High,
            endpoints: vec![
                ApiEndpoint {
                    method: "POST".to_string(),
                    path: "/api/treasury/deposit".to_string(),
                    description: "Process deposit".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
                ApiEndpoint {
                    method: "POST".to_string(),
                    path: "/api/treasury/withdraw".to_string(),
                    description: "Process withdrawal".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
                ApiEndpoint {
                    method: "GET".to_string(),
                    path: "/api/treasury/balance".to_string(),
                    description: "Get balances".to_string(),
                    implementation_status: ImplementationStatus::Implemented,
                },
            ],
            config_fields: vec![
                ConfigField {
                    name: "FIREBLOCKS_API_KEY".to_string(),
                    field_type: "secret".to_string(),
                    required: false,
                    description: "Fireblocks API Key (за реални crypto транзакции)".to_string(),
                    current_value: None,
                },
            ],
            connection_steps: vec![
                "✅ Treasury модулът работи (paper trading)".to_string(),
                "За реални транзакции: свържи Fireblocks".to_string(),
            ],
            example_config: serde_json::json!({
                "mode": "paper_trading",
                "supported_currencies": ["USDC", "BTC", "ETH"]
            }),
            testing_commands: vec!["cargo test treasury".to_string()],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/treasury.md".to_string(),
        }
    }
    
    // ==================== DEPRECATED ====================
    
    fn fiat_gateway_integration(&self) -> Integration {
        Integration {
            id: "fiat-gateway".to_string(),
            name: "Fiat Gateway (DEPRECATED)".to_string(),
            description: "Банкови преводи - ИЗИСКВА БАНКОВ ЛИЦЕНЗ".to_string(),
            integration_type: IntegrationType::ExternalApi,
            status: IntegrationStatus::RequiresLicense,
            priority: IntegrationPriority::Deprecated,
            endpoints: vec![],
            config_fields: vec![],
            connection_steps: vec![
                "❌ Премахнато - изисква банков лиценз (1+ година, $100k+ капитал)".to_string(),
                "Използвай външни процесори (Stripe, Wise) и депозирай USDC".to_string(),
            ],
            example_config: serde_json::json!({}),
            testing_commands: vec![],
            last_check: None,
            error_message: Some("Fiat operations not supported - requires banking license".to_string()),
            documentation_url: "https://github.com/investor-os/docs/blob/main/deprecated/fiat.md".to_string(),
        }
    }
    
    fn fx_converter_integration(&self) -> Integration {
        Integration {
            id: "fx-converter".to_string(),
            name: "FX Converter (DEPRECATED)".to_string(),
            description: "Валутна конверсия - ИЗИСКВА FX ЛИЦЕНЗ".to_string(),
            integration_type: IntegrationType::ExternalApi,
            status: IntegrationStatus::RequiresLicense,
            priority: IntegrationPriority::Deprecated,
            endpoints: vec![],
            config_fields: vec![],
            connection_steps: vec![
                "❌ Премахнато - изисква FX лиценз".to_string(),
                "За multi-currency: използвай DEX (Uniswap) или CEX".to_string(),
            ],
            example_config: serde_json::json!({}),
            testing_commands: vec![],
            last_check: None,
            error_message: Some("FX operations not supported".to_string()),
            documentation_url: "https://github.com/investor-os/docs/blob/main/deprecated/fx.md".to_string(),
        }
    }
    
    // ==================== INFRASTRUCTURE ====================
    
    fn database_integration(&self) -> Integration {
        Integration {
            id: "database".to_string(),
            name: "PostgreSQL Database".to_string(),
            description: "Основна база данни".to_string(),
            integration_type: IntegrationType::Database,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::Critical,
            endpoints: vec![],
            config_fields: vec![
                ConfigField {
                    name: "DATABASE_URL".to_string(),
                    field_type: "secret".to_string(),
                    required: true,
                    description: "PostgreSQL connection string".to_string(),
                    current_value: Some("postgresql://localhost/investor_os".to_string()),
                },
            ],
            connection_steps: vec![
                "1. PostgreSQL трябва да е инсталиран".to_string(),
                "2. DATABASE_URL трябва да е конфигурирана".to_string(),
            ],
            example_config: serde_json::json!({
                "url": "postgresql://user:pass@localhost/investor_os"
            }),
            testing_commands: vec!["psql $DATABASE_URL -c 'SELECT 1'".to_string()],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/database.md".to_string(),
        }
    }
    
    fn redis_integration(&self) -> Integration {
        Integration {
            id: "redis".to_string(),
            name: "Redis Cache".to_string(),
            description: "Кеш и message queue".to_string(),
            integration_type: IntegrationType::MessageQueue,
            status: IntegrationStatus::Connected,
            priority: IntegrationPriority::Medium,
            endpoints: vec![],
            config_fields: vec![
                ConfigField {
                    name: "REDIS_URL".to_string(),
                    field_type: "string".to_string(),
                    required: true,
                    description: "Redis connection URL".to_string(),
                    current_value: Some("redis://localhost:6379".to_string()),
                },
            ],
            connection_steps: vec![
                "1. Redis трябва да е инсталиран и пуснат".to_string(),
                "2. REDIS_URL трябва да е конфигурирана".to_string(),
            ],
            example_config: serde_json::json!({
                "url": "redis://localhost:6379"
            }),
            testing_commands: vec!["redis-cli ping".to_string()],
            last_check: Some(Utc::now()),
            error_message: None,
            documentation_url: "https://github.com/investor-os/docs/blob/main/redis.md".to_string(),
        }
    }
}

impl Default for IntegrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
