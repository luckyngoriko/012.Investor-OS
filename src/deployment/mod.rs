//! Sprint 35: Deployment & Production Module
//!
//! Health checks, readiness probes, and deployment utilities

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Health check manager
#[derive(Debug)]
pub struct HealthCheckManager {
    checks: HashMap<String, Box<dyn HealthCheck>>,
    start_time: Instant,
    version: String,
}

/// Health check trait
pub trait HealthCheck: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn check(&self) -> CheckResult;
}

/// Check result
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub status: CheckStatus,
    pub message: Option<String>,
    pub response_time_ms: u64,
}

/// Check status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
}

impl CheckStatus {
    pub fn to_string(&self) -> String {
        match self {
            CheckStatus::Pass => "pass".to_string(),
            CheckStatus::Fail => "fail".to_string(),
            CheckStatus::Warn => "warn".to_string(),
        }
    }
}

/// Overall health status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub timestamp: DateTime<Utc>,
    pub checks: HashMap<String, CheckResult>,
}

/// Readiness check
#[derive(Debug)]
pub struct ReadinessCheck {
    dependencies: Vec<String>,
}

/// Liveness check
#[derive(Debug)]
pub struct LivenessCheck;

impl HealthCheckManager {
    /// Create new health check manager
    pub fn new(version: &str) -> Self {
        Self {
            checks: HashMap::new(),
            start_time: Instant::now(),
            version: version.to_string(),
        }
    }
    
    /// Register a health check
    pub fn register_check(&mut self, check: Box<dyn HealthCheck>) {
        let name = check.name().to_string();
        self.checks.insert(name, check);
    }
    
    /// Run all health checks
    pub fn check_health(&self) -> HealthStatus {
        let mut checks = HashMap::new();
        let mut all_pass = true;
        let mut has_warn = false;
        
        for (name, check) in &self.checks {
            let start = Instant::now();
            let result = check.check();
            let elapsed = start.elapsed().as_millis() as u64;
            
            let result = CheckResult {
                status: result.status,
                message: result.message,
                response_time_ms: elapsed,
            };
            
            if result.status == CheckStatus::Fail {
                all_pass = false;
            }
            if result.status == CheckStatus::Warn {
                has_warn = true;
            }
            
            checks.insert(name.clone(), result);
        }
        
        let status = if all_pass && !has_warn {
            "healthy"
        } else if all_pass {
            "degraded"
        } else {
            "unhealthy"
        };
        
        HealthStatus {
            status: status.to_string(),
            version: self.version.clone(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            timestamp: Utc::now(),
            checks,
        }
    }
    
    /// Check if service is ready
    pub fn is_ready(&self) -> bool {
        self.check_health().status == "healthy"
    }
    
    /// Check if service is alive
    pub fn is_alive(&self) -> bool {
        // Liveness check - just verify the service hasn't hung
        self.start_time.elapsed() < Duration::from_secs(300) || !self.checks.is_empty()
    }
    
    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Get version
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl Default for HealthCheckManager {
    fn default() -> Self {
        Self::new("0.1.0")
    }
}

impl ReadinessCheck {
    /// Create new readiness check
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }
    
    /// Add dependency
    pub fn add_dependency(&mut self, name: &str) {
        self.dependencies.push(name.to_string());
    }
}

impl Default for ReadinessCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl LivenessCheck {
    /// Create new liveness check
    pub fn new() -> Self {
        Self
    }
}

impl Default for LivenessCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// Deployment configuration
#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub environment: Environment,
    pub log_level: String,
    pub paper_trading: bool,
    pub rate_limit_enabled: bool,
    pub circuit_breaker_enabled: bool,
}

/// Deployment environment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Some(Environment::Development),
            "staging" => Some(Environment::Staging),
            "prod" | "production" => Some(Environment::Production),
            _ => None,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Staging => "staging",
            Environment::Production => "production",
        }
    }
    
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            environment: Environment::Development,
            log_level: "info".to_string(),
            paper_trading: true,
            rate_limit_enabled: false,
            circuit_breaker_enabled: false,
        }
    }
}

/// Graceful shutdown handler
#[derive(Debug)]
pub struct ShutdownHandler {
    shutdown_requested: bool,
    timeout_seconds: u64,
}

impl ShutdownHandler {
    /// Create new shutdown handler
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            shutdown_requested: false,
            timeout_seconds,
        }
    }
    
    /// Request shutdown
    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
    }
    
    /// Check if shutdown requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }
    
    /// Get timeout
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }
}

impl Default for ShutdownHandler {
    fn default() -> Self {
        Self::new(30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockHealthCheck {
        name: String,
        should_pass: bool,
    }

    impl HealthCheck for MockHealthCheck {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn check(&self) -> CheckResult {
            CheckResult {
                status: if self.should_pass { CheckStatus::Pass } else { CheckStatus::Fail },
                message: None,
                response_time_ms: 10,
            }
        }
    }

    #[test]
    fn test_health_check_manager_creation() {
        let manager = HealthCheckManager::new("1.0.0");
        assert_eq!(manager.version(), "1.0.0");
        assert!(manager.uptime().as_secs() < 1);
    }

    #[test]
    fn test_health_check_registration() {
        let mut manager = HealthCheckManager::new("1.0.0");
        
        let check = MockHealthCheck {
            name: "test".to_string(),
            should_pass: true,
        };
        
        manager.register_check(Box::new(check));
        
        let health = manager.check_health();
        assert_eq!(health.checks.len(), 1);
    }

    #[test]
    fn test_healthy_status() {
        let mut manager = HealthCheckManager::new("1.0.0");
        
        manager.register_check(Box::new(MockHealthCheck {
            name: "db".to_string(),
            should_pass: true,
        }));
        
        let health = manager.check_health();
        assert_eq!(health.status, "healthy");
        assert!(manager.is_ready());
    }

    #[test]
    fn test_unhealthy_status() {
        let mut manager = HealthCheckManager::new("1.0.0");
        
        manager.register_check(Box::new(MockHealthCheck {
            name: "db".to_string(),
            should_pass: false,
        }));
        
        let health = manager.check_health();
        assert_eq!(health.status, "unhealthy");
        assert!(!manager.is_ready());
    }

    #[test]
    fn test_degraded_status() {
        let mut manager = HealthCheckManager::new("1.0.0");
        
        #[derive(Debug)]
        struct WarningCheck;
        
        impl HealthCheck for WarningCheck {
            fn name(&self) -> &str {
                "warning_check"
            }
            
            fn check(&self) -> CheckResult {
                CheckResult {
                    status: CheckStatus::Warn,
                    message: Some("Slow response".to_string()),
                    response_time_ms: 1000,
                }
            }
        }
        
        manager.register_check(Box::new(WarningCheck));
        
        let health = manager.check_health();
        assert_eq!(health.status, "degraded");
    }

    #[test]
    fn test_environment_from_str() {
        assert_eq!(Environment::from_str("dev"), Some(Environment::Development));
        assert_eq!(Environment::from_str("development"), Some(Environment::Development));
        assert_eq!(Environment::from_str("staging"), Some(Environment::Staging));
        assert_eq!(Environment::from_str("prod"), Some(Environment::Production));
        assert_eq!(Environment::from_str("production"), Some(Environment::Production));
        assert_eq!(Environment::from_str("unknown"), None);
    }

    #[test]
    fn test_environment_name() {
        assert_eq!(Environment::Development.name(), "development");
        assert_eq!(Environment::Staging.name(), "staging");
        assert_eq!(Environment::Production.name(), "production");
    }

    #[test]
    fn test_is_production() {
        assert!(!Environment::Development.is_production());
        assert!(!Environment::Staging.is_production());
        assert!(Environment::Production.is_production());
    }

    #[test]
    fn test_shutdown_handler() {
        let mut handler = ShutdownHandler::new(60);
        
        assert!(!handler.is_shutdown_requested());
        assert_eq!(handler.timeout(), Duration::from_secs(60));
        
        handler.request_shutdown();
        assert!(handler.is_shutdown_requested());
    }

    #[test]
    fn test_check_status_to_string() {
        assert_eq!(CheckStatus::Pass.to_string(), "pass");
        assert_eq!(CheckStatus::Fail.to_string(), "fail");
        assert_eq!(CheckStatus::Warn.to_string(), "warn");
    }

    #[test]
    fn test_readiness_check() {
        let mut check = ReadinessCheck::new();
        check.add_dependency("database");
        check.add_dependency("redis");
        
        assert_eq!(check.dependencies.len(), 2);
    }

    #[test]
    fn test_liveness_check() {
        let check = LivenessCheck::new();
        // Just verify it can be created
        let _ = format!("{:?}", check);
    }

    #[test]
    fn test_deployment_config_defaults() {
        let config = DeploymentConfig::default();
        
        assert_eq!(config.environment, Environment::Development);
        assert_eq!(config.log_level, "info");
        assert!(config.paper_trading);
        assert!(!config.rate_limit_enabled);
        assert!(!config.circuit_breaker_enabled);
    }

    #[test]
    fn test_health_status_fields() {
        let mut checks = HashMap::new();
        checks.insert("test".to_string(), CheckResult {
            status: CheckStatus::Pass,
            message: Some("OK".to_string()),
            response_time_ms: 50,
        });
        
        let status = HealthStatus {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            timestamp: Utc::now(),
            checks,
        };
        
        assert_eq!(status.version, "1.0.0");
        assert_eq!(status.uptime_seconds, 3600);
        assert_eq!(status.checks.len(), 1);
    }
}
