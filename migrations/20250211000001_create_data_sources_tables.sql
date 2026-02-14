-- Migration: Data Sources Management System
-- Creates tables for managing free and paid data sources

-- ============================================
-- 1. DATA SOURCES TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS data_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- Basic Information
    name VARCHAR(255) NOT NULL,
    description TEXT,
    provider VARCHAR(100) NOT NULL,
    category VARCHAR(50) NOT NULL, -- 'market_data', 'economic', 'news', 'alternative', 'geospatial'
    
    -- Source Type
    source_type VARCHAR(50) NOT NULL, -- 'free', 'freemium', 'paid', 'scraped', 'government'
    
    -- API Information
    base_url VARCHAR(500),
    api_version VARCHAR(20),
    documentation_url VARCHAR(500),
    
    -- Authentication
    auth_type VARCHAR(50) DEFAULT 'none', -- 'none', 'api_key', 'oauth2', 'bearer'
    api_key_env_var VARCHAR(100),
    
    -- Rate Limits (for free tiers)
    rate_limit_requests INTEGER,
    rate_limit_window VARCHAR(20), -- 'day', 'minute', 'hour', 'month'
    
    -- Status
    status VARCHAR(50) DEFAULT 'inactive',
    is_enabled BOOLEAN DEFAULT false,
    priority INTEGER DEFAULT 100,
    
    -- ML Usage
    used_for_training BOOLEAN DEFAULT false,
    training_data_volume BIGINT DEFAULT 0,
    
    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_tested_at TIMESTAMPTZ,
    last_error TEXT,
    
    -- Configuration (JSON)
    config JSONB DEFAULT '{}',
    
    -- Constraints
    CONSTRAINT valid_source_type CHECK (source_type IN ('free', 'freemium', 'paid', 'scraped', 'government')),
    CONSTRAINT valid_category CHECK (category IN ('market_data', 'economic', 'news', 'alternative', 'geospatial', 'commodities', 'crypto', 'forex')),
    CONSTRAINT valid_auth_type CHECK (auth_type IN ('none', 'api_key', 'oauth2', 'bearer', 'basic'))
);

-- Create indexes
CREATE INDEX idx_data_sources_category ON data_sources(category);
CREATE INDEX idx_data_sources_source_type ON data_sources(source_type);
CREATE INDEX idx_data_sources_status ON data_sources(status);
CREATE INDEX idx_data_sources_enabled ON data_sources(is_enabled);

-- ============================================
-- 2. DATA SOURCE ENDPOINTS TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS data_source_endpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    
    name VARCHAR(100) NOT NULL,
    method VARCHAR(10) NOT NULL CHECK (method IN ('GET', 'POST', 'PUT', 'DELETE')),
    path VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- Parameters
    required_params JSONB DEFAULT '[]',
    optional_params JSONB DEFAULT '{}',
    
    -- Response
    response_schema JSONB,
    
    -- Usage
    is_active BOOLEAN DEFAULT true,
    avg_response_ms INTEGER,
    success_rate DECIMAL(5,2) CHECK (success_rate >= 0 AND success_rate <= 100),
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT unique_endpoint UNIQUE (source_id, path, method)
);

CREATE INDEX idx_endpoints_source ON data_source_endpoints(source_id);
CREATE INDEX idx_endpoints_active ON data_source_endpoints(is_active);

-- ============================================
-- 3. DATA SOURCE PRICING TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS data_source_pricing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES data_sources(id) ON DELETE CASCADE,
    
    -- Tier Information
    tier_name VARCHAR(100) NOT NULL,
    tier_level INTEGER DEFAULT 1 CHECK (tier_level >= 1), -- 1=Free, 2=Basic, 3=Pro, 4=Enterprise
    
    -- Pricing
    price_monthly_usd DECIMAL(10,2),
    price_yearly_usd DECIMAL(10,2),
    currency VARCHAR(3) DEFAULT 'USD',
    
    -- Limits
    requests_per_day INTEGER,
    requests_per_minute INTEGER,
    data_points_per_request INTEGER,
    historical_data_years INTEGER,
    
    -- Features
    features JSONB DEFAULT '[]',
    
    -- Notes
    notes TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_pricing_source ON data_source_pricing(source_id);
CREATE INDEX idx_pricing_tier ON data_source_pricing(tier_level);

-- ============================================
-- 4. SCRAPER JOBS TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS scraper_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    name VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- Target
    target_url VARCHAR(500),
    url_pattern VARCHAR(255),
    
    -- Scraping Config
    scraper_type VARCHAR(50) NOT NULL, -- 'firecrawl', 'scrapy', 'playwright', 'custom'
    config JSONB DEFAULT '{}',
    
    -- Schedule
    schedule VARCHAR(100), -- cron expression
    timezone VARCHAR(50) DEFAULT 'UTC',
    
    -- Status
    status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'running', 'completed', 'failed', 'paused'
    
    -- Results
    last_run_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ,
    last_result JSONB,
    
    -- ML Integration
    output_dataset VARCHAR(100),
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_scraper_status ON scraper_jobs(status);
CREATE INDEX idx_scraper_next_run ON scraper_jobs(next_run_at);

-- ============================================
-- 5. ML DATASETS TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS ml_datasets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    name VARCHAR(255) NOT NULL,
    description TEXT,
    dataset_type VARCHAR(50) NOT NULL, -- 'training', 'validation', 'test'
    
    -- Source Information
    source_ids UUID[] DEFAULT '{}',
    scraper_job_ids UUID[] DEFAULT '{}',
    
    -- Volume
    record_count BIGINT DEFAULT 0,
    size_bytes BIGINT DEFAULT 0,
    time_range_start TIMESTAMPTZ,
    time_range_end TIMESTAMPTZ,
    
    -- Quality
    quality_score DECIMAL(3,2) CHECK (quality_score >= 0 AND quality_score <= 1),
    validation_errors JSONB DEFAULT '[]',
    
    -- Storage
    storage_path VARCHAR(500),
    format VARCHAR(20), -- 'parquet', 'csv', 'json', 'feather'
    
    -- Versioning
    version INTEGER DEFAULT 1,
    parent_version_id UUID REFERENCES ml_datasets(id),
    
    -- Usage
    used_in_models TEXT[] DEFAULT '{}',
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_datasets_type ON ml_datasets(dataset_type);
CREATE INDEX idx_datasets_quality ON ml_datasets(quality_score);

-- ============================================
-- 6. USAGE LOGS TABLE (TimescaleDB hypertable)
-- ============================================
CREATE TABLE IF NOT EXISTS data_source_usage_logs (
    id UUID,
    source_id UUID REFERENCES data_sources(id),
    endpoint_id UUID REFERENCES data_source_endpoints(id),
    
    request_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    response_time_ms INTEGER,
    
    request_params JSONB,
    response_status INTEGER,
    
    records_fetched INTEGER DEFAULT 0,
    error_message TEXT,
    
    cost_usd DECIMAL(10,6) DEFAULT 0,
    
    PRIMARY KEY (id, request_time)
);

-- Convert to hypertable if TimescaleDB is available
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
        PERFORM create_hypertable('data_source_usage_logs', 'request_time', if_not_exists => TRUE);
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'TimescaleDB not available, keeping as regular table';
END $$;

CREATE INDEX idx_usage_logs_source ON data_source_usage_logs(source_id, request_time DESC);
CREATE INDEX idx_usage_logs_time ON data_source_usage_logs(request_time DESC);

-- ============================================
-- 7. UPDATE TRIGGER
-- ============================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_data_sources_updated_at BEFORE UPDATE ON data_sources
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_data_source_endpoints_updated_at BEFORE UPDATE ON data_source_endpoints
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_data_source_pricing_updated_at BEFORE UPDATE ON data_source_pricing
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_scraper_jobs_updated_at BEFORE UPDATE ON scraper_jobs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
