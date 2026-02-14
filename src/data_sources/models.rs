//! Data Source Models
//!
//! Database models and DTOs for data source management

use chrono::{DateTime, Utc};
// Decimal replaced with f64 for sqlx compatibility
use serde::{Deserialize, Serialize};
// Note: FromRow removed due to jsonb complexity - manual mapping used
use uuid::Uuid;

use super::{AuthType, DataSourceCategory, DataSourceStatus, DatasetType, ScraperJobStatus, ScraperType, SourceType};

// ============================================
// DATABASE MODELS
// ============================================

#[derive(Debug, Clone)]
pub struct DataSourceRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub provider: String,
    pub category: String,
    pub source_type: String,
    pub base_url: Option<String>,
    pub api_version: Option<String>,
    pub documentation_url: Option<String>,
    pub auth_type: String,
    pub api_key_env_var: Option<String>,
    pub rate_limit_requests: Option<i32>,
    pub rate_limit_window: Option<String>,
    pub status: String,
    pub is_enabled: bool,
    pub priority: i32,
    pub used_for_training: bool,
    pub training_data_volume: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_tested_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct DataSourceEndpointRow {
    pub id: Uuid,
    pub source_id: Uuid,
    pub name: String,
    pub method: String,
    pub path: String,
    pub description: Option<String>,
    pub required_params: serde_json::Value,
    pub optional_params: serde_json::Value,
    pub response_schema: Option<serde_json::Value>,
    pub is_active: bool,
    pub avg_response_ms: Option<i32>,
    pub success_rate: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DataSourcePricingRow {
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

#[derive(Debug, Clone)]
pub struct ScraperJobRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_url: Option<String>,
    pub url_pattern: Option<String>,
    pub scraper_type: String,
    pub config: serde_json::Value,
    pub schedule: Option<String>,
    pub timezone: String,
    pub status: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_result: Option<serde_json::Value>,
    pub output_dataset: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct MlDatasetRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub dataset_type: String,
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

// ============================================
// API REQUEST/RESPONSE DTOs
// ============================================

/// List data sources request
#[derive(Debug, Clone, Deserialize)]
pub struct ListDataSourcesRequest {
    pub category: Option<DataSourceCategory>,
    pub source_type: Option<SourceType>,
    pub status: Option<DataSourceStatus>,
    pub enabled_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List data sources response
#[derive(Debug, Clone, Serialize)]
pub struct ListDataSourcesResponse {
    pub sources: Vec<DataSourceSummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Data source summary (for lists)
#[derive(Debug, Clone, Serialize)]
pub struct DataSourceSummary {
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub category: DataSourceCategory,
    pub source_type: SourceType,
    pub status: DataSourceStatus,
    pub is_enabled: bool,
    pub rate_limit: Option<String>,
}

/// Data source detail response
#[derive(Debug, Clone, Serialize)]
pub struct DataSourceDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub provider: String,
    pub category: DataSourceCategory,
    pub source_type: SourceType,
    pub base_url: Option<String>,
    pub documentation_url: Option<String>,
    pub auth_type: AuthType,
    pub rate_limit: Option<RateLimitInfo>,
    pub status: DataSourceStatus,
    pub is_enabled: bool,
    pub priority: i32,
    pub used_for_training: bool,
    pub endpoints: Vec<EndpointSummary>,
    pub pricing: Vec<PricingTierSummary>,
    pub last_tested_at: Option<DateTime<Utc>>,
}

/// Rate limit info
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitInfo {
    pub requests: i32,
    pub window: String,
}

/// Endpoint summary
#[derive(Debug, Clone, Serialize)]
pub struct EndpointSummary {
    pub id: Uuid,
    pub name: String,
    pub method: String,
    pub path: String,
    pub description: String,
    pub is_active: bool,
}

/// Pricing tier summary
#[derive(Debug, Clone, Serialize)]
pub struct PricingTierSummary {
    pub tier_name: String,
    pub tier_level: i32,
    pub price_monthly_usd: Option<f64>,
    pub price_yearly_usd: Option<f64>,
    pub requests_per_day: Option<i32>,
    pub features: Vec<String>,
}

/// Test connection response
#[derive(Debug, Clone, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub response_time_ms: Option<i64>,
    pub timestamp: DateTime<Utc>,
}

/// Create scraper job request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateScraperJobRequest {
    pub name: String,
    pub description: Option<String>,
    pub target_url: Option<String>,
    pub url_pattern: Option<String>,
    pub scraper_type: ScraperType,
    pub config: serde_json::Value,
    pub schedule: Option<String>,
    pub timezone: Option<String>,
    pub output_dataset: Option<String>,
}

/// Scraper job response
#[derive(Debug, Clone, Serialize)]
pub struct ScraperJobResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_url: Option<String>,
    pub scraper_type: ScraperType,
    pub status: ScraperJobStatus,
    pub schedule: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Create ML dataset request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMlDatasetRequest {
    pub name: String,
    pub description: Option<String>,
    pub dataset_type: DatasetType,
    pub source_ids: Vec<Uuid>,
    pub date_range_start: Option<DateTime<Utc>>,
    pub date_range_end: Option<DateTime<Utc>>,
}

/// ML dataset response
#[derive(Debug, Clone, Serialize)]
pub struct MlDatasetResponse {
    pub id: Uuid,
    pub name: String,
    pub dataset_type: DatasetType,
    pub record_count: i64,
    pub size_bytes: i64,
    pub quality_score: Option<f64>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
}

/// Pricing catalog response
#[derive(Debug, Clone, Serialize)]
pub struct PricingCatalogResponse {
    pub categories: Vec<PricingCategory>,
}

/// Pricing category
#[derive(Debug, Clone, Serialize)]
pub struct PricingCategory {
    pub category: DataSourceCategory,
    pub sources: Vec<SourcePricingInfo>,
}

/// Source pricing info
#[derive(Debug, Clone, Serialize)]
pub struct SourcePricingInfo {
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub source_type: SourceType,
    pub tiers: Vec<PricingTierDetail>,
}

/// Pricing tier detail
#[derive(Debug, Clone, Serialize)]
pub struct PricingTierDetail {
    pub tier_name: String,
    pub tier_level: i32,
    pub price_monthly_usd: Option<f64>,
    pub price_yearly_usd: Option<f64>,
    pub requests_per_day: Option<i32>,
    pub requests_per_minute: Option<i32>,
    pub features: Vec<String>,
}

/// Cost estimate request
#[derive(Debug, Clone, Deserialize)]
pub struct CostEstimateRequest {
    pub source_ids: Vec<Uuid>,
    pub estimated_requests_per_day: i32,
}

/// Cost estimate response
#[derive(Debug, Clone, Serialize)]
pub struct CostEstimateResponse {
    pub estimates: Vec<SourceCostEstimate>,
    pub total_monthly_usd: f64,
    pub total_yearly_usd: f64,
}

/// Source cost estimate
#[derive(Debug, Clone, Serialize)]
pub struct SourceCostEstimate {
    pub source_id: Uuid,
    pub source_name: String,
    pub recommended_tier: String,
    pub monthly_cost_usd: f64,
    pub yearly_cost_usd: f64,
}

// ============================================
// CONVERSION TRAITS
// ============================================

impl From<DataSourceRow> for super::DataSource {
    fn from(row: DataSourceRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description.unwrap_or_default(),
            provider: row.provider,
            category: super::parse_category(&row.category),
            source_type: super::parse_source_type(&row.source_type),
            base_url: row.base_url,
            api_version: row.api_version,
            documentation_url: row.documentation_url,
            auth_type: super::parse_auth_type(&row.auth_type),
            api_key_env_var: row.api_key_env_var,
            rate_limit_requests: row.rate_limit_requests,
            rate_limit_window: row.rate_limit_window,
            status: super::parse_status(&row.status),
            is_enabled: row.is_enabled,
            priority: row.priority,
            used_for_training: row.used_for_training,
            training_data_volume: row.training_data_volume,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_tested_at: row.last_tested_at,
            last_error: row.last_error,
            config: row.config,
        }
    }
}

impl From<DataSourceRow> for DataSourceSummary {
    fn from(row: DataSourceRow) -> Self {
        let rate_limit = match (row.rate_limit_requests, row.rate_limit_window.as_ref()) {
            (Some(reqs), Some(window)) => Some(format!("{}/{} requests", reqs, window)),
            _ => None,
        };
        
        Self {
            id: row.id,
            name: row.name,
            provider: row.provider,
            category: super::parse_category(&row.category),
            source_type: super::parse_source_type(&row.source_type),
            status: super::parse_status(&row.status),
            is_enabled: row.is_enabled,
            rate_limit,
        }
    }
}

impl From<DataSourceEndpointRow> for super::DataSourceEndpoint {
    fn from(row: DataSourceEndpointRow) -> Self {
        Self {
            id: row.id,
            source_id: row.source_id,
            name: row.name,
            method: row.method,
            path: row.path,
            description: row.description.unwrap_or_default(),
            required_params: row.required_params,
            optional_params: row.optional_params,
            response_schema: row.response_schema,
            is_active: row.is_active,
            avg_response_ms: row.avg_response_ms,
            success_rate: row.success_rate,
            created_at: row.created_at,
        }
    }
}

impl From<DataSourcePricingRow> for super::DataSourcePricing {
    fn from(row: DataSourcePricingRow) -> Self {
        Self {
            id: row.id,
            source_id: row.source_id,
            tier_name: row.tier_name,
            tier_level: row.tier_level,
            price_monthly_usd: row.price_monthly_usd,
            price_yearly_usd: row.price_yearly_usd,
            currency: row.currency,
            requests_per_day: row.requests_per_day,
            requests_per_minute: row.requests_per_minute,
            data_points_per_request: row.data_points_per_request,
            historical_data_years: row.historical_data_years,
            features: row.features,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<ScraperJobRow> for super::ScraperJob {
    fn from(row: ScraperJobRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            target_url: row.target_url,
            url_pattern: row.url_pattern,
            scraper_type: super::parse_scraper_type(&row.scraper_type),
            config: row.config,
            schedule: row.schedule,
            timezone: row.timezone,
            status: super::parse_scraper_job_status(&row.status),
            last_run_at: row.last_run_at,
            next_run_at: row.next_run_at,
            last_result: row.last_result,
            output_dataset: row.output_dataset,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<MlDatasetRow> for super::MlDataset {
    fn from(row: MlDatasetRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            dataset_type: super::parse_dataset_type(&row.dataset_type),
            source_ids: row.source_ids,
            scraper_job_ids: row.scraper_job_ids,
            record_count: row.record_count,
            size_bytes: row.size_bytes,
            time_range_start: row.time_range_start,
            time_range_end: row.time_range_end,
            quality_score: row.quality_score,
            validation_errors: row.validation_errors,
            storage_path: row.storage_path,
            format: row.format,
            version: row.version,
            parent_version_id: row.parent_version_id,
            used_in_models: row.used_in_models,
            created_at: row.created_at,
        }
    }
}

// Note: Parsing helpers are in mod.rs
