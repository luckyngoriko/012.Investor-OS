//! Sprint 78: Post-Certification Reliability Guardrails
//!
//! Focus:
//! - Migration portability checks
//! - Seed schema drift prevention
//! - Idempotent migration execution on a clean database

use regex::Regex;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::time::Duration;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

fn read_migration(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read migration file {path}: {err}");
    })
}

fn replace_database_name(url: &str, db_name: &str) -> String {
    let (base, query) = match url.split_once('?') {
        Some((left, right)) => (left, Some(right)),
        None => (url, None),
    };

    let authority_start = base.find("://").map(|index| index + 3).unwrap_or(0);

    let db_separator = base[authority_start..]
        .rfind('/')
        .map(|offset| authority_start + offset)
        .unwrap_or(base.len());

    let mut rebuilt = format!("{}/{}", &base[..db_separator], db_name);
    if let Some(suffix) = query {
        rebuilt.push('?');
        rebuilt.push_str(suffix);
    }
    rebuilt
}

#[test]
fn migration_001_avoids_transaction_unsafe_concurrently_indexes() {
    let migration = read_migration("migrations/001_postgres_optimization.sql");
    assert!(
        !migration.contains("CREATE INDEX CONCURRENTLY"),
        "001 migration must not use CREATE INDEX CONCURRENTLY inside sqlx transaction"
    );
}

#[test]
fn migration_001_has_portability_guards_for_optional_features() {
    let migration = read_migration("migrations/001_postgres_optimization.sql");

    for required_marker in [
        "Skipping auto_explain extension",
        "Skipping vector extension",
        "Skipping TimescaleDB compression setup",
        "to_regclass('public.signals')",
        "to_regclass('public.positions')",
    ] {
        assert!(
            migration.contains(required_marker),
            "missing portability guard marker in 001 migration: {required_marker}"
        );
    }
}

#[test]
fn seed_migration_has_no_requests_per_month_column_reference() {
    let seed = read_migration("migrations/20250211000002_seed_data_sources.sql");
    assert!(
        !seed.contains("requests_per_month"),
        "seed migration references unsupported column requests_per_month"
    );
}

#[test]
fn seed_migration_uses_on_conflict_do_nothing_for_seed_inserts() {
    let seed = read_migration("migrations/20250211000002_seed_data_sources.sql");
    let insert_re = Regex::new(
        r"(?si)INSERT\s+INTO\s+(data_sources|data_source_endpoints|data_source_pricing)\b.*?;",
    )
    .expect("valid regex");

    let mut total = 0usize;
    for captures in insert_re.captures_iter(&seed) {
        total += 1;
        let statement = captures.get(0).expect("full match").as_str();
        let table_name = captures.get(1).expect("table capture").as_str();
        assert!(
            statement
                .to_ascii_uppercase()
                .contains("ON CONFLICT DO NOTHING"),
            "insert into {table_name} is missing ON CONFLICT DO NOTHING"
        );
    }

    assert!(total > 0, "expected at least one seed insert statement");
}

#[tokio::test]
async fn migrations_apply_and_reapply_on_fresh_database() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("Skipping migration guardrail integration test: DATABASE_URL not set");
            return;
        }
    };

    let admin_url = replace_database_name(&database_url, "postgres");
    let admin_pool = match PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&admin_url)
        .await
    {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!(
                "Skipping migration guardrail integration test: cannot connect to admin DB ({}): {}",
                admin_url, err
            );
            return;
        }
    };

    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let db_name = format!("investor_os_s78_{}_{}", std::process::id(), unique_suffix);

    sqlx::query(&format!(r#"CREATE DATABASE "{}""#, db_name))
        .execute(&admin_pool)
        .await
        .unwrap_or_else(|err| panic!("failed creating temporary database {db_name}: {err}"));

    let test_url = replace_database_name(&database_url, &db_name);
    let test_pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&test_url)
        .await
        .unwrap_or_else(|err| panic!("failed connecting to temporary database {db_name}: {err}"));

    let test_result = async {
        MIGRATOR.run(&test_pool).await?;
        MIGRATOR.run(&test_pool).await?;

        let v1_success: bool =
            sqlx::query("SELECT success FROM _sqlx_migrations WHERE version = 1")
                .fetch_one(&test_pool)
                .await?
                .get("success");

        let seed_success: bool =
            sqlx::query("SELECT success FROM _sqlx_migrations WHERE version = 20250211000002")
                .fetch_one(&test_pool)
                .await?
                .get("success");

        assert!(
            v1_success,
            "migration version 1 should be marked successful"
        );
        assert!(
            seed_success,
            "seed migration version 20250211000002 should be marked successful"
        );

        Ok::<(), sqlx::Error>(())
    }
    .await;

    drop(test_pool);
    let _ = sqlx::query(&format!(
        r#"DROP DATABASE IF EXISTS "{}" WITH (FORCE)"#,
        db_name
    ))
    .execute(&admin_pool)
    .await;

    if let Err(err) = test_result {
        panic!("migration guardrail integration test failed: {err}");
    }
}
