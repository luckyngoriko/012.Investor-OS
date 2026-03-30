-- Migration: ML Model Registry (Sprint 115)
-- Tracks all ML models (CatBoost, GARCH, FinBERT, Kronos, etc.)
-- with version history, training metadata, and status lifecycle.

-- ============================================
-- 1. MODEL REGISTRY TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS ml_model_registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Model identity
    model_name VARCHAR(100) NOT NULL,
    model_version VARCHAR(50) NOT NULL,
    model_type VARCHAR(50) NOT NULL,  -- 'catboost','garch','finbert','skfolio','kronos','chronos','tft','fingpt'

    -- Tier classification
    tier INTEGER NOT NULL DEFAULT 1,  -- 1=CPU, 2=GPU, 3=Research

    -- Lifecycle
    status VARCHAR(20) NOT NULL DEFAULT 'inactive',

    -- Storage
    file_path TEXT,
    file_hash VARCHAR(64),            -- SHA-256 of model file
    file_size_bytes BIGINT,

    -- Training metadata
    parameters JSONB DEFAULT '{}',    -- hyperparameters
    metrics JSONB DEFAULT '{}',       -- training metrics (rmse, accuracy, sharpe, etc.)
    trained_at TIMESTAMPTZ,
    training_duration_seconds INTEGER,
    training_data_hash VARCHAR(64),
    training_samples BIGINT,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    UNIQUE(model_name, model_version),
    CONSTRAINT valid_status CHECK (status IN ('active', 'inactive', 'training', 'failed', 'retired')),
    CONSTRAINT valid_tier CHECK (tier IN (1, 2, 3))
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_ml_model_registry_name ON ml_model_registry(model_name);
CREATE INDEX IF NOT EXISTS idx_ml_model_registry_status ON ml_model_registry(status);
CREATE INDEX IF NOT EXISTS idx_ml_model_registry_tier ON ml_model_registry(tier);
CREATE INDEX IF NOT EXISTS idx_ml_model_registry_type ON ml_model_registry(model_type);

-- Auto-update updated_at
CREATE OR REPLACE FUNCTION update_ml_model_registry_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_ml_model_registry_updated_at ON ml_model_registry;
CREATE TRIGGER trg_ml_model_registry_updated_at
    BEFORE UPDATE ON ml_model_registry
    FOR EACH ROW
    EXECUTE FUNCTION update_ml_model_registry_updated_at();
