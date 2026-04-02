-- Wave 4 Task 25: MiFID II / MiCA Regulatory Compliance Reports
-- Stores compliance reports for multiple EU regulations

CREATE TABLE IF NOT EXISTS compliance_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID,
    report_type VARCHAR(50) NOT NULL,
    regulation VARCHAR(20) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    data JSONB NOT NULL DEFAULT '{}',
    period_start TIMESTAMPTZ,
    period_end TIMESTAMPTZ,
    submitted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_regulation CHECK (regulation IN ('mifid2', 'mica', 'ai_act', 'gdpr')),
    CONSTRAINT valid_report_status CHECK (status IN ('draft', 'submitted', 'accepted', 'rejected'))
);

CREATE INDEX IF NOT EXISTS idx_compliance_reports_user ON compliance_reports(user_id);
CREATE INDEX IF NOT EXISTS idx_compliance_reports_regulation ON compliance_reports(regulation);
