-- Investor OS - PostgreSQL Performance Optimizations
-- Migration: 001_postgres_optimization.sql
-- Date: 2026-02-08

-- =====================================================
-- 1. Essential Extensions
-- =====================================================

-- Query performance tracking
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- Auto-explain for slow queries
CREATE EXTENSION IF NOT EXISTS auto_explain;

-- Pre-warm tables to keep hot data in memory
CREATE EXTENSION IF NOT EXISTS pg_prewarm;

-- Fast text search for SEC filings
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- GiST index support for range queries
CREATE EXTENSION IF NOT EXISTS btree_gist;

-- UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- TimescaleDB (already installed, just ensure it's there)
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- =====================================================
-- 2. Auto-Explain Configuration (for debugging)
-- =====================================================

-- Log slow queries (> 500ms)
ALTER SYSTEM SET auto_explain.log_min_duration = '500ms';
ALTER SYSTEM SET auto_explain.log_analyze = true;
ALTER SYSTEM SET auto_explain.log_buffers = true;
ALTER SYSTEM SET auto_explain.log_timing = true;

-- =====================================================
-- 3. Table Optimizations
-- =====================================================

-- Ensure prices is a hypertable with optimal chunk size
SELECT create_hypertable('prices', 'timestamp', 
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE,
    migrate_data => TRUE
);

-- Add compression policy (compress data older than 7 days)
-- This reduces storage by ~90% for time-series data
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM timescaledb_information.compression_settings 
        WHERE hypertable_name = 'prices'
    ) THEN
        ALTER TABLE prices SET (
            timescaledb.compress,
            timescaledb.compress_segmentby = 'ticker',
            timescaledb.compress_orderby = 'timestamp DESC'
        );
        
        -- Compress after 7 days
        PERFORM add_compression_policy('prices', INTERVAL '7 days');
    END IF;
END $$;

-- Retention policy: Keep 2 years of data
SELECT add_retention_policy('prices', INTERVAL '2 years', if_not_exists => TRUE);

-- =====================================================
-- 4. Index Optimizations
-- =====================================================

-- Covering index for CQ calculations (includes frequently accessed columns)
CREATE INDEX IF NOT EXISTS idx_signals_cq_covering 
ON signals (ticker, calculated_at DESC) 
INCLUDE (cq_score, regime_fit, quality_score, value_score, momentum_score);

-- Partial index for pending proposals (only indexes pending, not closed)
CREATE INDEX IF NOT EXISTS idx_proposals_pending 
ON proposals (created_at DESC) 
WHERE status = 'PENDING';

-- GiST index for time-range queries on positions
CREATE INDEX IF NOT EXISTS idx_positions_time_range 
ON positions USING GIST (entry_date, COALESCE(closed_at, '9999-12-31'::date));

-- Trigram index for text search in journal notes
CREATE INDEX IF NOT EXISTS idx_journal_notes_trgm 
ON decision_journal USING GIN (notes gin_trgm_ops);

-- Composite index for insider transactions with clustering
CREATE INDEX IF NOT EXISTS idx_insider_transactions_clustered 
ON insider_transactions (ticker, filing_date DESC, transaction_type);

-- Index for EPS estimates by ticker and period
CREATE INDEX IF NOT EXISTS idx_eps_estimates_lookup 
ON eps_estimates (ticker, period DESC);

-- =====================================================
-- 5. Materialized Views for Dashboard Performance
-- =====================================================

-- Daily portfolio snapshots (refreshed periodically)
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_portfolio_daily AS
SELECT 
    date_trunc('day', p.timestamp) as day,
    p.ticker,
    first(p.open, p.timestamp) as day_open,
    max(p.high) as day_high,
    min(p.low) as day_low,
    last(p.close, p.timestamp) as day_close,
    sum(p.volume) as total_volume,
    count(*) as data_points
FROM prices p
GROUP BY 1, 2
ORDER BY 1 DESC, 2;

-- Create index on materialized view
CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_portfolio_daily 
ON mv_portfolio_daily (day, ticker);

-- CQ score history summary (for charts)
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_cq_history AS
SELECT 
    date_trunc('day', calculated_at) as day,
    ticker,
    avg(cq_score) as avg_cq,
    max(cq_score) as max_cq,
    min(cq_score) as min_cq,
    count(*) as calculations
FROM signals
GROUP BY 1, 2;

CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_cq_history 
ON mv_cq_history (day, ticker);

-- Win/Loss statistics for journal
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_trade_statistics AS
SELECT 
    decision_type,
    outcome,
    count(*) as trade_count,
    avg(pnl) as avg_pnl,
    sum(pnl) as total_pnl,
    avg(CASE WHEN pnl > 0 THEN pnl END) as avg_win,
    avg(CASE WHEN pnl < 0 THEN pnl END) as avg_loss
FROM decision_journal
WHERE outcome != 'PENDING'
GROUP BY 1, 2;

-- =====================================================
-- 6. Partitioned Tables for Large Datasets
-- =====================================================

-- If audit_log table exists, partition it by month
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_log') THEN
        -- Convert to hypertable if not already
        PERFORM create_hypertable('audit_log', 'created_at', 
            chunk_time_interval => INTERVAL '1 month',
            if_not_exists => TRUE
        );
    END IF;
END $$;

-- =====================================================
-- 7. Query Performance Functions
-- =====================================================

-- Function to get CQ percentile for a ticker
CREATE OR REPLACE FUNCTION get_cq_percentile(p_ticker TEXT)
RETURNS TABLE (percentile NUMERIC, avg_cq NUMERIC, max_cq NUMERIC) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY cq_score),
        AVG(cq_score),
        MAX(cq_score)
    FROM signals
    WHERE ticker = p_ticker
    AND calculated_at > NOW() - INTERVAL '90 days';
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to refresh materialized views (call from cron/job)
CREATE OR REPLACE FUNCTION refresh_dashboard_views()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_portfolio_daily;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_cq_history;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_trade_statistics;
END;
$$ LANGUAGE plpgsql;

-- =====================================================
-- 8. Table Statistics Update
-- =====================================================

-- Analyze all tables for optimal query planning
ANALYZE prices;
ANALYZE signals;
ANALYZE proposals;
ANALYZE positions;
ANALYZE decision_journal;

-- =====================================================
-- 9. Connection Pooling Settings (apply via postgresql.conf)
-- =====================================================

/*
Add to postgresql.conf or via ALTER SYSTEM:

max_connections = 200
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 16MB
maintenance_work_mem = 128MB
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1
effective_io_concurrency = 200
checkpoint_completion_target = 0.9
wal_compression = on
max_wal_size = 2GB
min_wal_size = 512MB
max_worker_processes = 8
max_parallel_workers_per_gather = 4
max_parallel_workers = 8
max_parallel_maintenance_workers = 4
*/

-- Apply configuration changes
-- SELECT pg_reload_conf();
