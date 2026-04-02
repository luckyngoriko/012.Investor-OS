-- Wave 1b Task 7: Subscription tiers for billing & feature gating
-- Stores user subscription state, Stripe identifiers, and tier level.

CREATE TABLE IF NOT EXISTS subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL UNIQUE REFERENCES auth_users(id),
    tier VARCHAR(20) NOT NULL DEFAULT 'free',
    stripe_customer_id VARCHAR(100),
    stripe_subscription_id VARCHAR(100),
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    current_period_end TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_tier CHECK (tier IN ('free', 'pro', 'enterprise')),
    CONSTRAINT valid_status CHECK (status IN ('active', 'past_due', 'canceled', 'trialing'))
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user ON subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe ON subscriptions(stripe_customer_id);
