-- Migration: Multi-Tenant Database Schema (Wave 1a – Task 3)
-- Adds tenant isolation for SaaS multi-user platform.
-- - Adds user_id to ml_predictions and ml_feature_store (nullable, no FK)
-- - Creates user_strategies table (per-user trading strategies)
-- - Creates user_trades table (per-user trade execution log)
-- - Enables Row Level Security (RLS) on new user tables
-- - Creates indexes for efficient per-user queries

-- ============================================
-- 0. PREREQUISITE: Ensure auth_users exists
--    (auth_users migration may not have been applied yet)
-- ============================================
CREATE TABLE IF NOT EXISTS auth_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    password_hash TEXT NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'viewer'
        CHECK (role IN ('admin', 'trader', 'viewer')),
    permissions JSONB NOT NULL DEFAULT '[]',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS auth_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    refresh_token_hash TEXT NOT NULL UNIQUE,
    client_ip VARCHAR(45),
    user_agent TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_auth_sessions_user_id ON auth_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_sessions_expires_at ON auth_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_auth_users_email ON auth_users(email);

-- ============================================
-- 1. ADD user_id TO ml_predictions
--    (nullable, no FK – avoids breaking existing rows)
-- ============================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'ml_predictions' AND column_name = 'user_id'
    ) THEN
        ALTER TABLE ml_predictions ADD COLUMN user_id UUID;
    END IF;
END;
$$;

CREATE INDEX IF NOT EXISTS idx_ml_predictions_user_id ON ml_predictions(user_id);

-- ============================================
-- 2. ADD user_id TO ml_feature_store
--    (nullable, no FK – avoids breaking existing rows)
-- ============================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'ml_feature_store' AND column_name = 'user_id'
    ) THEN
        ALTER TABLE ml_feature_store ADD COLUMN user_id UUID;
    END IF;
END;
$$;

CREATE INDEX IF NOT EXISTS idx_ml_feature_store_user_id ON ml_feature_store(user_id);

-- ============================================
-- 3. USER STRATEGIES TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS user_strategies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Owner
    user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,

    -- Strategy identity
    name VARCHAR(100) NOT NULL,

    -- Configuration
    symbols TEXT[] NOT NULL DEFAULT '{BTCUSDT}',
    models TEXT[] NOT NULL DEFAULT '{catboost,garch}',
    mode VARCHAR(20) NOT NULL DEFAULT 'signal'
        CHECK (mode IN ('signal', 'semi_auto', 'full_auto')),

    -- Risk management
    risk_limits JSONB NOT NULL DEFAULT '{"max_position_pct":5,"max_daily_loss_pct":2,"max_drawdown_pct":10}',

    -- Lifecycle
    is_active BOOLEAN NOT NULL DEFAULT true,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_user_strategies_user_id ON user_strategies(user_id);
CREATE INDEX IF NOT EXISTS idx_user_strategies_active ON user_strategies(user_id, is_active)
    WHERE is_active = true;

-- Auto-update updated_at
CREATE OR REPLACE FUNCTION update_user_strategies_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_user_strategies_updated_at ON user_strategies;
CREATE TRIGGER trg_user_strategies_updated_at
    BEFORE UPDATE ON user_strategies
    FOR EACH ROW
    EXECUTE FUNCTION update_user_strategies_updated_at();

-- ============================================
-- 4. USER TRADES TABLE
-- ============================================
CREATE TABLE IF NOT EXISTS user_trades (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Owner
    user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,

    -- Strategy reference (nullable – manual trades have no strategy)
    strategy_id UUID REFERENCES user_strategies(id) ON DELETE SET NULL,

    -- Trade details
    broker_type VARCHAR(20) NOT NULL,      -- 'paper','ibkr','binance','polygon'
    symbol VARCHAR(20) NOT NULL,
    side VARCHAR(10) NOT NULL
        CHECK (side IN ('buy', 'sell')),

    -- Quantities
    quantity DOUBLE PRECISION NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    commission DOUBLE PRECISION NOT NULL DEFAULT 0,
    pnl DOUBLE PRECISION,                  -- filled after position close

    -- Execution
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_user_trades_user_id ON user_trades(user_id);
CREATE INDEX IF NOT EXISTS idx_user_trades_strategy ON user_trades(strategy_id);
CREATE INDEX IF NOT EXISTS idx_user_trades_symbol ON user_trades(symbol);
CREATE INDEX IF NOT EXISTS idx_user_trades_executed ON user_trades(executed_at);
CREATE INDEX IF NOT EXISTS idx_user_trades_user_symbol ON user_trades(user_id, symbol, executed_at DESC);

-- ============================================
-- 5. ROW LEVEL SECURITY (RLS)
-- ============================================

-- Enable RLS on user_strategies
ALTER TABLE user_strategies ENABLE ROW LEVEL SECURITY;

-- Policy: users can only see/modify their own strategies
DROP POLICY IF EXISTS user_strategies_isolation ON user_strategies;
CREATE POLICY user_strategies_isolation ON user_strategies
    USING (user_id = current_setting('app.current_user_id', true)::UUID)
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

-- Admin bypass: table owner (investor) bypasses RLS
-- (PostgreSQL default: table owner is exempt from RLS unless FORCE ROW LEVEL SECURITY)

-- Enable RLS on user_trades
ALTER TABLE user_trades ENABLE ROW LEVEL SECURITY;

-- Policy: users can only see/modify their own trades
DROP POLICY IF EXISTS user_trades_isolation ON user_trades;
CREATE POLICY user_trades_isolation ON user_trades
    USING (user_id = current_setting('app.current_user_id', true)::UUID)
    WITH CHECK (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================
-- 6. VERIFICATION QUERIES (informational)
-- ============================================
-- After running this migration, verify with:
--   \dt user_*
--   \d user_strategies
--   \d user_trades
--   SELECT column_name FROM information_schema.columns WHERE table_name = 'ml_predictions' AND column_name = 'user_id';
--   SELECT column_name FROM information_schema.columns WHERE table_name = 'ml_feature_store' AND column_name = 'user_id';
