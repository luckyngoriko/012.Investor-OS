-- Wave 3 Task 17: Copy Trading Marketplace
-- Strategy listings for the marketplace with public browsing and subscriptions

CREATE TABLE IF NOT EXISTS strategy_listings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    creator_id UUID NOT NULL REFERENCES auth_users(id),
    strategy_id UUID NOT NULL REFERENCES user_strategies(id),
    name VARCHAR(200) NOT NULL,
    description TEXT,
    performance JSONB DEFAULT '{}',
    price_monthly_usd INTEGER NOT NULL DEFAULT 0,
    subscribers_count INTEGER NOT NULL DEFAULT 0,
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_listings_creator ON strategy_listings(creator_id);
CREATE INDEX IF NOT EXISTS idx_listings_public ON strategy_listings(is_public) WHERE is_public = true;
