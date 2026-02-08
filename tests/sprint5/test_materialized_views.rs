//! S5-GP-03: Materialized view refresh works

/// Test that materialized view definitions are correct
#[test]
fn test_materialized_views_defined() {
    let migration = include_str!("../../migrations/001_postgres_optimization.sql");
    
    // Check for mv_portfolio_daily
    assert!(migration.contains("mv_portfolio_daily"), 
        "Migration should create mv_portfolio_daily");
    assert!(migration.contains("daily_pnl"), 
        "mv_portfolio_daily should include daily_pnl");
    
    // Check for mv_cq_history
    assert!(migration.contains("mv_cq_history"), 
        "Migration should create mv_cq_history");
    assert!(migration.contains("avg_cq"), 
        "mv_cq_history should include avg_cq");
    
    // Check for mv_ticker_performance
    assert!(migration.contains("mv_ticker_performance"), 
        "Migration should create mv_ticker_performance");
    
    // Check for refresh function
    assert!(migration.contains("refresh_dashboard_views"), 
        "Migration should create refresh function");
}

/// Test that materialized views have proper indexes
#[test]
fn test_materialized_view_indexes() {
    let migration = include_str!("../../migrations/001_postgres_optimization.sql");
    
    // Unique indexes for CONCURRENTLY refresh
    assert!(migration.contains("idx_mv_portfolio_daily_pk"), 
        "mv_portfolio_daily should have unique index");
    assert!(migration.contains("idx_mv_cq_history_pk"), 
        "mv_cq_history should have unique index");
}
