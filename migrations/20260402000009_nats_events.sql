-- NATS Event Logger audit trail
-- Stores every event flowing through the NATS bus for compliance and debugging.

CREATE TABLE IF NOT EXISTS nats_events (
    id BIGSERIAL,
    subject TEXT NOT NULL,
    symbol VARCHAR(20),
    source VARCHAR(50),
    payload_hash VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_nats_events_subject ON nats_events(subject, created_at);
CREATE INDEX IF NOT EXISTS idx_nats_events_time ON nats_events(created_at);

-- Make it a hypertable if TimescaleDB is available
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'timescaledb') THEN
        PERFORM create_hypertable('nats_events', 'created_at', if_not_exists => TRUE);
    END IF;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;
