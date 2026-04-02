-- Broker Connections: encrypted API key storage for multi-broker support
-- Wave 1a Task 4: Broker Gateway with AES-256-GCM Encrypted API Keys

CREATE TABLE IF NOT EXISTS broker_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    broker_type VARCHAR(20) NOT NULL,
    encrypted_api_key TEXT NOT NULL,
    encrypted_api_secret TEXT NOT NULL,
    key_nonce VARCHAR(32) NOT NULL,
    secret_nonce VARCHAR(32) NOT NULL,
    permissions TEXT[] DEFAULT '{"read","trade"}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    label VARCHAR(100),
    last_verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_broker CHECK (broker_type IN ('binance', 'ibkr', 'oanda', 'paper')),
    UNIQUE(user_id, broker_type, label)
);

CREATE INDEX IF NOT EXISTS idx_broker_connections_user ON broker_connections(user_id);

ALTER TABLE broker_connections ENABLE ROW LEVEL SECURITY;
