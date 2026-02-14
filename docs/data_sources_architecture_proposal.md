# Архитектура за Data Sources Management System

## Обобщение

Система за управление на безплатни и платени източници на данни с интегриран web scraper и ML training pipeline.

---

## 🎯 Цели

1. **Centralized Data Source Catalog** - Всички източници на едно място
2. **Free Tier First** - Безплатни източници за начално обучение
3. **Premium Upgrade Path** - Платени за production/дообучение
4. **Web Scraping** - Firecrawl за сайтове без API
5. **ML Training Pipeline** - Feature store, версиониране, качество

---

## 🏗️ Архитектура

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         ADMIN PANEL (React/Web)                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ Free Sources │  │Paid Sources  │  │ Web Scraper  │  │ML Pipeline   │    │
│  │   Catalog    │  │  Pricing     │  │   Control    │  │  Dashboard   │    │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         API LAYER (Axum/Rust)                                │
├─────────────────────────────────────────────────────────────────────────────┤
│  /api/admin/data-sources          /api/admin/data-sources/pricing           │
│  /api/admin/data-sources/{id}/test  /api/admin/scraper/jobs                 │
│  /api/admin/ml/features           /api/admin/ml/datasets                    │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      DATA SOURCE MANAGER                                     │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│  │  Free Sources   │  │  Paid Sources   │  │ Scraped Sources │             │
│  │   Connector     │  │   Connector     │  │   Connector     │             │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘             │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    ▼                  ▼                  ▼
┌──────────────────────┐  ┌──────────────────────┐  ┌──────────────────────┐
│   EXTERNAL APIs      │  │    WEB SCRAPER       │  │   ML PIPELINE        │
│  (Free & Premium)    │  │   (Firecrawl)        │  │  (Feature Store)     │
├──────────────────────┤  ├──────────────────────┤  ├──────────────────────┤
│ • Alpha Vantage      │  │ • Scrapy (Python)    │  │ • Raw Data Storage   │
│ • Yahoo Finance      │  │ • Firecrawl (API)    │  │ • Feature Engineering│
│ • FRED               │  │ • Playwright         │  │ • Training Datasets  │
│ • World Bank         │  │ • Crawlee            │  │ • Data Versioning    │
│ • Eurostat           │  │                      │  │ • Quality Metrics    │
└──────────────────────┘  └──────────────────────┘  └──────────────────────┘
```

---

## 📊 Database Schema

### Таблица: `data_sources`

```sql
CREATE TABLE data_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Основна информация
    name VARCHAR(255) NOT NULL,
    description TEXT,
    provider VARCHAR(100),           -- "Alpha Vantage", "World Bank", etc.
    category VARCHAR(50),            -- "market_data", "economic", "news", etc.
    
    -- Тип на източника
    source_type VARCHAR(50),         -- "free", "freemium", "paid", "scraped"
    
    -- API информация
    base_url VARCHAR(500),
    api_version VARCHAR(20),
    documentation_url VARCHAR(500),
    
    -- Authentication
    auth_type VARCHAR(50),           -- "none", "api_key", "oauth2", "bearer"
    api_key_env_var VARCHAR(100),    -- env variable name for API key
    
    -- Rate Limits (за безплатни)
    rate_limit_requests INTEGER,     -- напр. 25
    rate_limit_window VARCHAR(20),   -- "day", "minute", "hour"
    
    -- Status
    status VARCHAR(50) DEFAULT 'inactive', -- "active", "inactive", "error"
    is_enabled BOOLEAN DEFAULT false,
    priority INTEGER DEFAULT 100,    -- за подреждане
    
    -- ML Usage
    used_for_training BOOLEAN DEFAULT false,
    training_data_volume BIGINT,     -- bytes или records
    
    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_tested_at TIMESTAMPTZ,
    last_error TEXT,
    
    -- Configuration (JSON)
    config JSONB DEFAULT '{}'
);
```

### Таблица: `data_source_endpoints`

```sql
CREATE TABLE data_source_endpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID REFERENCES data_sources(id) ON DELETE CASCADE,
    
    name VARCHAR(100) NOT NULL,      -- "Get Stock Quote", "Search News"
    method VARCHAR(10) NOT NULL,     -- GET, POST
    path VARCHAR(255) NOT NULL,      -- "/query", "/v1/stocks"
    description TEXT,
    
    -- Parameters
    required_params JSONB,           -- ["symbol", "interval"]
    optional_params JSONB,           -- {"outputsize": "compact"}
    
    -- Response
    response_schema JSONB,           -- JSON schema за валидация
    
    -- Usage
    is_active BOOLEAN DEFAULT true,
    avg_response_ms INTEGER,
    success_rate DECIMAL(5,2),       -- percentage
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Таблица: `data_source_pricing`

```sql
CREATE TABLE data_source_pricing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID REFERENCES data_sources(id) ON DELETE CASCADE,
    
    -- Tier информация
    tier_name VARCHAR(100),          -- "Free", "Basic", "Pro", "Enterprise"
    tier_level INTEGER DEFAULT 1,    -- 1=Free, 2=Basic, etc.
    
    -- Цена
    price_monthly_usd DECIMAL(10,2),
    price_yearly_usd DECIMAL(10,2),
    currency VARCHAR(3) DEFAULT 'USD',
    
    -- Limits
    requests_per_day INTEGER,
    requests_per_minute INTEGER,
    data_points_per_request INTEGER,
    historical_data_years INTEGER,
    
    -- Features
    features JSONB,                  -- ["realtime", "websocket", "support"]
    
    -- Notes
    notes TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Таблица: `scraper_jobs`

```sql
CREATE TABLE scraper_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    name VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- Target
    target_url VARCHAR(500),
    url_pattern VARCHAR(255),        -- regex pattern за множество URL-и
    
    -- Scraping config
    scraper_type VARCHAR(50),        -- "firecrawl", "scrapy", "playwright"
    config JSONB,                    -- специфична конфигурация
    
    -- Schedule
    schedule VARCHAR(100),           -- cron expression
    timezone VARCHAR(50) DEFAULT 'UTC',
    
    -- Status
    status VARCHAR(50) DEFAULT 'pending', -- "pending", "running", "completed", "failed"
    
    -- Results
    last_run_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ,
    last_result JSONB,               -- summary от последното пускане
    
    -- ML Integration
    output_dataset VARCHAR(100),     -- къде се записват данните
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Таблица: `ml_datasets`

```sql
CREATE TABLE ml_datasets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    name VARCHAR(255) NOT NULL,
    description TEXT,
    dataset_type VARCHAR(50),        -- "training", "validation", "test"
    
    -- Source information
    source_ids UUID[],               -- от кои източници идват данните
    scraper_job_ids UUID[],          -- от кои scraper jobs
    
    -- Volume
    record_count BIGINT,
    size_bytes BIGINT,
    time_range_start TIMESTAMPTZ,
    time_range_end TIMESTAMPTZ,
    
    -- Quality
    quality_score DECIMAL(3,2),      -- 0.00 - 1.00
    validation_errors JSONB,
    
    -- Storage
    storage_path VARCHAR(500),       -- S3, local path, etc.
    format VARCHAR(20),              -- "parquet", "csv", "json"
    
    -- Versioning
    version INTEGER DEFAULT 1,
    parent_version_id UUID REFERENCES ml_datasets(id),
    
    -- Usage
    used_in_models VARCHAR[],        -- списък с модели
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Таблица: `data_source_usage_logs`

```sql
CREATE TABLE data_source_usage_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID REFERENCES data_sources(id),
    endpoint_id UUID REFERENCES data_source_endpoints(id),
    
    request_time TIMESTAMPTZ DEFAULT NOW(),
    response_time_ms INTEGER,
    
    request_params JSONB,
    response_status INTEGER,
    
    records_fetched INTEGER,
    error_message TEXT,
    
    cost_usd DECIMAL(10,6)           -- за платени източници
);
```

---

## 🔌 API Endpoints

### Data Sources Management

```
GET    /api/admin/data-sources                    # Списък всички източници
GET    /api/admin/data-sources/free               # Само безплатни
GET    /api/admin/data-sources/paid               # Само платени
GET    /api/admin/data-sources/category/:category # По категория

GET    /api/admin/data-sources/:id                # Детайли
POST   /api/admin/data-sources/:id/test           # Тест на връзката
PUT    /api/admin/data-sources/:id/enable         # Активиране
PUT    /api/admin/data-sources/:id/disable        # Деактивиране

GET    /api/admin/data-sources/:id/endpoints      # Endpoint-и
POST   /api/admin/data-sources/:id/endpoints      # Добавяне endpoint
```

### Pricing Catalog

```
GET    /api/admin/data-sources/pricing            # Цени на всички платени
GET    /api/admin/data-sources/pricing/compare    # Сравнение между доставчици
GET    /api/admin/data-sources/cost-estimate      # Оценка на месечен разход

# Примери за данни:
# {
#   "source": "Bloomberg Terminal",
#   "price_yearly": 24000,
#   "features": ["realtime", "news", "analytics"]
# }
```

### Web Scraper

```
GET    /api/admin/scraper/jobs                    # Списък jobs
POST   /api/admin/scraper/jobs                    # Създаване job
GET    /api/admin/scraper/jobs/:id                # Детайли
PUT    /api/admin/scraper/jobs/:id/run            # Стартиране
PUT    /api/admin/scraper/jobs/:id/stop           # Спиране
DELETE /api/admin/scraper/jobs/:id                # Изтриване

GET    /api/admin/scraper/jobs/:id/logs           # Логове
GET    /api/admin/scraper/templates               # Готови шаблони
POST   /api/admin/scraper/extract                 # Еднократно извличане
```

### ML Training Pipeline

```
GET    /api/admin/ml/datasets                     # Датасети
POST   /api/admin/ml/datasets                     # Създаване от източници
GET    /api/admin/ml/datasets/:id                 # Детайли
GET    /api/admin/ml/datasets/:id/download        # Изтегляне

GET    /api/admin/ml/features                     # Feature store
POST   /api/admin/ml/features/compute             # Изчисляване

GET    /api/admin/ml/training-jobs                # Обучаващи jobs
POST   /api/admin/ml/training-jobs                # Стартиране обучение
```

---

## 🛠️ Компоненти

### 1. Data Source Connector Trait

```rust
/// Trait за всички data source connectors
#[async_trait]
pub trait DataSourceConnector: Send + Sync {
    /// Име на конектора
    fn name(&self) -> &str;
    
    /// Тип на източника
    fn source_type(&self) -> SourceType;
    
    /// Проверка на връзката
    async fn test_connection(&self) -> Result<ConnectionTest, DataSourceError>;
    
    /// Извличане на данни
    async fn fetch(&self, request: DataRequest) -> Result<DataResponse, DataSourceError>;
    
    /// Rate limit статус
    async fn rate_limit_status(&self) -> Result<RateLimitStatus, DataSourceError>;
}

/// Фабрика за конектори
pub struct ConnectorFactory;

impl ConnectorFactory {
    pub fn create(source: &DataSource) -> Box<dyn DataSourceConnector> {
        match source.provider.as_str() {
            "alpha_vantage" => Box::new(AlphaVantageConnector::new(source.config.clone())),
            "yahoo_finance" => Box::new(YahooFinanceConnector::new()),
            "fred" => Box::new(FredConnector::new(source.config.clone())),
            "world_bank" => Box::new(WorldBankConnector::new()),
            "eurostat" => Box::new(EurostatConnector::new()),
            "firecrawl" => Box::new(FirecrawlConnector::new(source.config.clone())),
            _ => Box::new(GenericRestConnector::new(source.clone())),
        }
    }
}
```

### 2. Web Scraper Service

```rust
/// Управление на web scraping
pub struct WebScraperService {
    firecrawl_client: Option<FirecrawlClient>,
    scrapy_runner: Option<ScrapyRunner>,
}

impl WebScraperService {
    /// Създаване на scraper job
    pub async fn create_job(&self, config: ScraperConfig) -> Result<ScraperJob, ScraperError> {
        match config.scraper_type {
            ScraperType::Firecrawl => self.create_firecrawl_job(config).await,
            ScraperType::Scrapy => self.create_scrapy_job(config).await,
            ScraperType::Playwright => self.create_playwright_job(config).await,
        }
    }
    
    /// Еднократно извличане
    pub async fn scrape_once(&self, url: &str, options: ScrapeOptions) -> Result<ScrapeResult, ScraperError> {
        // Използва Firecrawl API или self-hosted
    }
}
```

### 3. ML Data Pipeline

```rust
/// Pipeline за подготовка на ML данни
pub struct MlDataPipeline {
    source_manager: Arc<DataSourceManager>,
    feature_store: Arc<FeatureStore>,
    storage: Arc<dyn DatasetStorage>,
}

impl MlDataPipeline {
    /// Създаване на training dataset от множество източници
    pub async fn create_training_dataset(
        &self,
        config: TrainingDatasetConfig,
    ) -> Result<MlDataset, PipelineError> {
        // 1. Извличане от всички конфигурирани източници
        // 2. Обединяване и нормализация
        // 3. Feature engineering
        // 4. Quality validation
        // 5. Запазване с версиониране
    }
    
    /// Автоматично обновяване на dataset
    pub async fn refresh_dataset(&self, dataset_id: Uuid) -> Result<MlDataset, PipelineError> {
        // Инкрементално обновяване
    }
}
```

---

## 📈 Платени Източници - Pricing Catalog

### Tier 1: Entry Level (Free - $100/mo)

| Източник | Цена | Limits | Best For |
|----------|------|--------|----------|
| **Alpha Vantage** | Free / $49/mo | 25/day free, 75/min paid | Начално обучение |
| **IEX Cloud** | Free / $19/mo | 50K msg/mo free | US акции |
| **EODHD** | Free / $19/mo | 20/day free, 100K/day paid | Глобални пазари |
| **FRED** | Free | Неограничено | Икономически данни |
| **World Bank** | Free | Неограничено | Макро данни |

### Tier 2: Professional ($100 - $1000/mo)

| Източник | Цена | Features |
|----------|------|----------|
| **Polygon.io** | $199/mo | Real-time US, websocket |
| **Tiingo** | $348/mo | 500K calls/day, fundamentals |
| **Financial Modeling Prep** | $228/mo | Full fundamentals, screening |
| **Quandl/NASDAQ Data Link** | $150/mo | Алтернативни данни |
| **NewsAPI.ai** | $449/mo | 1M calls/mo, анализ |

### Tier 3: Enterprise ($1000+/mo)

| Източник | Цена | Features |
|----------|------|----------|
| **Bloomberg API** | $2000+/mo | Institutional-grade |
| **Refinitiv Eikon** | $3000+/mo | Global markets, news |
| **FactSet** | $12000/yr | Analytics, screening |
| **S&P Capital IQ** | $25000+/yr | Full fundamentals |
| ** RavenPack** | По договаряне | News analytics |
| **Second Measure** | По договаряне | Consumer transaction data |
| **Earnest Research** | По договаряне | Consumer analytics |
| **Thinknum** | По договаряне | Alternative data |

### Tier 4: Premium Terminal Access

| Източник | Цена | Features |
|----------|------|----------|
| **Bloomberg Terminal** | $24000/yr | Full terminal |
| **Refinitiv Eikon** | $3600-22000/yr | В зависимост от пакет |
| **FactSet Workstation** | $12000+/yr | Full workstation |

---

## 🕷️ Web Scraper Опции

### Опция 1: Firecrawl (Препоръчително)

**Предимства:**
- ✅ Open source, self-hosted опция
- ✅ LLM-ready output (markdown, JSON)
- ✅ JavaScript rendering
- ✅ Automatic retries, proxies
- ✅ PDF, DOCX parsing
- ✅ Actions (click, scroll, input)

**Цена:**
- Self-hosted: Безплатно (само инфраструктура)
- Cloud: $0.001-0.01/page

**Интеграция:**
```rust
pub struct FirecrawlClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl FirecrawlClient {
    pub async fn scrape(&self, url: &str) -> Result<ScrapeResult> {
        // REST API call
    }
    
    pub async fn crawl(&self, url: &str, options: CrawlOptions) -> Result<CrawlResult> {
        // Crawl цял сайт
    }
}
```

### Опция 2: Scrapy (Python)

**Предимства:**
- ✅ Зрял, battle-tested
- ✅ 2400+ requests/min
- ✅ Extensible middleware
- ✅ Built-in export formats

**Интеграция:**
```rust
// Rust wrapper around Python Scrapy
pub struct ScrapyRunner {
    python_env: PythonEnv,
}

impl ScrapyRunner {
    pub async fn run_spider(&self, spider_config: SpiderConfig) -> Result<SpiderResult> {
        // Извиква Python scrapy process
    }
}
```

### Опция 3: Playwright/Crawlee

**Предимства:**
- ✅ Modern browser automation
- ✅ Handles SPAs
- ✅ Screenshots, PDFs
- ✅ TypeScript/JavaScript

---

## 🎓 ML Training Pipeline Flow

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        ML TRAINING PIPELINE                                │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  PHASE 1: DATA COLLECTION                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │ Free APIs   │  │ Scraped     │  │ Internal    │  │ File Upload │       │
│  │ (Training)  │  │ (Training)  │  │ (Features)  │  │ (Import)    │       │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘       │
│         └─────────────────┴─────────────────┴─────────────────┘            │
│                                    │                                       │
│                                    ▼                                       │
│  PHASE 2: DATA PROCESSING                                                  │
│  ┌─────────────────────────────────────────────────────────────┐          │
│  │ • Validation & Cleaning                                     │          │
│  │ • Normalization (tz, currency, units)                      │          │
│  │ • Deduplication                                             │          │
│  │ • Outlier detection                                         │          │
│  └─────────────────────────────────────────────────────────────┘          │
│                                    │                                       │
│                                    ▼                                       │
│  PHASE 3: FEATURE ENGINEERING                                              │
│  ┌─────────────────────────────────────────────────────────────┐          │
│  │ • Technical Indicators (RSI, MACD, etc.)                   │          │
│  │ • Fundamental Ratios (P/E, ROE, etc.)                      │          │
│  │ • Sentiment Scores                                          │          │
│  │ • Market Microstructure (spread, depth)                    │          │
│  │ • Lag Features                                              │          │
│  └─────────────────────────────────────────────────────────────┘          │
│                                    │                                       │
│                                    ▼                                       │
│  PHASE 4: STORAGE & VERSIONING                                             │
│  ┌─────────────────────────────────────────────────────────────┐          │
│  │ • Feature Store (Feast)                                     │          │
│  │ • Dataset Versioning (DVC)                                  │          │
│  │ • Metadata Tracking (MLflow)                                │          │
│  │ • Quality Metrics                                           │          │
│  └─────────────────────────────────────────────────────────────┘          │
│                                    │                                       │
│                                    ▼                                       │
│  PHASE 5: TRAINING                                                         │
│  ┌─────────────────────────────────────────────────────────────┐          │
│  │ • Model Selection                                           │          │
│  │ • Hyperparameter Tuning                                     │          │
│  │ • Cross-Validation                                          │          │
│  │ • Backtesting                                               │          │
│  └─────────────────────────────────────────────────────────────┘          │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## 🚀 Implementation Plan

### Phase 1: Core Infrastructure (1-2 седмици)

1. **Database Migration**
   - Създаване на таблиците
   - Seed с безплатни източници

2. **API Layer**
   - CRUD endpoints за data sources
   - Test connection functionality
   - Pricing catalog endpoints

3. **Admin Panel UI**
   - Списък с източници
   - Детайли и конфигурация
   - Pricing таблица

### Phase 2: Connectors (2-3 седмици)

1. **Free Sources Connectors**
   - Alpha Vantage
   - Yahoo Finance
   - FRED
   - World Bank
   - Eurostat

2. **Connector Framework**
   - Retry logic
   - Rate limiting
   - Caching layer

### Phase 3: Web Scraper (2 седмици)

1. **Firecrawl Integration**
   - Self-hosted setup
   - API wrapper
   - Job management

2. **Scraper Jobs**
   - Schedule management
   - Result storage
   - Error handling

### Phase 4: ML Pipeline (3-4 седмици)

1. **Feature Store**
   - Feast setup
   - Feature definitions
   - Online/Offline stores

2. **Dataset Management**
   - Versioning
   - Quality checks
   - Export formats

---

## 💡 Препоръки

### За Безплатни Източници:
1. **Yahoo Finance** - за исторически данни
2. **FRED** - за икономически индикатори
3. **Alpha Vantage** - за fundamentals
4. **World Bank** - за макро данни
5. **Firecrawl** - за сайтове без API

### За Обучение:
1. Започни с безплатни за initial training
2. Събери поне 5-10 години исторически данни
3. Focus на качество, не количество
4. Version control на datasets

### За Production:
1. **Polygon.io** или **IEX Cloud** за real-time
2. **Tiingo** или **FMP** за fundamentals
3. **NewsAPI.ai** за sentiment
4. **Firecrawl** за proprietary scraping

---

## 📚 Ресурси

- [Firecrawl Docs](https://docs.firecrawl.dev/)
- [Feast Feature Store](https://docs.feast.dev/)
- [Alpha Vantage Docs](https://www.alphavantage.co/documentation/)
- [FRED API](https://fred.stlouisfed.org/docs/api/fred/)
- [World Bank API](https://datahelpdesk.worldbank.org/knowledgebase/articles/889386-api-faqs)
