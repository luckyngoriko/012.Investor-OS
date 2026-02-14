-- Sprint 52: EU AI Act & GDPR Compliance
-- Migration for compliance tables

-- GDPR Deletion Requests (Article 17 - Right to erasure)
CREATE TABLE IF NOT EXISTS gdpr_deletion_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    scheduled_deletion TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    deleted_by UUID,
    verification_code VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT valid_status CHECK (status IN ('pending', 'in_progress', 'completed', 'failed', 'cancelled'))
);

-- Index for efficient lookup by user
CREATE INDEX idx_gdpr_deletion_user_id ON gdpr_deletion_requests(user_id);

-- Index for scheduled deletion processing
CREATE INDEX idx_gdpr_deletion_scheduled ON gdpr_deletion_requests(scheduled_deletion) 
    WHERE status = 'pending';

-- EU AI Act Decision Logs (Article 12 - Logging)
CREATE TABLE IF NOT EXISTS ai_decision_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    system_id UUID NOT NULL,
    decision_type VARCHAR(50) NOT NULL,
    input_data_hash VARCHAR(64) NOT NULL,
    output_data JSONB NOT NULL,
    confidence FLOAT NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    explanation TEXT NOT NULL,
    human_reviewed BOOLEAN NOT NULL DEFAULT FALSE,
    human_decision JSONB,
    user_id UUID,
    session_id UUID,
    ip_address INET,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX idx_ai_decision_system_id ON ai_decision_logs(system_id);
CREATE INDEX idx_ai_decision_timestamp ON ai_decision_logs(timestamp);
CREATE INDEX idx_ai_decision_type ON ai_decision_logs(decision_type);
CREATE INDEX idx_ai_decision_human_reviewed ON ai_decision_logs(human_reviewed) 
    WHERE human_reviewed = FALSE;

-- Composite index for compliance queries
CREATE INDEX idx_ai_decision_compliance ON ai_decision_logs(system_id, timestamp, decision_type);

-- Compliance Audit Trail (general purpose)
CREATE TABLE IF NOT EXISTS compliance_audit_trail (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID,
    user_id UUID,
    action VARCHAR(50) NOT NULL,
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_compliance_audit_timestamp ON compliance_audit_trail(timestamp);
CREATE INDEX idx_compliance_audit_entity ON compliance_audit_trail(entity_type, entity_id);
CREATE INDEX idx_compliance_audit_user ON compliance_audit_trail(user_id);

-- Compliance Score Tracking
CREATE TABLE IF NOT EXISTS compliance_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    system_id UUID NOT NULL,
    organization_id UUID NOT NULL,
    score INTEGER NOT NULL CHECK (score >= 0 AND score <= 100),
    status VARCHAR(20) NOT NULL,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    findings JSONB DEFAULT '[]',
    recommendations JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT valid_status CHECK (status IN ('compliant', 'at_risk', 'non_compliant', 'pending_review'))
);

CREATE INDEX idx_compliance_score_system ON compliance_scores(system_id);
CREATE INDEX idx_compliance_score_org ON compliance_scores(organization_id);
CREATE INDEX idx_compliance_score_calculated ON compliance_scores(calculated_at);

-- GDPR Data Export Log (track data portability requests)
CREATE TABLE IF NOT EXISTS gdpr_data_exports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    format VARCHAR(10) NOT NULL,
    file_size_bytes BIGINT,
    file_hash VARCHAR(64),
    download_url TEXT,
    expires_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT valid_format CHECK (format IN ('json', 'xml', 'csv')),
    CONSTRAINT valid_status CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'expired'))
);

CREATE INDEX idx_gdpr_export_user_id ON gdpr_data_exports(user_id);
CREATE INDEX idx_gdpr_export_status ON gdpr_data_exports(status);

-- Human Oversight Decisions (EU AI Act Article 14)
CREATE TABLE IF NOT EXISTS human_oversight_decisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    decision_log_id UUID NOT NULL REFERENCES ai_decision_logs(id) ON DELETE CASCADE,
    verifier_id UUID NOT NULL,
    decision VARCHAR(20) NOT NULL,
    reason TEXT NOT NULL,
    decided_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    verification_method VARCHAR(50),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT valid_decision CHECK (decision IN ('approved', 'rejected', 'modified', 'escalated'))
);

CREATE INDEX idx_human_oversight_log ON human_oversight_decisions(decision_log_id);
CREATE INDEX idx_human_oversight_verifier ON human_oversight_decisions(verifier_id);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for updated_at
CREATE TRIGGER update_gdpr_deletion_requests_updated_at 
    BEFORE UPDATE ON gdpr_deletion_requests 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_ai_decision_logs_updated_at 
    BEFORE UPDATE ON ai_decision_logs 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_compliance_scores_updated_at 
    BEFORE UPDATE ON compliance_scores 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_gdpr_data_exports_updated_at 
    BEFORE UPDATE ON gdpr_data_exports 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Row Level Security (RLS) policies for GDPR compliance

-- Enable RLS on sensitive tables
ALTER TABLE gdpr_deletion_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE gdpr_data_exports ENABLE ROW LEVEL SECURITY;
ALTER TABLE ai_decision_logs ENABLE ROW LEVEL SECURITY;

-- Policy: Users can only see their own deletion requests
CREATE POLICY gdpr_deletion_user_isolation ON gdpr_deletion_requests
    FOR ALL
    USING (user_id = current_setting('app.current_user_id')::UUID);

-- Policy: Users can only see their own data exports
CREATE POLICY gdpr_export_user_isolation ON gdpr_data_exports
    FOR ALL
    USING (user_id = current_setting('app.current_user_id')::UUID);

-- Policy: Users can only see their own decision logs
CREATE POLICY ai_decision_user_isolation ON ai_decision_logs
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id')::UUID);

-- Comments for documentation
COMMENT ON TABLE gdpr_deletion_requests IS 'GDPR Article 17 - Right to erasure requests';
COMMENT ON TABLE ai_decision_logs IS 'EU AI Act Article 12 - AI system decision logging';
COMMENT ON TABLE compliance_audit_trail IS 'General compliance audit trail for all sensitive operations';
COMMENT ON TABLE compliance_scores IS 'EU AI Act compliance scoring per AI system';
COMMENT ON TABLE gdpr_data_exports IS 'GDPR Article 20 - Data portability requests';
COMMENT ON TABLE human_oversight_decisions IS 'EU AI Act Article 14 - Human oversight decisions';
