-- Wave 4 Task 23: KYC/AML Verification via Sumsub
-- Creates the kyc_verifications table for tracking user identity verification status.

CREATE TABLE IF NOT EXISTS kyc_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL UNIQUE,
    sumsub_applicant_id VARCHAR(100),
    status VARCHAR(20) NOT NULL DEFAULT 'not_started',
    level VARCHAR(20) NOT NULL DEFAULT 'basic',
    rejection_reason TEXT,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_status CHECK (status IN ('not_started','pending','approved','rejected','retry'))
);

CREATE INDEX IF NOT EXISTS idx_kyc_user ON kyc_verifications(user_id);
