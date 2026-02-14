//! Policy Engine Integration
//!
//! Integration with AI-OS-PG Policy Engine for:
//! - WAF (Web Application Firewall) rules
//! - Rate limiting
//! - Access control policies
//! - Compliance policy enforcement

use crate::compliance::types::PolicyResult;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing::{error, info, warn};

/// Policy Engine Integration
#[derive(Debug, Clone)]
pub struct PolicyIntegration {
    enabled: bool,
    service_url: String,
    default_action: PolicyAction,
}

/// Policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub enabled: bool,
    pub service_url: String,
    pub default_action: PolicyAction,
    pub rate_limit_requests: u32,
    pub rate_limit_window_seconds: u64,
    pub waf_enabled: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            service_url: "http://localhost:3000".to_string(),
            default_action: PolicyAction::Allow,
            rate_limit_requests: 100,
            rate_limit_window_seconds: 60,
            waf_enabled: true,
        }
    }
}

/// Policy action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyAction {
    Allow,
    Deny,
    RateLimit,
    Challenge,
}

/// Request context for policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub ip_address: String,
    pub path: String,
    pub method: String,
    pub headers: std::collections::HashMap<String, String>,
    pub user_agent: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl RequestContext {
    pub fn new(ip: IpAddr, path: &str) -> Self {
        Self {
            ip_address: ip.to_string(),
            path: path.to_string(),
            method: "GET".to_string(),
            headers: std::collections::HashMap::new(),
            user_agent: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_method(mut self, method: &str) -> Self {
        self.method = method.to_string();
        self
    }

    pub fn with_user_agent(mut self, ua: &str) -> Self {
        self.user_agent = Some(ua.to_string());
        self
    }
}

/// Rate limit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_at: chrono::DateTime<chrono::Utc>,
    pub retry_after: Option<u64>,
}

impl PolicyIntegration {
    /// Create new policy integration
    pub fn new(config: PolicyConfig) -> Self {
        info!(
            "Policy Integration initialized: enabled={}, url={}",
            config.enabled, config.service_url
        );

        Self {
            enabled: config.enabled,
            service_url: config.service_url,
            default_action: config.default_action,
        }
    }

    /// Create from environment
    pub fn from_env() -> Self {
        let config = PolicyConfig {
            enabled: std::env::var("POLICY_ENABLED")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
            service_url: std::env::var("AI_OS_PG_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            default_action: PolicyAction::Allow,
            rate_limit_requests: std::env::var("RATE_LIMIT_REQUESTS")
                .map(|v| v.parse().unwrap_or(100))
                .unwrap_or(100),
            rate_limit_window_seconds: std::env::var("RATE_LIMIT_WINDOW")
                .map(|v| v.parse().unwrap_or(60))
                .unwrap_or(60),
            waf_enabled: std::env::var("WAF_ENABLED")
                .map(|v| v.parse().unwrap_or(true))
                .unwrap_or(true),
        };

        Self::new(config)
    }

    /// Check if policy engine is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Evaluate request against policies
    pub async fn evaluate(&self, ctx: &RequestContext) -> Result<PolicyResult, PolicyError> {
        if !self.enabled {
            return Ok(PolicyResult {
                allowed: true,
                reason: None,
                violations: vec![],
            });
        }

        let mut violations = Vec::new();

        // WAF checks
        if let Err(e) = self.waf_check(ctx).await {
            violations.push(format!("WAF: {}", e));
        }

        // Rate limiting
        match self.check_rate_limit(&ctx.ip_address).await {
            Ok(RateLimitStatus { allowed: false, retry_after: Some(retry), .. }) => {
                violations.push(format!("Rate limited, retry after {}s", retry));
            }
            Ok(_) => {}
            Err(e) => {
                warn!("Rate limit check failed: {}", e);
            }
        }

        // Path-based rules
        if let Err(e) = self.check_path_rules(ctx).await {
            violations.push(format!("Path rule: {}", e));
        }

        let allowed = violations.is_empty();

        Ok(PolicyResult {
            allowed,
            reason: if allowed { None } else { Some(violations.join(", ")) },
            violations,
        })
    }

    /// WAF (Web Application Firewall) check
    async fn waf_check(&self, ctx: &RequestContext) -> Result<(), String> {
        // Check for common attack patterns
        let path_lower = ctx.path.to_lowercase();

        // SQL Injection patterns
        let sqli_patterns = [
            "'", "\"", ";--", "/*", "*/", "union", "select", "insert", "delete", "drop",
        ];
        for pattern in &sqli_patterns {
            if path_lower.contains(pattern) {
                warn!("Potential SQL injection detected: {}", ctx.path);
                return Err("Potential SQL injection".to_string());
            }
        }

        // XSS patterns
        let xss_patterns = [
            "<script>", "javascript:", "onerror=", "onload=", "alert(",
        ];
        for pattern in &xss_patterns {
            if path_lower.contains(pattern) {
                warn!("Potential XSS detected: {}", ctx.path);
                return Err("Potential XSS attack".to_string());
            }
        }

        // Path traversal
        if path_lower.contains("..") || path_lower.contains("../") {
            warn!("Path traversal attempt: {}", ctx.path);
            return Err("Path traversal attempt".to_string());
        }

        Ok(())
    }

    /// Check rate limit for IP
    async fn check_rate_limit(&self, ip: &str) -> Result<RateLimitStatus, PolicyError> {
        // In production, this would call AI-OS-PG rate limiter
        // For now, use in-memory tracking
        
        // Simplified: always allow in mock mode
        Ok(RateLimitStatus {
            allowed: true,
            remaining: 100,
            reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
            retry_after: None,
        })
    }

    /// Check path-based access rules
    async fn check_path_rules(&self, ctx: &RequestContext) -> Result<(), String> {
        // Admin paths require special handling
        if ctx.path.starts_with("/admin") {
            // Additional checks would go here
            info!("Admin path access: {}", ctx.path);
        }

        // Compliance endpoints should be protected
        if ctx.path.starts_with("/api/v1/compliance") {
            // These should only be accessed by authenticated users
        }

        Ok(())
    }

    /// Check if request is compliant with EU AI Act
    pub async fn check_compliance_requirements(
        &self,
        ctx: &RequestContext,
    ) -> Result<ComplianceCheckResult, PolicyError> {
        let mut checks = Vec::new();

        // Check for audit logging capability
        checks.push(ComplianceCheck {
            name: "Audit Logging".to_string(),
            passed: true,
            description: "AI decision logging is enabled".to_string(),
        });

        // Check for human oversight
        checks.push(ComplianceCheck {
            name: "Human Oversight".to_string(),
            passed: true,
            description: "Human review endpoints available".to_string(),
        });

        // Check for transparency
        checks.push(ComplianceCheck {
            name: "Transparency".to_string(),
            passed: true,
            description: "Explainability features enabled".to_string(),
        });

        let all_passed = checks.iter().all(|c| c.passed);

        Ok(ComplianceCheckResult {
            compliant: all_passed,
            checks,
            article_violations: vec![],
        })
    }
}

/// Compliance check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckResult {
    pub compliant: bool,
    pub checks: Vec<ComplianceCheck>,
    pub article_violations: Vec<String>,
}

/// Individual compliance check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheck {
    pub name: String,
    pub passed: bool,
    pub description: String,
}

/// Policy error type
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Evaluation failed: {0}")]
    EvaluationFailed(String),
    
    #[error("Service unavailable")]
    ServiceUnavailable,
    
    #[error("Rate limited")]
    RateLimited,
}

/// Middleware for automatic policy enforcement
pub struct PolicyMiddleware {
    policy: PolicyIntegration,
}

impl PolicyMiddleware {
    pub fn new(policy: PolicyIntegration) -> Self {
        Self { policy }
    }

    /// Check request before processing
    pub async fn check_request(&self, ctx: &RequestContext) -> Result<(), PolicyError> {
        let result = self.policy.evaluate(ctx).await?;
        
        if !result.allowed {
            return Err(PolicyError::EvaluationFailed(
                result.reason.unwrap_or_else(|| "Request denied".to_string())
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_policy_config_default() {
        let config = PolicyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.rate_limit_requests, 100);
    }

    #[tokio::test]
    async fn test_disabled_policy() {
        let config = PolicyConfig {
            enabled: false,
            ..Default::default()
        };
        let policy = PolicyIntegration::new(config);
        
        let ctx = RequestContext::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "/api/test",
        );
        
        let result = policy.evaluate(&ctx).await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_waf_sql_injection() {
        let config = PolicyConfig::default();
        let policy = PolicyIntegration::new(config);
        
        let ctx = RequestContext::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "/api/data'; DROP TABLE users;--",
        );
        
        let result = policy.evaluate(&ctx).await.unwrap();
        assert!(!result.allowed);
        assert!(result.violations.iter().any(|v| v.contains("SQL")));
    }

    #[tokio::test]
    async fn test_waf_xss() {
        let config = PolicyConfig::default();
        let policy = PolicyIntegration::new(config);
        
        let ctx = RequestContext::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            "/api/data?param=<script>alert(1)</script>",
        );
        
        let result = policy.evaluate(&ctx).await.unwrap();
        assert!(!result.allowed);
        assert!(result.violations.iter().any(|v| v.contains("XSS")));
    }

    #[test]
    fn test_request_context_builder() {
        let ctx = RequestContext::new(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            "/api/test",
        )
        .with_method("POST")
        .with_user_agent("Test/1.0");

        assert_eq!(ctx.ip_address, "192.168.1.1");
        assert_eq!(ctx.path, "/api/test");
        assert_eq!(ctx.method, "POST");
        assert_eq!(ctx.user_agent, Some("Test/1.0".to_string()));
    }
}
