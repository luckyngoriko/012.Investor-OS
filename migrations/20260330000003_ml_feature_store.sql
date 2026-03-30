-- Migration: ML Feature Store (Sprint 115)
-- Centralized storage for computed features shared across models.
-- TimescaleDB hypertable for time-series access patterns.

-- ============================================
-- 1. FEATURE STORE TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS ml_feature_store (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Feature identity
    symbol VARCHAR(20) NOT NULL,
    feature_set VARCHAR(50) NOT NULL,       -- 'technical','sentiment','fundamental','macro'

    -- Feature values
    features JSONB NOT NULL,                 -- {"rsi_14": 65.2, "macd_signal": 0.003, ...}

    -- Validity
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_until TIMESTAMPTZ,                 -- optional TTL for stale feature detection

    -- Source tracking
    source VARCHAR(50) NOT NULL,             -- 'rust_native','sidecar','external'

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_ml_feature_store_symbol ON ml_feature_store(symbol);
CREATE INDEX IF NOT EXISTS idx_ml_feature_store_set ON ml_feature_store(feature_set);
CREATE INDEX IF NOT EXISTS idx_ml_feature_store_computed ON ml_feature_store(computed_at);
CREATE INDEX IF NOT EXISTS idx_ml_feature_store_symbol_set
    ON ml_feature_store(symbol, feature_set, computed_at DESC);

-- Convert to TimescaleDB hypertable if available
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
        PERFORM create_hypertable('ml_feature_store', 'computed_at', if_not_exists => TRUE);
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Skipping ml_feature_store hypertable conversion: %', SQLERRM;
END;
$$;
