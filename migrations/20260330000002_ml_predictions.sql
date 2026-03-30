-- Migration: ML Predictions (Sprint 115)
-- Stores all predictions from ML models with actual values for accuracy tracking.
-- TimescaleDB hypertable for efficient time-series queries.

-- ============================================
-- 1. PREDICTIONS TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS ml_predictions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Model reference
    model_id UUID NOT NULL REFERENCES ml_model_registry(id),
    model_name VARCHAR(100) NOT NULL,       -- denormalized for fast queries
    model_version VARCHAR(50) NOT NULL,     -- denormalized

    -- Prediction target
    symbol VARCHAR(20) NOT NULL,
    prediction_type VARCHAR(50) NOT NULL,   -- 'price','return','volatility','sentiment','allocation'
    horizon VARCHAR(20),                     -- '1h','4h','1d','1w','1m'

    -- Prediction values
    predicted_value JSONB NOT NULL,
    actual_value JSONB,                      -- filled after horizon elapses
    confidence FLOAT NOT NULL,

    -- Features used (for reproducibility)
    features_used JSONB DEFAULT '{}',

    -- Performance
    latency_ms INTEGER,
    error_metric FLOAT,                      -- filled after resolution (e.g., MAE, RMSE)
    is_fallback BOOLEAN NOT NULL DEFAULT FALSE,

    -- EU AI Act compliance reference
    ai_decision_log_id UUID,                 -- FK to ai_decision_logs.id

    -- Timestamps
    predicted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,                 -- when actual_value was filled
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT valid_confidence CHECK (confidence >= 0 AND confidence <= 1),
    CONSTRAINT valid_prediction_type CHECK (prediction_type IN (
        'price', 'return', 'volatility', 'sentiment', 'allocation', 'regime', 'kline'
    ))
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_ml_predictions_symbol ON ml_predictions(symbol);
CREATE INDEX IF NOT EXISTS idx_ml_predictions_model ON ml_predictions(model_name);
CREATE INDEX IF NOT EXISTS idx_ml_predictions_predicted_at ON ml_predictions(predicted_at);
CREATE INDEX IF NOT EXISTS idx_ml_predictions_type ON ml_predictions(prediction_type);
CREATE INDEX IF NOT EXISTS idx_ml_predictions_unresolved
    ON ml_predictions(predicted_at) WHERE resolved_at IS NULL;

-- Convert to TimescaleDB hypertable if available
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
        PERFORM create_hypertable('ml_predictions', 'predicted_at', if_not_exists => TRUE);
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Skipping ml_predictions hypertable conversion: %', SQLERRM;
END;
$$;

-- Materialized view: model accuracy over rolling windows
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_ml_model_accuracy AS
SELECT
    model_name,
    model_version,
    prediction_type,
    COUNT(*) AS total_predictions,
    COUNT(actual_value) AS resolved_predictions,
    AVG(error_metric) FILTER (WHERE error_metric IS NOT NULL) AS avg_error,
    AVG(confidence) AS avg_confidence,
    COUNT(*) FILTER (WHERE is_fallback) AS fallback_count,
    MIN(predicted_at) AS first_prediction,
    MAX(predicted_at) AS last_prediction
FROM ml_predictions
GROUP BY model_name, model_version, prediction_type;

CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_ml_model_accuracy
    ON mv_ml_model_accuracy(model_name, model_version, prediction_type);
