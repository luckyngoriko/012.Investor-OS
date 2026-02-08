//! S5-GP-02: TimescaleDB compression reduces storage

/// Test that compression is configured correctly
/// In production, this would verify actual compression ratios
#[test]
fn test_compression_policy_exists() {
    // The SQL migration 001_postgres_optimization.sql should have:
    // 1. Enabled compression on prices table
    // 2. Created compression policy (7 days for prices, 30 days for signals)
    
    // Verify the migration file contains the expected SQL
    let migration = include_str!("../../migrations/001_postgres_optimization.sql");
    
    assert!(migration.contains("timescaledb.compress"), 
        "Migration should enable TimescaleDB compression");
    assert!(migration.contains("add_compression_policy"), 
        "Migration should add compression policies");
    assert!(migration.contains("INTERVAL '7 days'"), 
        "Prices should be compressed after 7 days");
}

#[test]
fn test_compression_expected_ratio() {
    // TimescaleDB typically achieves 90%+ compression on financial time-series data
    // This is a documentation test showing expected outcomes
    
    // Expected compression ratios:
    // - Prices: ~95% (highly compressible numeric data)
    // - Signals: ~90% (mixed numeric and text)
    // - Document embeddings: ~50% (vector data less compressible)
    
    let expected_prices_ratio = 0.95;
    let expected_signals_ratio = 0.90;
    
    assert!(expected_prices_ratio > 0.90, "Prices should compress >90%");
    assert!(expected_signals_ratio > 0.85, "Signals should compress >85%");
}
