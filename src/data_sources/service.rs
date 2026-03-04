//! Data Source Service
//!
//! Business logic for managing data sources

use anyhow::Context;
use chrono::Utc;
use sqlx::{postgres::PgRow, PgPool, Postgres, QueryBuilder, Row};
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::data_sources::models::*;
use crate::data_sources::ConnectionTest;
use crate::data_sources::ConnectorFactory;

/// Data source service
pub struct DataSourceService {
    pool: Arc<PgPool>,
}

impl DataSourceService {
    /// Create new service
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// List all data sources with filters
    pub async fn list_sources(
        &self,
        category: Option<&str>,
        source_type: Option<&str>,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<ListDataSourcesResponse> {
        let normalized_limit = limit.clamp(1, 500);
        let normalized_offset = offset.max(0);
        let category_filter = category.map(str::to_owned);
        let source_type_filter = source_type.map(str::to_owned);

        let mut count_qb = QueryBuilder::<Postgres>::new(
            "SELECT COUNT(*)::BIGINT AS total FROM data_sources ds WHERE 1=1",
        );
        Self::apply_source_filters(
            &mut count_qb,
            category_filter.clone(),
            source_type_filter.clone(),
            enabled_only,
        );
        let count_row = count_qb
            .build()
            .fetch_one(&*self.pool)
            .await
            .context("failed to count filtered data sources")?;
        let total: i64 = count_row
            .try_get("total")
            .context("missing total count column")?;

        let mut data_qb = QueryBuilder::<Postgres>::new(
            r#"
            SELECT
                ds.id,
                ds.name,
                ds.description,
                ds.provider,
                ds.category,
                ds.source_type,
                ds.base_url,
                ds.api_version,
                ds.documentation_url,
                ds.auth_type,
                ds.api_key_env_var,
                ds.rate_limit_requests,
                ds.rate_limit_window,
                ds.status,
                ds.is_enabled,
                ds.priority,
                ds.used_for_training,
                ds.training_data_volume,
                ds.created_at,
                ds.updated_at,
                ds.last_tested_at,
                ds.last_error,
                ds.config
            FROM data_sources ds
            WHERE 1=1
            "#,
        );
        Self::apply_source_filters(
            &mut data_qb,
            category_filter,
            source_type_filter,
            enabled_only,
        );
        data_qb.push(" ORDER BY ds.priority ASC, ds.name ASC LIMIT ");
        data_qb.push_bind(normalized_limit);
        data_qb.push(" OFFSET ");
        data_qb.push_bind(normalized_offset);

        let rows = data_qb
            .build()
            .fetch_all(&*self.pool)
            .await
            .context("failed to list filtered data sources")?;

        let mut sources = Vec::with_capacity(rows.len());
        for row in rows {
            let source_row = Self::map_data_source_row(&row)?;
            sources.push(DataSourceSummary::from(source_row));
        }

        Ok(ListDataSourcesResponse {
            sources,
            total,
            limit: normalized_limit,
            offset: normalized_offset,
        })
    }

    /// Get data source by ID
    pub async fn get_source(&self, id: Uuid) -> anyhow::Result<Option<DataSourceDetailResponse>> {
        let source_row = sqlx::query(
            r#"
            SELECT
                ds.id,
                ds.name,
                ds.description,
                ds.provider,
                ds.category,
                ds.source_type,
                ds.base_url,
                ds.api_version,
                ds.documentation_url,
                ds.auth_type,
                ds.api_key_env_var,
                ds.rate_limit_requests,
                ds.rate_limit_window,
                ds.status,
                ds.is_enabled,
                ds.priority,
                ds.used_for_training,
                ds.training_data_volume,
                ds.created_at,
                ds.updated_at,
                ds.last_tested_at,
                ds.last_error,
                ds.config
            FROM data_sources ds
            WHERE ds.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .context("failed to fetch data source by id")?;

        let Some(source_row) = source_row else {
            return Ok(None);
        };

        let source = Self::map_data_source_row(&source_row)?;

        let endpoint_rows = sqlx::query(
            r#"
            SELECT
                e.id,
                e.name,
                e.method,
                e.path,
                e.description,
                e.is_active
            FROM data_source_endpoints e
            WHERE e.source_id = $1
            ORDER BY e.method, e.path
            "#,
        )
        .bind(id)
        .fetch_all(&*self.pool)
        .await
        .context("failed to fetch data source endpoints")?;

        let endpoints = endpoint_rows
            .into_iter()
            .map(|row| EndpointSummary {
                id: row.try_get("id").unwrap_or_default(),
                name: row.try_get("name").unwrap_or_default(),
                method: row.try_get("method").unwrap_or_default(),
                path: row.try_get("path").unwrap_or_default(),
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                is_active: row.try_get("is_active").unwrap_or(false),
            })
            .collect();

        let pricing_rows = sqlx::query(
            r#"
            SELECT
                p.tier_name,
                p.tier_level,
                p.price_monthly_usd,
                p.price_yearly_usd,
                p.requests_per_day,
                p.features
            FROM data_source_pricing p
            WHERE p.source_id = $1
            ORDER BY p.tier_level ASC, p.tier_name ASC
            "#,
        )
        .bind(id)
        .fetch_all(&*self.pool)
        .await
        .context("failed to fetch data source pricing")?;

        let pricing = pricing_rows
            .into_iter()
            .map(|row| {
                let features = row
                    .try_get::<serde_json::Value, _>("features")
                    .map_or_else(|_| Vec::new(), |value| parse_json_string_array(&value));

                PricingTierSummary {
                    tier_name: row.try_get("tier_name").unwrap_or_default(),
                    tier_level: row.try_get("tier_level").unwrap_or(1),
                    price_monthly_usd: row.try_get("price_monthly_usd").ok(),
                    price_yearly_usd: row.try_get("price_yearly_usd").ok(),
                    requests_per_day: row.try_get("requests_per_day").ok(),
                    features,
                }
            })
            .collect();

        let rate_limit = match (source.rate_limit_requests, source.rate_limit_window.clone()) {
            (Some(requests), Some(window)) => Some(RateLimitInfo { requests, window }),
            _ => None,
        };

        Ok(Some(DataSourceDetailResponse {
            id: source.id,
            name: source.name,
            description: source.description.unwrap_or_default(),
            provider: source.provider,
            category: crate::data_sources::parse_category(&source.category),
            source_type: crate::data_sources::parse_source_type(&source.source_type),
            base_url: source.base_url,
            documentation_url: source.documentation_url,
            auth_type: crate::data_sources::parse_auth_type(&source.auth_type),
            rate_limit,
            status: crate::data_sources::parse_status(&source.status),
            is_enabled: source.is_enabled,
            priority: source.priority,
            used_for_training: source.used_for_training,
            endpoints,
            pricing,
            last_tested_at: source.last_tested_at,
        }))
    }

    /// Test connection to data source
    pub async fn test_connection(&self, id: Uuid) -> anyhow::Result<TestConnectionResponse> {
        let source_row = sqlx::query(
            r#"
            SELECT id, provider, config
            FROM data_sources
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .context("failed to fetch data source configuration")?;

        let Some(row) = source_row else {
            return Ok(TestConnectionResponse {
                success: false,
                message: "Data source not found".to_string(),
                response_time_ms: None,
                timestamp: Utc::now(),
            });
        };

        let provider: String = row.try_get("provider")?;
        let config: serde_json::Value = row.try_get("config")?;

        let test_result = match ConnectorFactory::create(&provider, config) {
            Some(connector) => match connector.test_connection().await {
                Ok(result) => result,
                Err(err) => ConnectionTest {
                    success: false,
                    response_time_ms: 0,
                    message: format!("Connection check failed: {}", err),
                    details: None,
                },
            },
            None => ConnectionTest {
                success: false,
                response_time_ms: 0,
                message: format!("Unknown provider '{}'", provider),
                details: None,
            },
        };

        let status = if test_result.success {
            "active"
        } else {
            "error"
        };

        sqlx::query(
            r#"
            UPDATE data_sources
            SET
                status = $1,
                last_tested_at = $2,
                last_error = $3
            WHERE id = $4
            "#,
        )
        .bind(status)
        .bind(Utc::now())
        .bind(if test_result.success {
            None::<String>
        } else {
            Some(test_result.message.clone())
        })
        .bind(id)
        .execute(&*self.pool)
        .await
        .context("failed to persist data source test result")?;

        Ok(TestConnectionResponse {
            success: test_result.success,
            message: test_result.message,
            response_time_ms: Some(test_result.response_time_ms),
            timestamp: Utc::now(),
        })
    }

    /// Enable/disable data source
    pub async fn set_enabled(&self, id: Uuid, enabled: bool) -> anyhow::Result<bool> {
        let result =
            sqlx::query("UPDATE data_sources SET is_enabled = $1, updated_at = $2 WHERE id = $3")
                .bind(enabled)
                .bind(Utc::now())
                .bind(id)
                .execute(&*self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get free data sources (for training)
    pub async fn get_free_sources(&self) -> anyhow::Result<Vec<DataSourceSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                ds.id,
                ds.name,
                ds.description,
                ds.provider,
                ds.category,
                ds.source_type,
                ds.base_url,
                ds.api_version,
                ds.documentation_url,
                ds.auth_type,
                ds.api_key_env_var,
                ds.rate_limit_requests,
                ds.rate_limit_window,
                ds.status,
                ds.is_enabled,
                ds.priority,
                ds.used_for_training,
                ds.training_data_volume,
                ds.created_at,
                ds.updated_at,
                ds.last_tested_at,
                ds.last_error,
                ds.config
            FROM data_sources ds
            WHERE ds.source_type = 'free'
              AND ds.is_enabled = TRUE
            ORDER BY ds.priority ASC, ds.name ASC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("failed to fetch free data sources")?;

        let mut sources = Vec::with_capacity(rows.len());
        for row in rows {
            let source_row = Self::map_data_source_row(&row)?;
            sources.push(DataSourceSummary::from(source_row));
        }

        Ok(sources)
    }

    /// Get pricing catalog
    pub async fn get_pricing_catalog(&self) -> anyhow::Result<PricingCatalogResponse> {
        let rows = sqlx::query(
            r#"
            SELECT
                ds.id,
                ds.name,
                ds.provider,
                ds.category,
                ds.source_type,
                dp.tier_name,
                dp.tier_level,
                dp.price_monthly_usd,
                dp.price_yearly_usd,
                dp.requests_per_day,
                dp.requests_per_minute,
                dp.features
            FROM data_sources ds
            LEFT JOIN data_source_pricing dp ON ds.id = dp.source_id
            WHERE ds.is_enabled = TRUE
            ORDER BY ds.category ASC, ds.priority ASC, ds.name ASC, dp.tier_level ASC NULLS LAST
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .context("failed to fetch pricing catalog")?;

        let mut categories: BTreeMap<String, BTreeMap<Uuid, SourcePricingInfo>> = BTreeMap::new();

        for row in rows {
            let source_id: Uuid = row.try_get("id")?;
            let source_name: String = row.try_get("name")?;
            let provider: String = row.try_get("provider")?;
            let category_raw: String = row.try_get("category")?;
            let source_type_raw: String = row.try_get("source_type")?;

            let category_sources = categories.entry(category_raw.clone()).or_default();
            let source_entry =
                category_sources
                    .entry(source_id)
                    .or_insert_with(|| SourcePricingInfo {
                        id: source_id,
                        name: source_name,
                        provider,
                        source_type: crate::data_sources::parse_source_type(&source_type_raw),
                        tiers: Vec::new(),
                    });

            let tier_name: Option<String> = row.try_get("tier_name")?;
            if let Some(tier_name) = tier_name {
                let features_value: Option<serde_json::Value> = row.try_get("features").ok();
                source_entry.tiers.push(PricingTierDetail {
                    tier_name,
                    tier_level: row.try_get::<Option<i32>, _>("tier_level")?.unwrap_or(1),
                    price_monthly_usd: row.try_get("price_monthly_usd").ok(),
                    price_yearly_usd: row.try_get("price_yearly_usd").ok(),
                    requests_per_day: row.try_get("requests_per_day").ok(),
                    requests_per_minute: row.try_get("requests_per_minute").ok(),
                    features: features_value
                        .as_ref()
                        .map(parse_json_string_array)
                        .unwrap_or_default(),
                });
            }
        }

        let pricing_categories = categories
            .into_iter()
            .map(|(category_raw, sources)| PricingCategory {
                category: crate::data_sources::parse_category(&category_raw),
                sources: sources.into_values().collect(),
            })
            .collect();

        Ok(PricingCatalogResponse {
            categories: pricing_categories,
        })
    }

    /// Estimate costs for selected sources
    pub async fn estimate_costs(
        &self,
        _source_ids: &[Uuid],
        _estimated_requests_per_day: i32,
    ) -> anyhow::Result<CostEstimateResponse> {
        Ok(CostEstimateResponse {
            estimates: vec![],
            total_monthly_usd: 0.0,
            total_yearly_usd: 0.0,
        })
    }

    /// List scraper jobs
    pub async fn list_scraper_jobs(&self) -> anyhow::Result<Vec<ScraperJobResponse>> {
        Ok(vec![])
    }

    /// Create scraper job
    pub async fn create_scraper_job(
        &self,
        req: CreateScraperJobRequest,
    ) -> anyhow::Result<ScraperJobResponse> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO scraper_jobs (
                id, name, description, target_url, url_pattern,
                scraper_type, config, schedule, timezone,
                status, output_dataset, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.target_url)
        .bind(&req.url_pattern)
        .bind(format!("{:?}", req.scraper_type).to_lowercase())
        .bind(&req.config)
        .bind(&req.schedule)
        .bind(req.timezone.as_deref().unwrap_or("UTC"))
        .bind("pending")
        .bind(&req.output_dataset)
        .bind(Utc::now())
        .execute(&*self.pool)
        .await?;

        Ok(ScraperJobResponse {
            id,
            name: req.name,
            description: req.description,
            target_url: req.target_url,
            scraper_type: req.scraper_type,
            status: super::ScraperJobStatus::Pending,
            schedule: req.schedule,
            last_run_at: None,
            next_run_at: None,
            created_at: Utc::now(),
        })
    }

    /// List ML datasets
    pub async fn list_ml_datasets(&self) -> anyhow::Result<Vec<MlDatasetResponse>> {
        Ok(vec![])
    }

    /// Create ML dataset
    pub async fn create_ml_dataset(
        &self,
        req: CreateMlDatasetRequest,
    ) -> anyhow::Result<MlDatasetResponse> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO ml_datasets (
                id, name, description, dataset_type, source_ids,
                time_range_start, time_range_end, version, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(format!("{:?}", req.dataset_type).to_lowercase())
        .bind(&req.source_ids)
        .bind(req.date_range_start)
        .bind(req.date_range_end)
        .bind(1i32)
        .bind(Utc::now())
        .execute(&*self.pool)
        .await?;

        Ok(MlDatasetResponse {
            id,
            name: req.name,
            dataset_type: req.dataset_type,
            record_count: 0,
            size_bytes: 0,
            quality_score: None,
            version: 1,
            created_at: Utc::now(),
        })
    }

    fn apply_source_filters(
        qb: &mut QueryBuilder<'_, Postgres>,
        category: Option<String>,
        source_type: Option<String>,
        enabled_only: bool,
    ) {
        if let Some(category) = category {
            qb.push(" AND ds.category = ");
            qb.push_bind(category);
        }

        if let Some(source_type) = source_type {
            qb.push(" AND ds.source_type = ");
            qb.push_bind(source_type);
        }

        if enabled_only {
            qb.push(" AND ds.is_enabled = TRUE");
        }
    }

    fn map_data_source_row(row: &PgRow) -> anyhow::Result<DataSourceRow> {
        Ok(DataSourceRow {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            provider: row.try_get("provider")?,
            category: row.try_get("category")?,
            source_type: row.try_get("source_type")?,
            base_url: row.try_get("base_url")?,
            api_version: row.try_get("api_version")?,
            documentation_url: row.try_get("documentation_url")?,
            auth_type: row.try_get("auth_type")?,
            api_key_env_var: row.try_get("api_key_env_var")?,
            rate_limit_requests: row.try_get("rate_limit_requests")?,
            rate_limit_window: row.try_get("rate_limit_window")?,
            status: row.try_get("status")?,
            is_enabled: row.try_get("is_enabled")?,
            priority: row.try_get("priority")?,
            used_for_training: row.try_get("used_for_training")?,
            training_data_volume: row.try_get("training_data_volume")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            last_tested_at: row.try_get("last_tested_at")?,
            last_error: row.try_get("last_error")?,
            config: row.try_get("config")?,
        })
    }
}

fn parse_json_string_array(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_string_array_valid() {
        let value = serde_json::json!(["quotes", "historical", "news"]);
        let parsed = parse_json_string_array(&value);

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "quotes");
        assert_eq!(parsed[2], "news");
    }

    #[test]
    fn test_parse_json_string_array_non_array() {
        let value = serde_json::json!({ "feature": "quotes" });
        let parsed = parse_json_string_array(&value);
        assert!(parsed.is_empty());
    }
}
