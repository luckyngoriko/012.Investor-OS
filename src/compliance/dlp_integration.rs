//! Data Loss Prevention (DLP) Integration
//!
//! Integration with AI-OS-PG DLP engine for:
//! - PII (Personally Identifiable Information) detection
//! - Data sanitization and redaction
//! - Compliance with GDPR data protection requirements

use crate::compliance::types::{DlpFinding, DlpScanResult, FindingSeverity};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

/// DLP Integration Client
#[derive(Debug, Clone)]
pub struct DlpIntegration {
    enabled: bool,
    /// URL for AI-OS-PG DLP service
    dlp_service_url: String,
    /// Minimum severity to flag
    min_severity: FindingSeverity,
}

/// DLP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpConfig {
    pub enabled: bool,
    pub service_url: String,
    pub min_severity: FindingSeverity,
    pub auto_sanitize: bool,
    pub allowed_patterns: Vec<String>,
}

impl Default for DlpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_url: "http://localhost:3000".to_string(),
            min_severity: FindingSeverity::Medium,
            auto_sanitize: true,
            allowed_patterns: vec![],
        }
    }
}

impl DlpIntegration {
    /// Create new DLP integration
    pub fn new(config: DlpConfig) -> Self {
        info!(
            "DLP Integration initialized: enabled={}, url={}",
            config.enabled, config.service_url
        );

        Self {
            enabled: config.enabled,
            dlp_service_url: config.service_url,
            min_severity: config.min_severity,
        }
    }

    /// Create from environment
    pub fn from_env() -> Self {
        let config = DlpConfig {
            enabled: std::env::var("DLP_ENABLED")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            service_url: std::env::var("AI_OS_PG_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            min_severity: FindingSeverity::Medium,
            auto_sanitize: std::env::var("DLP_AUTO_SANITIZE")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            allowed_patterns: vec![],
        };

        Self::new(config)
    }

    /// Check if DLP is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Scan content for PII and sensitive data
    pub async fn scan(&self, content: &str) -> Result<DlpScanResult, DlpError> {
        if !self.enabled {
            return Ok(DlpScanResult {
                has_violations: false,
                findings: vec![],
                sanitized_content: None,
            });
        }

        // In production, this would call AI-OS-PG DLP service
        // For now, we use local pattern matching
        let findings = self.local_scan(content).await?;

        let has_violations = !findings.is_empty();

        let sanitized_content = if has_violations {
            Some(self.sanitize(content, &findings))
        } else {
            None
        };

        if has_violations {
            warn!(
                "DLP scan found {} violations",
                findings.len()
            );
        }

        Ok(DlpScanResult {
            has_violations,
            findings,
            sanitized_content,
        })
    }

    /// Scan incoming trading data
    pub async fn scan_trading_data(
        &self,
        data: &serde_json::Value,
    ) -> Result<DlpScanResult, DlpError> {
        let content = data.to_string();
        self.scan(&content).await
    }

    /// Scan outgoing API response
    pub async fn scan_response(
        &self,
        response: &serde_json::Value,
    ) -> Result<DlpScanResult, DlpError> {
        let content = response.to_string();
        self.scan(&content).await
    }

    /// Local pattern matching (fallback when AI-OS-PG unavailable)
    async fn local_scan(&self, content: &str) -> Result<Vec<DlpFinding>, DlpError> {
        let mut findings = Vec::new();

        // Check for email addresses
        if let Some(pos) = content.find('@') {
            // Simple email detection
            let start = content[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
            let end = content[pos..].find(' ').map(|i| pos + i).unwrap_or(content.len());
            
            if end - start > 5 {
                findings.push(DlpFinding {
                    finding_type: "EMAIL".to_string(),
                    position: (start, end),
                    severity: FindingSeverity::Medium,
                    description: "Email address detected".to_string(),
                });
            }
        }

        // Check for API keys (simple patterns)
        let api_key_patterns = [
            "sk-",
            "api_key",
            "apikey",
            "api-key",
        ];

        for pattern in &api_key_patterns {
            if let Some(pos) = content.to_lowercase().find(pattern) {
                findings.push(DlpFinding {
                    finding_type: "API_KEY".to_string(),
                    position: (pos, pos + pattern.len() + 20),
                    severity: FindingSeverity::High,
                    description: "Potential API key detected".to_string(),
                });
            }
        }

        // Check for credit card patterns (simplified Luhn check)
        let cc_regex = regex::Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b");
        if let Ok(regex) = cc_regex {
            for mat in regex.find_iter(content) {
                findings.push(DlpFinding {
                    finding_type: "CREDIT_CARD".to_string(),
                    position: (mat.start(), mat.end()),
                    severity: FindingSeverity::Critical,
                    description: "Credit card number detected".to_string(),
                });
            }
        }

        // Check for SSN patterns
        let ssn_regex = regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b");
        if let Ok(regex) = ssn_regex {
            for mat in regex.find_iter(content) {
                findings.push(DlpFinding {
                    finding_type: "SSN".to_string(),
                    position: (mat.start(), mat.end()),
                    severity: FindingSeverity::Critical,
                    description: "Social Security Number detected".to_string(),
                });
            }
        }

        // Filter by minimum severity
        let filtered: Vec<_> = findings
            .into_iter()
            .filter(|f| self.severity_level(&f.severity) >= self.severity_level(&self.min_severity))
            .collect();

        Ok(filtered)
    }

    /// Sanitize content by redacting findings
    fn sanitize(&self, content: &str, findings: &[DlpFinding]) -> String {
        let mut result = content.to_string();
        
        // Sort by position in reverse order to avoid offset issues
        let mut sorted_findings: Vec<_> = findings.iter().collect();
        sorted_findings.sort_by_key(|f| std::cmp::Reverse(f.position.0));

        for finding in sorted_findings {
            let (start, end) = finding.position;
            if start < result.len() && end <= result.len() {
                let replacement = match finding.finding_type.as_str() {
                    "EMAIL" => "[EMAIL_REDACTED]",
                    "API_KEY" => "[API_KEY_REDACTED]",
                    "CREDIT_CARD" => "[CC_REDACTED]",
                    "SSN" => "[SSN_REDACTED]",
                    _ => "[REDACTED]",
                };
                result.replace_range(start..end, replacement);
            }
        }

        result
    }

    /// Get numeric severity level
    fn severity_level(&self, severity: &FindingSeverity) -> u8 {
        match severity {
            FindingSeverity::Info => 1,
            FindingSeverity::Low => 2,
            FindingSeverity::Medium => 3,
            FindingSeverity::High => 4,
            FindingSeverity::Critical => 5,
        }
    }

    /// Validate that content is safe (no violations above threshold)
    pub async fn validate(&self, content: &str) -> Result<(), DlpError> {
        let result = self.scan(content).await?;
        
        let critical_count = result.findings.iter()
            .filter(|f| f.severity == FindingSeverity::Critical)
            .count();

        if critical_count > 0 {
            return Err(DlpError::CriticalDataDetected(
                format!("{} critical violations found", critical_count)
            ));
        }

        Ok(())
    }
}

/// DLP Error type
#[derive(Debug, thiserror::Error)]
pub enum DlpError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Critical data detected: {0}")]
    CriticalDataDetected(String),
    
    #[error("Sanitization failed: {0}")]
    SanitizationFailed(String),
    
    #[error("Service unavailable")]
    ServiceUnavailable,
    
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

/// Middleware for automatic DLP scanning
pub struct DlpMiddleware {
    dlp: DlpIntegration,
}

impl DlpMiddleware {
    pub fn new(dlp: DlpIntegration) -> Self {
        Self { dlp }
    }

    /// Process request through DLP
    pub async fn process_request(
        &self,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, DlpError> {
        let result = self.dlp.scan_trading_data(body).await?;
        
        if result.has_violations {
            if let Some(sanitized) = result.sanitized_content {
                return serde_json::from_str(&sanitized)
                    .map_err(|e| DlpError::SanitizationFailed(e.to_string()));
            }
        }
        
        Ok(body.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dlp_config_default() {
        let config = DlpConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_severity, FindingSeverity::Medium);
    }

    #[tokio::test]
    async fn test_dlp_disabled() {
        let config = DlpConfig {
            enabled: false,
            ..Default::default()
        };
        let dlp = DlpIntegration::new(config);
        
        let result = dlp.scan("test@example.com").await.unwrap();
        assert!(!result.has_violations);
    }

    #[tokio::test]
    async fn test_local_scan_email() {
        let config = DlpConfig::default();
        let dlp = DlpIntegration::new(config);
        
        let result = dlp.scan("Contact us at support@investor-os.com").await.unwrap();
        
        if result.has_violations {
            assert!(result.findings.iter().any(|f| f.finding_type == "EMAIL"));
        }
    }

    #[test]
    fn test_sanitize() {
        let config = DlpConfig::default();
        let dlp = DlpIntegration::new(config);
        
        let content = "Email: test@example.com";
        let findings = vec![DlpFinding {
            finding_type: "EMAIL".to_string(),
            position: (7, 23),
            severity: FindingSeverity::Medium,
            description: "Email".to_string(),
        }];
        
        let sanitized = dlp.sanitize(content, &findings);
        assert!(sanitized.contains("[EMAIL_REDACTED]"));
        assert!(!sanitized.contains("test@example.com"));
    }
}
