-- Sprint 5: PostgreSQL Performance Optimization
-- Extensions and configuration for query performance monitoring

-- ============================================
-- S5-D1: PostgreSQL Extensions
-- ============================================

-- Query performance monitoring
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- Auto-explain slow queries (configured via postgresql.conf)
CREATE EXTENSION IF NOT EXISTS auto_explain;

-- Prewarm frequently accessed tables
CREATE EXTENSION IF NOT EXISTS pg_prewarm;

-- Text similarity for ticker matching
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- ============================================
-- S5-D2: Covering Indexes for CQ Queries
-- ============================================

-- Covering index for CQ calculation queries
-- Includes all columns needed for CQ formula calculation
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_signals_cq_covering 
ON signals (ticker, calculated_at, quality_score, value_score, momentum_score, insider_score)
INCLUDE (sentiment_score, regime_fit, breakout_score);

-- Partial index for pending proposals (active decisions)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_proposals_pending
ON proposals (portfolio_id, created_at)
WHERE status = 'pending'
INCLUDE (ticker, action, shares, expected_price);

-- Index for recent price lookups with covering columns
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_prices_recent_covering
ON prices (ticker, timestamp DESC)
INCLUDE (open, high, low, close, volume, adjusted_close)
WHERE timestamp > NOW() - INTERVAL '90 days';

-- GIN index for text search in decision journal
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_decisions_journal_gin
ON decisions USING GIN (journal_entry gin_trgm_ops);

-- ============================================
-- S5-D3: TimescaleDB Compression Configuration
-- ============================================

-- Enable compression on prices hypertable
ALTER TABLE prices SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'ticker',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Compression policy: compress chunks older than 7 days
SELECT add_compression_policy('prices', INTERVAL '7 days');

-- Compression policy for signals (less frequent compression)
ALTER TABLE signals SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'ticker',
    timescaledb.compress_orderby = 'calculated_at DESC'
);
SELECT add_compression_policy('signals', INTERVAL '30 days');

-- ============================================
-- S5-D4: Materialized Views for Dashboard
-- ============================================

-- Materialized view for daily portfolio summary
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_portfolio_daily AS
SELECT 
    p.id as portfolio_id,
    p.name as portfolio_name,
    date_trunc('day', pos.entry_date) as day,
    SUM(pos.pnl) as daily_pnl,
    SUM(pos.current_value) as total_value,
    COUNT(*) FILTER (WHERE pos.status = 'open') as open_positions,
    COUNT(*) FILTER (WHERE pos.status = 'closed') as closed_positions,
    SUM(pos.pnl) FILTER (WHERE pos.pnl > 0) as gross_profit,
    SUM(pos.pnl) FILTER (WHERE pos.pnl < 0) as gross_loss
FROM portfolios p
LEFT JOIN positions pos ON pos.portfolio_id = p.id
GROUP BY p.id, p.name, date_trunc('day', pos.entry_date);

-- Index on materialized view
CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_portfolio_daily_pk 
ON mv_portfolio_daily (portfolio_id, day);

-- Materialized view for CQ history (performance tracking)
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_cq_history AS
SELECT 
    s.ticker,
    date_trunc('day', s.calculated_at) as day,
    AVG(s.quality_score) as avg_quality,
    AVG(s.value_score) as avg_value,
    AVG(s.momentum_score) as avg_momentum,
    AVG(s.insider_score) as avg_insider,
    AVG(s.sentiment_score) as avg_sentiment,
    AVG(s.regime_fit) as avg_regime_fit,
    AVG(s.composite_quality) as avg_cq,
    COUNT(*) as signal_count
FROM signals s
GROUP BY s.ticker, date_trunc('day', s.calculated_at);

-- Index on materialized view
CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_cq_history_pk 
ON mv_cq_history (ticker, day);

-- Materialized view for ticker performance summary
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_ticker_performance AS
SELECT 
    pos.ticker,
    COUNT(*) as total_trades,
    AVG(pos.pnl) as avg_pnl,
    SUM(pos.pnl) as total_pnl,
    AVG(pos.return_pct) as avg_return,
    COUNT(*) FILTER (WHERE pos.pnl > 0) as winning_trades,
    COUNT(*) FILTER (WHERE pos.pnl < 0) as losing_trades,
    CASE 
        WHEN COUNT(*) > 0 THEN 
            COUNT(*) FILTER (WHERE pos.pnl > 0)::float / COUNT(*)::float 
        ELSE 0 
    END as win_rate
FROM positions pos
WHERE pos.status = 'closed'
GROUP BY pos.ticker;

-- ============================================
-- Functions for Refreshing Materialized Views
-- ============================================

-- Function to refresh all dashboard materialized views
CREATE OR REPLACE FUNCTION refresh_dashboard_views()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_portfolio_daily;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_cq_history;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_ticker_performance;
END;
$$ LANGUAGE plpgsql;

-- ============================================
-- Configuration for Query Performance
-- ============================================

-- Enable query plan logging for slow queries (configured in postgresql.conf)
-- auto_explain.log_min_duration = '1s'
-- auto_explain.log_analyze = true
-- auto_explain.log_buffers = true

-- Prewarm critical tables
SELECT pg_prewarm('signals');
SELECT pg_prewarm('prices');
SELECT pg_prewarm('positions');
SELECT pg_prewarm('decisions');

-- ============================================
-- RAG Support Tables (S5-D5 - S5-D8)
-- ============================================

-- Table for document embeddings (SEC filings, earnings transcripts)
CREATE TABLE IF NOT EXISTS document_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticker VARCHAR(10) NOT NULL,
    document_type VARCHAR(50) NOT NULL, -- '10-K', '10-Q', 'earnings_call', 'news'
    document_date DATE NOT NULL,
    source_url TEXT,
    content_chunk TEXT NOT NULL,
    embedding vector(384), -- all-MiniLM-L6-v2 dimension
    chunk_index INTEGER NOT NULL,
    total_chunks INTEGER NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(ticker, document_type, document_date, chunk_index)
);

-- Convert to hypertable for time-series data
SELECT create_hypertable('document_embeddings', 'document_date', 
    chunk_time_interval => INTERVAL '90 days',
    if_not_exists => TRUE);

-- HNSW index for vector similarity search
CREATE INDEX IF NOT EXISTS idx_document_embeddings_vector 
ON document_embeddings USING hnsw (embedding vector_cosine_ops)
WITH (m = 16, ef_construction = 64);

-- Index for ticker + type lookups
CREATE INDEX IF NOT EXISTS idx_document_embeddings_lookup 
ON document_embeddings (ticker, document_type, document_date DESC);

-- Table for RAG query history
CREATE TABLE IF NOT EXISTS rag_queries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_text TEXT NOT NULL,
    query_embedding vector(384),
    context_ticker VARCHAR(10),
    results JSONB,
    latency_ms INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

SELECT create_hypertable('rag_queries', 'created_at', 
    chunk_time_interval => INTERVAL '30 days',
    if_not_exists => TRUE);

-- ============================================
-- Monitoring Views
-- ============================================

-- View for slow query monitoring
CREATE OR REPLACE VIEW v_slow_queries AS
SELECT 
    query,
    calls,
    mean_exec_time,
    total_exec_time,
    rows,
    100.0 * shared_blks_hit / nullif(shared_blks_hit + shared_blks_read, 0) AS hit_percent
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 50;

-- View for table sizes
CREATE OR REPLACE VIEW v_table_sizes AS
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size,
    pg_total_relation_size(schemaname||'.'||tablename) as size_bytes
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- View for index usage statistics
CREATE OR REPLACE VIEW v_index_usage AS
SELECT 
    schemaname,
    tablename,
    indexrelname as index_name,
    idx_scan as times_used,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC;
