//! Compliance Types
//!
//! Core types for EU AI Act and GDPR compliance

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Compliance score (0-100)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComplianceScore(pub u8);

impl ComplianceScore {
    /// Create new compliance score
    pub fn new(score: u8) -> Self {
        Self(score.min(100))
    }

    /// Check if score meets minimum threshold (70%)
    pub fn is_acceptable(&self) -> bool {
        self.0 >= 70
    }

    /// Get score value
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl Default for ComplianceScore {
    fn default() -> Self {
        Self(100)
    }
}

/// Compliance status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    AtRisk,
    NonCompliant,
    PendingReview,
}

/// AI System registration info (EU AI Act requirement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISystemRegistration {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub provider: String,
    pub risk_level: RiskLevel,
    pub intended_use: String,
    pub training_data_info: String,
    pub registered_at: DateTime<Utc>,
}

/// Risk level per EU AI Act
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Minimal,
    Limited,
    High,
    Unacceptable,
}

impl RiskLevel {
    /// Check if human oversight is required
    pub fn requires_human_oversight(&self) -> bool {
        matches!(self, RiskLevel::High | RiskLevel::Unacceptable)
    }
}

/// AI Decision log entry (Article 12 requirement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIDecisionLog {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub system_id: Uuid,
    pub decision_type: DecisionType,
    pub input_data_hash: String,
    pub output_data: serde_json::Value,
    pub confidence: f64,
    pub explanation: String,
    pub human_reviewed: bool,
    pub human_decision: Option<HumanDecision>,
}

/// Type of AI decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionType {
    TradingSignal,
    RiskAssessment,
    PortfolioRebalancing,
    MarketRegimeDetection,
    Other,
}

/// Human oversight decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanDecision {
    pub verifier_id: Uuid,
    pub decision: HumanOversightDecision,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Human oversight outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumanOversightDecision {
    Approved,
    Rejected,
    Modified,
    Escalated,
}

/// GDPR data export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExport {
    pub user_id: Uuid,
    pub exported_at: DateTime<Utc>,
    pub data: serde_json::Value,
    pub format: ExportFormat,
}

/// Export format for GDPR portability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Xml,
    Csv,
}

/// GDPR deletion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRequest {
    pub user_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub scheduled_deletion: DateTime<Utc>,
    pub status: DeletionStatus,
}

/// Deletion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeletionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub generated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub overall_score: ComplianceScore,
    pub status: ComplianceStatus,
    pub findings: Vec<ComplianceFinding>,
    pub recommendations: Vec<String>,
}

/// Compliance finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFinding {
    pub id: Uuid,
    pub severity: FindingSeverity,
    pub category: ComplianceCategory,
    pub description: String,
    pub article_reference: String,
    pub remediation: String,
}

/// Finding severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Compliance category per EU AI Act
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceCategory {
    Transparency,   // Article 13
    HumanOversight, // Article 14
    Accuracy,       // Article 15
    Robustness,     // Article 16
    Logging,        // Article 12
    DataGovernance, // Article 10
}

/// DLP scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpScanResult {
    pub has_violations: bool,
    pub findings: Vec<DlpFinding>,
    pub sanitized_content: Option<String>,
}

/// DLP finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpFinding {
    pub finding_type: String,
    pub position: (usize, usize),
    pub severity: FindingSeverity,
    pub description: String,
}

/// Policy evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub violations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_score() {
        let score = ComplianceScore::new(85);
        assert_eq!(score.value(), 85);
        assert!(score.is_acceptable());

        let low_score = ComplianceScore::new(50);
        assert!(!low_score.is_acceptable());
    }

    #[test]
    fn test_risk_level() {
        assert!(!RiskLevel::Minimal.requires_human_oversight());
        assert!(!RiskLevel::Limited.requires_human_oversight());
        assert!(RiskLevel::High.requires_human_oversight());
        assert!(RiskLevel::Unacceptable.requires_human_oversight());
    }

    #[test]
    fn test_score_clamping() {
        let score = ComplianceScore::new(150);
        assert_eq!(score.value(), 100);
    }
}
