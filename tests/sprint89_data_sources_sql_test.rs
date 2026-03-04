//! Sprint 89: Data Source Service SQL Completion
//!
//! Integration evidence for SQL-backed service behavior:
//! - populated path (with runtime fixture rows)
//! - empty-result and pagination/filter contracts

use investor_os::data_sources::{DataSourceService, SourceType};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

async fn connect_test_pool() -> Option<sqlx::PgPool> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("Skipping Sprint 89 DB integration test: DATABASE_URL not set");
            return None;
        }
    };

    match PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
    {
        Ok(pool) => Some(pool),
        Err(err) => {
            eprintln!(
                "Skipping Sprint 89 DB integration test: cannot connect to DATABASE_URL: {}",
                err
            );
            None
        }
    }
}

async fn data_source_tables_exist(pool: &sqlx::PgPool) -> bool {
    let row = match sqlx::query(
        r#"
        SELECT
            to_regclass('public.data_sources') IS NOT NULL AS has_data_sources,
            to_regclass('public.data_source_endpoints') IS NOT NULL AS has_endpoints,
            to_regclass('public.data_source_pricing') IS NOT NULL AS has_pricing
        "#,
    )
    .fetch_one(pool)
    .await
    {
        Ok(row) => row,
        Err(err) => {
            eprintln!(
                "Skipping Sprint 89 DB integration test: table probe failed: {}",
                err
            );
            return false;
        }
    };

    let has_data_sources: bool = row.try_get("has_data_sources").unwrap_or(false);
    let has_endpoints: bool = row.try_get("has_endpoints").unwrap_or(false);
    let has_pricing: bool = row.try_get("has_pricing").unwrap_or(false);

    if !(has_data_sources && has_endpoints && has_pricing) {
        eprintln!(
            "Skipping Sprint 89 DB integration test: required tables missing (data_sources={}, endpoints={}, pricing={})",
            has_data_sources, has_endpoints, has_pricing
        );
        return false;
    }

    true
}

async fn cleanup_fixture_rows(
    pool: &sqlx::PgPool,
    source_ids: &[Uuid],
    endpoint_ids: &[Uuid],
    pricing_ids: &[Uuid],
) {
    for pricing_id in pricing_ids {
        let _ = sqlx::query("DELETE FROM data_source_pricing WHERE id = $1")
            .bind(pricing_id)
            .execute(pool)
            .await;
    }

    for endpoint_id in endpoint_ids {
        let _ = sqlx::query("DELETE FROM data_source_endpoints WHERE id = $1")
            .bind(endpoint_id)
            .execute(pool)
            .await;
    }

    for source_id in source_ids {
        let _ = sqlx::query("DELETE FROM data_sources WHERE id = $1")
            .bind(source_id)
            .execute(pool)
            .await;
    }
}

#[tokio::test]
async fn sprint89_sql_service_populated_fixture_contracts() {
    let Some(pool) = connect_test_pool().await else {
        return;
    };
    if !data_source_tables_exist(&pool).await {
        return;
    }

    let free_source_id = Uuid::new_v4();
    let freemium_source_id = Uuid::new_v4();
    let endpoint_id = Uuid::new_v4();
    let pricing_id = Uuid::new_v4();

    let source_ids = vec![free_source_id, freemium_source_id];
    let endpoint_ids = vec![endpoint_id];
    let pricing_ids = vec![pricing_id];

    let setup_result: Result<(), String> = async {
        sqlx::query(
            r#"
            INSERT INTO data_sources (
                id, name, description, provider, category, source_type,
                base_url, documentation_url, auth_type,
                status, is_enabled, priority, config
            ) VALUES (
                $1, $2, $3, $4, 'market_data', 'free',
                'https://fixture.free.example', 'https://fixture.free.example/docs', 'none',
                'active', TRUE, 1, '{}'::jsonb
            )
            "#,
        )
        .bind(free_source_id)
        .bind("Sprint89 Fixture Free")
        .bind("Fixture source for sprint89 integration test")
        .bind("fixture_provider_free")
        .execute(&pool)
        .await
        .map_err(|err| format!("failed inserting free fixture source: {err}"))?;

        sqlx::query(
            r#"
            INSERT INTO data_sources (
                id, name, description, provider, category, source_type,
                base_url, documentation_url, auth_type,
                status, is_enabled, priority, config
            ) VALUES (
                $1, $2, $3, $4, 'market_data', 'freemium',
                'https://fixture.freemium.example', 'https://fixture.freemium.example/docs', 'api_key',
                'active', TRUE, 2, '{"has_paid_tiers": true}'::jsonb
            )
            "#,
        )
        .bind(freemium_source_id)
        .bind("Sprint89 Fixture Freemium")
        .bind("Fixture freemium source for sprint89 integration test")
        .bind("fixture_provider_freemium")
        .execute(&pool)
        .await
        .map_err(|err| format!("failed inserting freemium fixture source: {err}"))?;

        sqlx::query(
            r#"
            INSERT INTO data_source_endpoints (
                id, source_id, name, method, path, description,
                required_params, optional_params, is_active
            ) VALUES (
                $1, $2, 'Quotes', 'GET', '/quotes', 'Fixture endpoint',
                '[]'::jsonb, '{}'::jsonb, TRUE
            )
            "#,
        )
        .bind(endpoint_id)
        .bind(free_source_id)
        .execute(&pool)
        .await
        .map_err(|err| format!("failed inserting endpoint fixture row: {err}"))?;

        sqlx::query(
            r#"
            INSERT INTO data_source_pricing (
                id, source_id, tier_name, tier_level,
                price_monthly_usd, price_yearly_usd,
                requests_per_day, requests_per_minute, features
            ) VALUES (
                $1, $2, 'Free', 1,
                0, 0,
                1000, 60, '["quotes"]'::jsonb
            )
            "#,
        )
        .bind(pricing_id)
        .bind(free_source_id)
        .execute(&pool)
        .await
        .map_err(|err| format!("failed inserting pricing fixture row: {err}"))?;

        Ok(())
    }
    .await;

    if let Err(err) = setup_result {
        cleanup_fixture_rows(&pool, &source_ids, &endpoint_ids, &pricing_ids).await;
        panic!("{err}");
    }

    let service = DataSourceService::new(Arc::new(pool.clone()));

    let test_result: Result<(), String> = async {
        let listed = service
            .list_sources(None, None, true, 50, 0)
            .await
            .map_err(|err| format!("list_sources failed: {err}"))?;

        let listed_ids: Vec<Uuid> = listed.sources.iter().map(|s| s.id).collect();
        if !listed_ids.contains(&free_source_id) || !listed_ids.contains(&freemium_source_id) {
            return Err("list_sources missing inserted fixture ids".to_string());
        }
        if listed.limit != 50 || listed.offset != 0 {
            return Err(format!(
                "list_sources contract mismatch limit/offset: got limit={} offset={}",
                listed.limit, listed.offset
            ));
        }

        let free_only = service
            .list_sources(None, Some("free"), true, 100, 0)
            .await
            .map_err(|err| format!("list_sources free filter failed: {err}"))?;
        if !free_only.sources.iter().any(|s| s.id == free_source_id) {
            return Err("free filter did not return free fixture source".to_string());
        }
        if free_only
            .sources
            .iter()
            .any(|s| s.source_type != SourceType::Free)
        {
            return Err("free filter returned non-free source type".to_string());
        }

        let detail = service
            .get_source(free_source_id)
            .await
            .map_err(|err| format!("get_source failed: {err}"))?
            .ok_or_else(|| "get_source returned None for inserted fixture source".to_string())?;
        if detail.endpoints.is_empty() {
            return Err("get_source detail missing endpoints".to_string());
        }
        if detail.pricing.is_empty() {
            return Err("get_source detail missing pricing tiers".to_string());
        }

        let free_sources = service
            .get_free_sources()
            .await
            .map_err(|err| format!("get_free_sources failed: {err}"))?;
        if !free_sources.iter().any(|s| s.id == free_source_id) {
            return Err("get_free_sources missing free fixture source".to_string());
        }
        if free_sources
            .iter()
            .any(|s| s.source_type != SourceType::Free || !s.is_enabled)
        {
            return Err(
                "get_free_sources returned invalid source type or disabled row".to_string(),
            );
        }

        let pricing_catalog = service
            .get_pricing_catalog()
            .await
            .map_err(|err| format!("get_pricing_catalog failed: {err}"))?;
        let pricing_contains_fixture = pricing_catalog
            .categories
            .iter()
            .flat_map(|c| c.sources.iter())
            .any(|s| s.id == free_source_id);
        if !pricing_contains_fixture {
            return Err("pricing catalog missing fixture source".to_string());
        }

        let missing = service
            .get_source(Uuid::new_v4())
            .await
            .map_err(|err| format!("get_source on random id failed: {err}"))?;
        if missing.is_some() {
            return Err("get_source returned data for random id".to_string());
        }

        Ok(())
    }
    .await;

    drop(service);
    cleanup_fixture_rows(&pool, &source_ids, &endpoint_ids, &pricing_ids).await;

    if let Err(err) = test_result {
        panic!("{err}");
    }
}

#[tokio::test]
async fn sprint89_sql_service_empty_and_pagination_contracts() {
    let Some(pool) = connect_test_pool().await else {
        return;
    };
    if !data_source_tables_exist(&pool).await {
        return;
    }

    let service = DataSourceService::new(Arc::new(pool.clone()));

    let empty = service
        .list_sources(Some("non_existing_category"), None, true, 0, -100)
        .await
        .expect("list_sources with non-existing category should succeed");
    assert_eq!(empty.total, 0);
    assert!(empty.sources.is_empty());
    assert_eq!(empty.limit, 1);
    assert_eq!(empty.offset, 0);

    let capped = service
        .list_sources(None, None, true, 10_000, 0)
        .await
        .expect("list_sources should clamp large limit");
    assert_eq!(capped.limit, 500);
}
