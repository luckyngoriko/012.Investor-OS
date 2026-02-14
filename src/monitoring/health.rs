//! Health Monitoring
//!
//! System health checks and monitoring

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Health monitor
#[derive(Debug)]
pub struct HealthMonitor {
    checks: Vec<Box<dyn HealthCheck>>,
    last_check_time: Option<DateTime<Utc>>,
    check_interval_seconds: u64,
    last_status: HealthStatus,
    check_results: HashMap<String, CheckResult>,
}

/// Health check trait
pub trait HealthCheck: std::fmt::Debug {
    fn name(&self) -> &str;
    fn check(&self) -> CheckResult;
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }
    
    pub fn priority(&self) -> u8 {
        match self {
            HealthStatus::Healthy => 0,
            HealthStatus::Degraded => 1,
            HealthStatus::Unhealthy => 2,
            HealthStatus::Unknown => 3,
        }
    }
}

/// Individual check result
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub response_time_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<HashMap<String, String>>,
}

impl CheckResult {
    pub fn healthy(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: HealthStatus::Healthy,
            message: "OK".to_string(),
            response_time_ms: 0,
            timestamp: Utc::now(),
            metadata: None,
        }
    }
    
    pub fn unhealthy(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: HealthStatus::Unhealthy,
            message: message.to_string(),
            response_time_ms: 0,
            timestamp: Utc::now(),
            metadata: None,
        }
    }
    
    pub fn degraded(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: HealthStatus::Degraded,
            message: message.to_string(),
            response_time_ms: 0,
            timestamp: Utc::now(),
            metadata: None,
        }
    }
    
    pub fn with_response_time(mut self, ms: u64) -> Self {
        self.response_time_ms = ms;
        self
    }
}

/// System health summary
#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub checks: Vec<CheckResult>,
    pub healthy_count: usize,
    pub degraded_count: usize,
    pub unhealthy_count: usize,
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
}

/// Database health check
#[derive(Debug)]
pub struct DatabaseHealthCheck {
    name: String,
    connection_string: String,
}

impl DatabaseHealthCheck {
    pub fn new(name: &str, connection_string: &str) -> Self {
        Self {
            name: name.to_string(),
            connection_string: connection_string.to_string(),
        }
    }
}

impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn check(&self) -> CheckResult {
        // Simplified check - in real implementation would actually test connection
        CheckResult::healthy(&self.name)
            .with_response_time(5)
    }
}

/// API health check
#[derive(Debug)]
pub struct ApiHealthCheck {
    name: String,
    endpoint: String,
    timeout_ms: u64,
}

impl ApiHealthCheck {
    pub fn new(name: &str, endpoint: &str) -> Self {
        Self {
            name: name.to_string(),
            endpoint: endpoint.to_string(),
            timeout_ms: 5000,
        }
    }
}

impl HealthCheck for ApiHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn check(&self) -> CheckResult {
        // Simplified check
        CheckResult::healthy(&self.name)
            .with_response_time(25)
    }
}

/// Memory health check
#[derive(Debug)]
pub struct MemoryHealthCheck {
    name: String,
    warning_threshold: f32, // Percentage
    critical_threshold: f32,
}

impl MemoryHealthCheck {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            warning_threshold: 80.0,
            critical_threshold: 95.0,
        }
    }
}

impl HealthCheck for MemoryHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn check(&self) -> CheckResult {
        // Simplified - in real impl would check actual memory
        let usage = 45.0; // 45% used
        
        if usage > self.critical_threshold {
            CheckResult::unhealthy(&self.name, &format!("Memory usage critical: {:.1}%", usage))
        } else if usage > self.warning_threshold {
            CheckResult::degraded(&self.name, &format!("Memory usage high: {:.1}%", usage))
        } else {
            CheckResult::healthy(&self.name)
        }
    }
}

/// Disk health check
#[derive(Debug)]
pub struct DiskHealthCheck {
    name: String,
    path: String,
    warning_threshold: f32,
    critical_threshold: f32,
}

impl DiskHealthCheck {
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            warning_threshold: 80.0,
            critical_threshold: 95.0,
        }
    }
}

impl HealthCheck for DiskHealthCheck {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn check(&self) -> CheckResult {
        // Simplified
        CheckResult::healthy(&self.name)
    }
}

impl HealthMonitor {
    /// Create new health monitor
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            last_check_time: None,
            check_interval_seconds: 30,
            last_status: HealthStatus::Unknown,
            check_results: HashMap::new(),
        }
    }
    
    /// Register a health check
    pub fn register_check(&mut self, check: Box<dyn HealthCheck>) {
        self.checks.push(check);
    }
    
    /// Run all health checks
    pub fn check_all(&mut self) -> SystemHealth {
        let mut results = Vec::new();
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;
        
        for check in &self.checks {
            let result = check.check();
            
            match result.status {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Degraded => degraded += 1,
                HealthStatus::Unhealthy => unhealthy += 1,
                _ => {}
            }
            
            self.check_results.insert(result.name.clone(), result.clone());
            results.push(result);
        }
        
        // Determine overall status
        let overall = if unhealthy > 0 {
            HealthStatus::Unhealthy
        } else if degraded > 0 {
            HealthStatus::Degraded
        } else if healthy > 0 {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        };
        
        self.last_status = overall;
        self.last_check_time = Some(Utc::now());
        
        SystemHealth {
            overall_status: overall,
            checks: results,
            healthy_count: healthy,
            degraded_count: degraded,
            unhealthy_count: unhealthy,
            timestamp: Utc::now(),
            uptime_seconds: 0, // Would track actual uptime
        }
    }
    
    /// Get last check status
    pub fn last_status(&self) -> HealthStatus {
        self.last_status
    }
    
    /// Get check result
    pub fn get_result(&self, name: &str) -> Option<&CheckResult> {
        self.check_results.get(name)
    }
    
    /// Get all results
    pub fn get_all_results(&self) -> &HashMap<String, CheckResult> {
        &self.check_results
    }
    
    /// Set check interval
    pub fn set_interval(&mut self, seconds: u64) {
        self.check_interval_seconds = seconds.max(5);
    }
    
    /// Check if should run checks
    pub fn should_check(&self) -> bool {
        if let Some(last) = self.last_check_time {
            let elapsed = (Utc::now() - last).num_seconds() as u64;
            elapsed >= self.check_interval_seconds
        } else {
            true
        }
    }
    
    /// Create default health checks
    pub fn create_default_checks(&mut self) {
        self.register_check(Box::new(DatabaseHealthCheck::new(
            "PostgreSQL",
            "postgresql://localhost/investor_os"
        )));
        
        self.register_check(Box::new(ApiHealthCheck::new(
            "REST API",
            "http://localhost:3000/health"
        )));
        
        self.register_check(Box::new(MemoryHealthCheck::new("Memory")));
        
        self.register_check(Box::new(DiskHealthCheck::new(
            "Disk",
            "/data"
        )));
    }
    
    /// Get check count
    pub fn check_count(&self) -> usize {
        self.checks.len()
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Degraded.is_healthy());
        assert!(!HealthStatus::Unhealthy.is_healthy());
        
        assert_eq!(HealthStatus::Healthy.priority(), 0);
        assert_eq!(HealthStatus::Unhealthy.priority(), 2);
    }

    #[test]
    fn test_check_result() {
        let healthy = CheckResult::healthy("test");
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert_eq!(healthy.message, "OK");
        
        let unhealthy = CheckResult::unhealthy("test", "Failed");
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.message, "Failed");
        
        let with_time = CheckResult::healthy("test").with_response_time(100);
        assert_eq!(with_time.response_time_ms, 100);
    }

    #[test]
    fn test_monitor_creation() {
        let monitor = HealthMonitor::new();
        assert_eq!(monitor.check_count(), 0);
        assert_eq!(monitor.last_status(), HealthStatus::Unknown);
    }

    #[test]
    fn test_register_check() {
        let mut monitor = HealthMonitor::new();
        
        monitor.register_check(Box::new(DatabaseHealthCheck::new("DB", "conn")));
        assert_eq!(monitor.check_count(), 1);
    }

    #[test]
    fn test_check_all() {
        let mut monitor = HealthMonitor::new();
        monitor.create_default_checks();
        
        let health = monitor.check_all();
        
        assert!(!health.checks.is_empty());
        assert!(health.healthy_count > 0);
        assert!(health.timestamp <= Utc::now());
    }

    #[test]
    fn test_should_check() {
        let mut monitor = HealthMonitor::new();
        
        // First check should run
        assert!(monitor.should_check());
        
        // After check, should not run immediately
        monitor.check_all();
        assert!(!monitor.should_check());
    }

    #[test]
    fn test_get_result() {
        let mut monitor = HealthMonitor::new();
        monitor.register_check(Box::new(DatabaseHealthCheck::new("TestDB", "conn")));
        
        monitor.check_all();
        
        let result = monitor.get_result("TestDB");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "TestDB");
    }

    #[test]
    fn test_system_health_counts() {
        let health = SystemHealth {
            overall_status: HealthStatus::Healthy,
            checks: vec![],
            healthy_count: 3,
            degraded_count: 1,
            unhealthy_count: 0,
            timestamp: Utc::now(),
            uptime_seconds: 3600,
        };
        
        assert_eq!(health.healthy_count, 3);
        assert_eq!(health.degraded_count, 1);
        assert_eq!(health.unhealthy_count, 0);
    }

    #[test]
    fn test_memory_check() {
        let check = MemoryHealthCheck::new("Memory");
        let result = check.check();
        
        assert_eq!(result.name, "Memory");
        // Should be healthy with mocked 45% usage
        assert_eq!(result.status, HealthStatus::Healthy);
    }
}
