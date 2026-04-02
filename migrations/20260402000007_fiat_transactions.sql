-- Wave 4 Task 24: Fiat On-Ramp via Stripe Connect
-- Stores fiat deposit/withdrawal transactions with Stripe payment intent tracking.

CREATE TABLE IF NOT EXISTS fiat_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tx_type VARCHAR(20) NOT NULL,
    amount_cents BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    stripe_payment_intent_id VARCHAR(100),
    stripe_payout_id VARCHAR(100),
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_tx_type CHECK (tx_type IN ('deposit', 'withdrawal')),
    CONSTRAINT valid_status CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'refunded'))
);

CREATE INDEX IF NOT EXISTS idx_fiat_tx_user ON fiat_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_fiat_tx_status ON fiat_transactions(status);
