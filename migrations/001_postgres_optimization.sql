-- Sprint 5: PostgreSQL Performance Optimization
-- Extensions and configuration for query performance monitoring

-- ============================================
-- S5-D1: PostgreSQL Extensions
-- ============================================

DO $$
BEGIN
    BEGIN
        CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'Skipping pg_stat_statements extension: %', SQLERRM;
    END;

    BEGIN
        CREATE EXTENSION IF NOT EXISTS auto_explain;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'Skipping auto_explain extension: %', SQLERRM;
    END;

    BEGIN
        CREATE EXTENSION IF NOT EXISTS pg_prewarm;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'Skipping pg_prewarm extension: %', SQLERRM;
    END;

    BEGIN
        CREATE EXTENSION IF NOT EXISTS pg_trgm;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'Skipping pg_trgm extension: %', SQLERRM;
    END;

    BEGIN
        CREATE EXTENSION IF NOT EXISTS vector;
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'Skipping vector extension: %', SQLERRM;
    END;
END $$;

-- ============================================
-- S5-D2: Covering Indexes for CQ Queries
-- ============================================

-- Covering index for CQ calculation queries
-- Includes all columns needed for CQ formula calculation
DO $$
BEGIN
    IF to_regclass('public.signals') IS NOT NULL THEN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_signals_cq_covering ON signals (ticker, calculated_at, quality_score, value_score, momentum_score, insider_score) INCLUDE (sentiment_score, regime_fit, breakout_score)';
    ELSE
        RAISE NOTICE 'Skipping idx_signals_cq_covering: signals table not found';
    END IF;
END $$;

-- Partial index for pending proposals (active decisions)
DO $$
BEGIN
    IF to_regclass('public.proposals') IS NOT NULL THEN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_proposals_pending ON proposals (portfolio_id, created_at) INCLUDE (ticker, action, shares, expected_price) WHERE status = ''pending''';
    ELSE
        RAISE NOTICE 'Skipping idx_proposals_pending: proposals table not found';
    END IF;
END $$;

-- Index for recent price lookups with covering columns
DO $$
BEGIN
    IF to_regclass('public.prices') IS NOT NULL THEN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_prices_recent_covering ON prices (ticker, "timestamp" DESC) INCLUDE (open, high, low, close, volume, adjusted_close)';
    ELSE
        RAISE NOTICE 'Skipping idx_prices_recent_covering: prices table not found';
    END IF;
END $$;

-- GIN index for text search in decision journal
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_trgm')
       AND to_regclass('public.decisions') IS NOT NULL THEN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_decisions_journal_gin ON decisions USING GIN (journal_entry gin_trgm_ops)';
    ELSE
        RAISE NOTICE 'Skipping trigram GIN index: pg_trgm extension or decisions table unavailable';
    END IF;
END $$;

-- ============================================
-- S5-D3: TimescaleDB Compression Configuration
-- ============================================

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
        BEGIN
            ALTER TABLE prices SET (
                timescaledb.compress,
                timescaledb.compress_segmentby = 'ticker',
                timescaledb.compress_orderby = 'timestamp DESC'
            );
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Skipping prices compression settings: %', SQLERRM;
        END;

        BEGIN
            PERFORM add_compression_policy('prices', INTERVAL '7 days');
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Skipping prices compression policy: %', SQLERRM;
        END;

        BEGIN
            ALTER TABLE signals SET (
                timescaledb.compress,
                timescaledb.compress_segmentby = 'ticker',
                timescaledb.compress_orderby = 'calculated_at DESC'
            );
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Skipping signals compression settings: %', SQLERRM;
        END;

        BEGIN
            PERFORM add_compression_policy('signals', INTERVAL '30 days');
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Skipping signals compression policy: %', SQLERRM;
        END;
    ELSE
        RAISE NOTICE 'Skipping TimescaleDB compression setup: extension unavailable';
    END IF;
END $$;

-- ============================================
-- S5-D4: Materialized Views for Dashboard
-- ============================================

-- Materialized view for daily portfolio summary
DO $$
BEGIN
    IF to_regclass('public.portfolios') IS NOT NULL
       AND to_regclass('public.positions') IS NOT NULL THEN
        EXECUTE $sql$
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_portfolio_daily AS
            SELECT
                p.id AS portfolio_id,
                p.name AS portfolio_name,
                date_trunc('day', pos.entry_date) AS day,
                SUM(pos.pnl) AS daily_pnl,
                SUM(pos.current_value) AS total_value,
                COUNT(*) FILTER (WHERE pos.status = 'open') AS open_positions,
                COUNT(*) FILTER (WHERE pos.status = 'closed') AS closed_positions,
                SUM(pos.pnl) FILTER (WHERE pos.pnl > 0) AS gross_profit,
                SUM(pos.pnl) FILTER (WHERE pos.pnl < 0) AS gross_loss
            FROM portfolios p
            LEFT JOIN positions pos ON pos.portfolio_id = p.id
            GROUP BY p.id, p.name, date_trunc('day', pos.entry_date)
        $sql$;
        EXECUTE 'CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_portfolio_daily_pk ON mv_portfolio_daily (portfolio_id, day)';
    ELSE
        RAISE NOTICE 'Skipping mv_portfolio_daily: portfolios/positions tables not found';
    END IF;
END $$;

-- Materialized view for CQ history (performance tracking)
DO $$
BEGIN
    IF to_regclass('public.signals') IS NOT NULL THEN
        EXECUTE $sql$
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_cq_history AS
            SELECT
                s.ticker,
                date_trunc('day', s.calculated_at) AS day,
                AVG(s.quality_score) AS avg_quality,
                AVG(s.value_score) AS avg_value,
                AVG(s.momentum_score) AS avg_momentum,
                AVG(s.insider_score) AS avg_insider,
                AVG(s.sentiment_score) AS avg_sentiment,
                AVG(s.regime_fit) AS avg_regime_fit,
                AVG(s.composite_quality) AS avg_cq,
                COUNT(*) AS signal_count
            FROM signals s
            GROUP BY s.ticker, date_trunc('day', s.calculated_at)
        $sql$;
        EXECUTE 'CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_cq_history_pk ON mv_cq_history (ticker, day)';
    ELSE
        RAISE NOTICE 'Skipping mv_cq_history: signals table not found';
    END IF;
END $$;

-- Materialized view for ticker performance summary
DO $$
BEGIN
    IF to_regclass('public.positions') IS NOT NULL THEN
        EXECUTE $sql$
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_ticker_performance AS
            SELECT
                pos.ticker,
                COUNT(*) AS total_trades,
                AVG(pos.pnl) AS avg_pnl,
                SUM(pos.pnl) AS total_pnl,
                AVG(pos.return_pct) AS avg_return,
                COUNT(*) FILTER (WHERE pos.pnl > 0) AS winning_trades,
                COUNT(*) FILTER (WHERE pos.pnl < 0) AS losing_trades,
                CASE
                    WHEN COUNT(*) > 0 THEN COUNT(*) FILTER (WHERE pos.pnl > 0)::float / COUNT(*)::float
                    ELSE 0
                END AS win_rate
            FROM positions pos
            WHERE pos.status = 'closed'
            GROUP BY pos.ticker
        $sql$;
        EXECUTE 'CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_ticker_performance_pk ON mv_ticker_performance (ticker)';
    ELSE
        RAISE NOTICE 'Skipping mv_ticker_performance: positions table not found';
    END IF;
END $$;

-- ============================================
-- Functions for Refreshing Materialized Views
-- ============================================

-- Function to refresh all dashboard materialized views
CREATE OR REPLACE FUNCTION refresh_dashboard_views()
RETURNS void AS $$
BEGIN
    IF to_regclass('public.mv_portfolio_daily') IS NOT NULL THEN
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY mv_portfolio_daily';
    END IF;
    IF to_regclass('public.mv_cq_history') IS NOT NULL THEN
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY mv_cq_history';
    END IF;
    IF to_regclass('public.mv_ticker_performance') IS NOT NULL THEN
        EXECUTE 'REFRESH MATERIALIZED VIEW CONCURRENTLY mv_ticker_performance';
    END IF;
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
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_prewarm') THEN
        IF to_regclass('public.signals') IS NOT NULL THEN
            PERFORM pg_prewarm('signals');
        END IF;
        IF to_regclass('public.prices') IS NOT NULL THEN
            PERFORM pg_prewarm('prices');
        END IF;
        IF to_regclass('public.positions') IS NOT NULL THEN
            PERFORM pg_prewarm('positions');
        END IF;
        IF to_regclass('public.decisions') IS NOT NULL THEN
            PERFORM pg_prewarm('decisions');
        END IF;
    ELSE
        RAISE NOTICE 'Skipping pg_prewarm calls: extension unavailable';
    END IF;
END $$;

-- ============================================
-- RAG Support Tables (S5-D5 - S5-D8)
-- ============================================

DO $$
DECLARE
    embedding_type TEXT;
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
        embedding_type := 'vector(384)';
    ELSE
        embedding_type := 'DOUBLE PRECISION[]';
        RAISE NOTICE 'Vector extension unavailable, using DOUBLE PRECISION[] fallback for embeddings';
    END IF;

    EXECUTE format($sql$
        CREATE TABLE IF NOT EXISTS document_embeddings (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            ticker VARCHAR(10) NOT NULL,
            document_type VARCHAR(50) NOT NULL,
            document_date DATE NOT NULL,
            source_url TEXT,
            content_chunk TEXT NOT NULL,
            embedding %s,
            chunk_index INTEGER NOT NULL,
            total_chunks INTEGER NOT NULL,
            metadata JSONB,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE(ticker, document_type, document_date, chunk_index)
        )
    $sql$, embedding_type);

    EXECUTE format($sql$
        CREATE TABLE IF NOT EXISTS rag_queries (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            query_text TEXT NOT NULL,
            query_embedding %s,
            context_ticker VARCHAR(10),
            results JSONB,
            latency_ms INTEGER,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
    $sql$, embedding_type);
END $$;

-- Convert to hypertable for time-series data where available
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
        BEGIN
            PERFORM create_hypertable(
                'document_embeddings',
                'document_date',
                chunk_time_interval => INTERVAL '90 days',
                if_not_exists => TRUE
            );
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Skipping document_embeddings hypertable conversion: %', SQLERRM;
        END;

        BEGIN
            PERFORM create_hypertable(
                'rag_queries',
                'created_at',
                chunk_time_interval => INTERVAL '30 days',
                if_not_exists => TRUE
            );
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Skipping rag_queries hypertable conversion: %', SQLERRM;
        END;
    END IF;
END $$;

-- HNSW index for vector similarity search
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector')
       AND EXISTS (SELECT 1 FROM pg_am WHERE amname = 'hnsw') THEN
        EXECUTE 'CREATE INDEX IF NOT EXISTS idx_document_embeddings_vector ON document_embeddings USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64)';
    ELSE
        RAISE NOTICE 'Skipping HNSW vector index: vector extension or hnsw access method unavailable';
    END IF;
END $$;

-- Index for ticker + type lookups
CREATE INDEX IF NOT EXISTS idx_document_embeddings_lookup
ON document_embeddings (ticker, document_type, document_date DESC);

-- ============================================
-- Monitoring Views
-- ============================================

-- View for slow query monitoring
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements') THEN
        EXECUTE $view$
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
            LIMIT 50
        $view$;
    ELSE
        EXECUTE $view$
            CREATE OR REPLACE VIEW v_slow_queries AS
            SELECT
                NULL::TEXT AS query,
                0::BIGINT AS calls,
                0::DOUBLE PRECISION AS mean_exec_time,
                0::DOUBLE PRECISION AS total_exec_time,
                0::BIGINT AS rows,
                0::DOUBLE PRECISION AS hit_percent
            WHERE FALSE
        $view$;
    END IF;
END $$;

-- View for table sizes
CREATE OR REPLACE VIEW v_table_sizes AS
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname || '.' || tablename)) AS size,
    pg_total_relation_size(schemaname || '.' || tablename) AS size_bytes
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname || '.' || tablename) DESC;

-- View for index usage statistics
CREATE OR REPLACE VIEW v_index_usage AS
SELECT
    schemaname,
    relname AS tablename,
    indexrelname AS index_name,
    idx_scan AS times_used,
    idx_tup_read AS tuples_read,
    idx_tup_fetch AS tuples_fetched
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC;
