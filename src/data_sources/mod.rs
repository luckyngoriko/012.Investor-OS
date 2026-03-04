//! Data Sources Management Module
//!
//! Manages free and paid data sources for the trading system.
//! Provides:
//! - Free data sources for initial training
//! - Paid source pricing catalog
//! - Web scraping integration (Firecrawl)
//! - ML training pipeline integration

use chrono::{DateTime, Utc};
// Decimal replaced with f64 for sqlx compatibility
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod connectors;
pub mod models;
pub mod scraper;
pub mod service;

pub use connectors::{ConnectorFactory, DataSourceConnector};
pub use models::*;
pub use scraper::{FirecrawlClient, ScraperService};
pub use service::DataSourceService;

/// Data source category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceCategory {
    MarketData,
    Economic,
    News,
    Alternative,
    Geospatial,
    Commodities,
    Crypto,
    Forex,
}

impl std::fmt::Display for DataSourceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataSourceCategory::MarketData => write!(f, "market_data"),
            DataSourceCategory::Economic => write!(f, "economic"),
            DataSourceCategory::News => write!(f, "news"),
            DataSourceCategory::Alternative => write!(f, "alternative"),
            DataSourceCategory::Geospatial => write!(f, "geospatial"),
            DataSourceCategory::Commodities => write!(f, "commodities"),
            DataSourceCategory::Crypto => write!(f, "crypto"),
            DataSourceCategory::Forex => write!(f, "forex"),
        }
    }
}

/// Source type (free, freemium, paid, scraped)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Free,
    Freemium,
    Paid,
    Scraped,
    Government,
}

/// Authentication type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    None,
    ApiKey,
    OAuth2,
    Bearer,
    Basic,
}

/// Data source status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceStatus {
    Active,
    Inactive,
    Error,
}

/// Main data source entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub provider: String,
    pub category: DataSourceCategory,
    pub source_type: SourceType,

    // API info
    pub base_url: Option<String>,
    pub api_version: Option<String>,
    pub documentation_url: Option<String>,

    // Auth
    pub auth_type: AuthType,
    pub api_key_env_var: Option<String>,

    // Rate limits
    pub rate_limit_requests: Option<i32>,
    pub rate_limit_window: Option<String>,

    // Status
    pub status: DataSourceStatus,
    pub is_enabled: bool,
    pub priority: i32,

    // ML
    pub used_for_training: bool,
    pub training_data_volume: i64,

    // Metadata
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,

    // Config
    pub config: serde_json::Value,
}

/// Data source endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceEndpoint {
    pub id: Uuid,
    pub source_id: Uuid,
    pub name: String,
    pub method: String,
    pub path: String,
    pub description: String,
    pub required_params: serde_json::Value,
    pub optional_params: serde_json::Value,
    pub response_schema: Option<serde_json::Value>,
    pub is_active: bool,
    pub avg_response_ms: Option<i32>,
    pub success_rate: Option<f64>,
    pub created_at: DateTime<Utc>,
}

/// Pricing tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourcePricing {
    pub id: Uuid,
    pub source_id: Uuid,
    pub tier_name: String,
    pub tier_level: i32,
    pub price_monthly_usd: Option<f64>,
    pub price_yearly_usd: Option<f64>,
    pub currency: String,
    pub requests_per_day: Option<i32>,
    pub requests_per_minute: Option<i32>,
    pub data_points_per_request: Option<i32>,
    pub historical_data_years: Option<i32>,
    pub features: serde_json::Value,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Scraper job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperJob {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_url: Option<String>,
    pub url_pattern: Option<String>,
    pub scraper_type: ScraperType,
    pub config: serde_json::Value,
    pub schedule: Option<String>,
    pub timezone: String,
    pub status: ScraperJobStatus,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_result: Option<serde_json::Value>,
    pub output_dataset: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Scraper type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScraperType {
    Firecrawl,
    Scrapy,
    Playwright,
    Custom,
}

/// Scraper job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScraperJobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Paused,
}

/// ML Dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlDataset {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub dataset_type: DatasetType,
    pub source_ids: Vec<Uuid>,
    pub scraper_job_ids: Vec<Uuid>,
    pub record_count: i64,
    pub size_bytes: i64,
    pub time_range_start: Option<DateTime<Utc>>,
    pub time_range_end: Option<DateTime<Utc>>,
    pub quality_score: Option<f64>,
    pub validation_errors: serde_json::Value,
    pub storage_path: Option<String>,
    pub format: Option<String>,
    pub version: i32,
    pub parent_version_id: Option<Uuid>,
    pub used_in_models: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Dataset type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetType {
    Training,
    Validation,
    Test,
}

/// Connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTest {
    pub success: bool,
    pub response_time_ms: i64,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Data fetch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFetchRequest {
    pub endpoint_id: Uuid,
    pub params: serde_json::Value,
}

/// Data fetch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFetchResponse {
    pub success: bool,
    pub data: serde_json::Value,
    pub records_count: usize,
    pub fetch_time_ms: i64,
    pub error: Option<String>,
}

// ============================================
// PARSING HELPERS (used by models and service)
// ============================================

/// Parse category from string
pub fn parse_category(s: &str) -> DataSourceCategory {
    match s {
        "market_data" => DataSourceCategory::MarketData,
        "economic" => DataSourceCategory::Economic,
        "news" => DataSourceCategory::News,
        "alternative" => DataSourceCategory::Alternative,
        "geospatial" => DataSourceCategory::Geospatial,
        "commodities" => DataSourceCategory::Commodities,
        "crypto" => DataSourceCategory::Crypto,
        "forex" => DataSourceCategory::Forex,
        _ => DataSourceCategory::MarketData,
    }
}

/// Parse source type from string
pub fn parse_source_type(s: &str) -> SourceType {
    match s {
        "free" => SourceType::Free,
        "freemium" => SourceType::Freemium,
        "paid" => SourceType::Paid,
        "scraped" => SourceType::Scraped,
        "government" => SourceType::Government,
        _ => SourceType::Free,
    }
}

/// Parse auth type from string
pub fn parse_auth_type(s: &str) -> AuthType {
    match s {
        "none" => AuthType::None,
        "api_key" => AuthType::ApiKey,
        "oauth2" => AuthType::OAuth2,
        "bearer" => AuthType::Bearer,
        "basic" => AuthType::Basic,
        _ => AuthType::None,
    }
}

/// Parse status from string
pub fn parse_status(s: &str) -> DataSourceStatus {
    match s {
        "active" => DataSourceStatus::Active,
        "inactive" => DataSourceStatus::Inactive,
        "error" => DataSourceStatus::Error,
        _ => DataSourceStatus::Inactive,
    }
}

/// Parse scraper type from string
pub fn parse_scraper_type(s: &str) -> ScraperType {
    match s {
        "firecrawl" => ScraperType::Firecrawl,
        "scrapy" => ScraperType::Scrapy,
        "playwright" => ScraperType::Playwright,
        "custom" => ScraperType::Custom,
        _ => ScraperType::Firecrawl,
    }
}

/// Parse scraper job status from string
pub fn parse_scraper_job_status(s: &str) -> ScraperJobStatus {
    match s {
        "pending" => ScraperJobStatus::Pending,
        "running" => ScraperJobStatus::Running,
        "completed" => ScraperJobStatus::Completed,
        "failed" => ScraperJobStatus::Failed,
        "paused" => ScraperJobStatus::Paused,
        _ => ScraperJobStatus::Pending,
    }
}

/// Parse dataset type from string
pub fn parse_dataset_type(s: &str) -> DatasetType {
    match s {
        "training" => DatasetType::Training,
        "validation" => DatasetType::Validation,
        "test" => DatasetType::Test,
        _ => DatasetType::Training,
    }
}
