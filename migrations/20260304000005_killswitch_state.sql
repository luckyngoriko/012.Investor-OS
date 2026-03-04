-- Killswitch persistent state (Sprint 108)
-- Single-row table pattern: only one row with id=1
CREATE TABLE IF NOT EXISTS killswitch_state (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    enabled BOOLEAN NOT NULL DEFAULT false,
    reason TEXT,
    triggered_by UUID REFERENCES auth_users(id),
    triggered_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed the single row
INSERT INTO killswitch_state (id, enabled) VALUES (1, false)
ON CONFLICT (id) DO NOTHING;
