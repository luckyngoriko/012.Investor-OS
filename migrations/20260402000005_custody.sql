-- Wave 4 Task 22: Fireblocks MPC Custody
-- custody_wallets + custody_transactions tables with RLS

-- ── Custody Wallets ───────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS custody_wallets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    fireblocks_vault_id TEXT NOT NULL,
    name            TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_custody_wallets_user_id
    ON custody_wallets (user_id);

CREATE INDEX IF NOT EXISTS idx_custody_wallets_fireblocks_vault_id
    ON custody_wallets (fireblocks_vault_id);

-- ── Custody Transactions ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS custody_transactions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id           UUID NOT NULL REFERENCES custody_wallets(id) ON DELETE CASCADE,
    tx_type             TEXT NOT NULL CHECK (tx_type IN ('deposit', 'withdrawal')),
    asset               TEXT NOT NULL,
    amount              NUMERIC NOT NULL,
    fireblocks_tx_id    TEXT,
    status              TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'completed', 'failed', 'cancelled')),
    tx_hash             TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_custody_transactions_wallet_id
    ON custody_transactions (wallet_id);

CREATE INDEX IF NOT EXISTS idx_custody_transactions_status
    ON custody_transactions (status);

CREATE INDEX IF NOT EXISTS idx_custody_transactions_fireblocks_tx_id
    ON custody_transactions (fireblocks_tx_id);

-- ── Row Level Security ────────────────────────────────────────────────

ALTER TABLE custody_wallets ENABLE ROW LEVEL SECURITY;
ALTER TABLE custody_transactions ENABLE ROW LEVEL SECURITY;

-- Wallet owner policy: users can only see their own wallets
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE tablename = 'custody_wallets' AND policyname = 'custody_wallets_owner'
    ) THEN
        CREATE POLICY custody_wallets_owner ON custody_wallets
            USING (user_id = current_setting('app.current_user_id', true)::uuid);
    END IF;
END $$;

-- Transaction owner policy: users can only see transactions for their wallets
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies
        WHERE tablename = 'custody_transactions' AND policyname = 'custody_transactions_owner'
    ) THEN
        CREATE POLICY custody_transactions_owner ON custody_transactions
            USING (wallet_id IN (
                SELECT id FROM custody_wallets
                WHERE user_id = current_setting('app.current_user_id', true)::uuid
            ));
    END IF;
END $$;
