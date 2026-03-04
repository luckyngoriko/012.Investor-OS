//! Sprint 35: Deployment Integration Tests
//!
//! Tests for Kubernetes deployment, health checks, and production readiness

use std::collections::HashMap;
use std::time::Duration;

/// Health check response
#[derive(Debug, Clone)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HashMap<String, CheckStatus>,
}

/// Individual check status
#[derive(Debug, Clone)]
pub struct CheckStatus {
    pub status: String,
    pub message: Option<String>,
    pub response_time_ms: u64,
}

/// Deployment validator
#[derive(Debug)]
pub struct DeploymentValidator {
    min_health_score: f32,
    required_checks: Vec<String>,
}

impl DeploymentValidator {
    pub fn new() -> Self {
        Self {
            min_health_score: 0.95,
            required_checks: vec![
                "database".to_string(),
                "redis".to_string(),
                "api".to_string(),
            ],
        }
    }

    /// Validate health response
    pub fn validate_health(&self, health: &HealthResponse) -> bool {
        // Check overall status
        if health.status != "healthy" {
            return false;
        }

        // Check all required checks pass
        for check_name in &self.required_checks {
            let Some(check) = health.checks.get(check_name) else {
                return false;
            };
            if check.status != "pass" {
                return false;
            }
        }

        true
    }

    /// Calculate health score
    pub fn calculate_score(&self, health: &HealthResponse) -> f32 {
        if health.checks.is_empty() {
            return 0.0;
        }

        let passed = health
            .checks
            .values()
            .filter(|c| c.status == "pass")
            .count();

        passed as f32 / health.checks.len() as f32
    }

    /// Check if deployment meets production criteria
    pub fn is_production_ready(&self, health: &HealthResponse) -> bool {
        let score = self.calculate_score(health);
        score >= self.min_health_score && self.validate_health(health)
    }

    /// Set minimum health score
    pub fn set_min_health_score(&mut self, score: f32) {
        self.min_health_score = score.clamp(0.0, 1.0);
    }
}

impl Default for DeploymentValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Kubernetes resource validator
#[derive(Debug)]
pub struct K8sResourceValidator {
    namespace: String,
}

impl K8sResourceValidator {
    pub fn new(namespace: &str) -> Self {
        Self {
            namespace: namespace.to_string(),
        }
    }

    /// Validate resource limits
    pub fn validate_resource_limits(&self, requests: &ResourceSpec, limits: &ResourceSpec) -> bool {
        // Memory: limits >= requests
        let memory_valid =
            Self::parse_memory(&limits.memory) >= Self::parse_memory(&requests.memory);

        // CPU: limits >= requests
        let cpu_valid = Self::parse_cpu(&limits.cpu) >= Self::parse_cpu(&requests.cpu);

        memory_valid && cpu_valid
    }

    /// Check if resources are production-grade
    pub fn is_production_grade(&self, requests: &ResourceSpec) -> bool {
        // Production should have at least 512Mi memory and 500m CPU
        let memory = Self::parse_memory(&requests.memory);
        let cpu = Self::parse_cpu(&requests.cpu);

        memory >= 512 * 1024 * 1024 && cpu >= 500.0 // 512Mi, 500m
    }

    /// Parse memory string (e.g., "512Mi", "1Gi") to bytes
    fn parse_memory(mem: &str) -> u64 {
        let mem = mem.trim();

        if mem.ends_with("Gi") {
            mem[..mem.len() - 2].parse::<u64>().unwrap_or(0) * 1024 * 1024 * 1024
        } else if mem.ends_with("Mi") {
            mem[..mem.len() - 2].parse::<u64>().unwrap_or(0) * 1024 * 1024
        } else if mem.ends_with("Ki") {
            mem[..mem.len() - 2].parse::<u64>().unwrap_or(0) * 1024
        } else {
            mem.parse::<u64>().unwrap_or(0)
        }
    }

    /// Parse CPU string (e.g., "500m", "1") to millicores
    fn parse_cpu(cpu: &str) -> f32 {
        let cpu = cpu.trim();

        if cpu.ends_with('m') {
            cpu[..cpu.len() - 1].parse::<f32>().unwrap_or(0.0)
        } else {
            cpu.parse::<f32>().unwrap_or(0.0) * 1000.0
        }
    }
}

/// Resource specification
#[derive(Debug, Clone)]
pub struct ResourceSpec {
    pub memory: String,
    pub cpu: String,
}

/// Canary deployment tester
#[derive(Debug)]
pub struct CanaryTester {
    traffic_split_percent: u8,
    min_success_rate: f32,
}

impl CanaryTester {
    pub fn new() -> Self {
        Self {
            traffic_split_percent: 10,
            min_success_rate: 0.99,
        }
    }

    /// Calculate canary traffic percentage
    pub fn calculate_traffic_split(&self, canary_replicas: u32, total_replicas: u32) -> f32 {
        if total_replicas == 0 {
            return 0.0;
        }

        canary_replicas as f32 / total_replicas as f32
    }

    /// Check if canary is ready for promotion
    pub fn can_promote(&self, canary_metrics: &CanaryMetrics) -> bool {
        // Success rate must be above threshold
        if canary_metrics.success_rate < self.min_success_rate {
            return false;
        }

        // Error rate must be very low
        if canary_metrics.error_rate > 0.001 {
            return false;
        }

        // Response time should be reasonable
        if canary_metrics.p99_latency_ms > 1000 {
            return false;
        }

        // Must have enough requests for statistical significance
        if canary_metrics.total_requests < 100 {
            return false;
        }

        true
    }

    /// Set traffic split percentage
    pub fn set_traffic_split(&mut self, percent: u8) {
        self.traffic_split_percent = percent.clamp(0, 100);
    }
}

impl Default for CanaryTester {
    fn default() -> Self {
        Self::new()
    }
}

/// Canary metrics
#[derive(Debug, Clone)]
pub struct CanaryMetrics {
    pub total_requests: u64,
    pub success_rate: f32,
    pub error_rate: f32,
    pub p99_latency_ms: u64,
}

/// Rollback detector
#[derive(Debug)]
pub struct RollbackDetector {
    error_threshold: f32,
    latency_threshold_ms: u64,
}

impl RollbackDetector {
    pub fn new() -> Self {
        Self {
            error_threshold: 0.05,
            latency_threshold_ms: 2000,
        }
    }

    /// Check if rollback is needed
    pub fn should_rollback(&self, metrics: &CanaryMetrics) -> bool {
        // High error rate
        if metrics.error_rate > self.error_threshold {
            return true;
        }

        // Very high latency
        if metrics.p99_latency_ms > self.latency_threshold_ms {
            return true;
        }

        // Very low success rate
        if metrics.success_rate < 0.95 {
            return true;
        }

        false
    }
}

impl Default for RollbackDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_validator_creation() {
        let validator = DeploymentValidator::new();
        assert!(validator.min_health_score > 0.9);
        assert!(!validator.required_checks.is_empty());
    }

    #[test]
    fn test_validate_health() {
        let validator = DeploymentValidator::new();

        let mut checks = HashMap::new();
        checks.insert(
            "database".to_string(),
            CheckStatus {
                status: "pass".to_string(),
                message: None,
                response_time_ms: 10,
            },
        );
        checks.insert(
            "redis".to_string(),
            CheckStatus {
                status: "pass".to_string(),
                message: None,
                response_time_ms: 5,
            },
        );
        checks.insert(
            "api".to_string(),
            CheckStatus {
                status: "pass".to_string(),
                message: None,
                response_time_ms: 20,
            },
        );

        let health = HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            checks,
        };

        assert!(validator.validate_health(&health));
        assert!(validator.is_production_ready(&health));
    }

    #[test]
    fn test_validate_health_failing_check() {
        let validator = DeploymentValidator::new();

        let mut checks = HashMap::new();
        checks.insert(
            "database".to_string(),
            CheckStatus {
                status: "fail".to_string(),
                message: Some("Connection timeout".to_string()),
                response_time_ms: 5000,
            },
        );

        let health = HealthResponse {
            status: "unhealthy".to_string(),
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            checks,
        };

        assert!(!validator.validate_health(&health));
        assert!(!validator.is_production_ready(&health));
    }

    #[test]
    fn test_calculate_health_score() {
        let validator = DeploymentValidator::new();

        let mut checks = HashMap::new();
        checks.insert(
            "check1".to_string(),
            CheckStatus {
                status: "pass".to_string(),
                message: None,
                response_time_ms: 10,
            },
        );
        checks.insert(
            "check2".to_string(),
            CheckStatus {
                status: "pass".to_string(),
                message: None,
                response_time_ms: 10,
            },
        );
        checks.insert(
            "check3".to_string(),
            CheckStatus {
                status: "fail".to_string(),
                message: None,
                response_time_ms: 10,
            },
        );

        let health = HealthResponse {
            status: "degraded".to_string(),
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            checks,
        };

        let score = validator.calculate_score(&health);
        assert!((score - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_k8s_resource_validator() {
        let validator = K8sResourceValidator::new("investor-os");

        let requests = ResourceSpec {
            memory: "512Mi".to_string(),
            cpu: "500m".to_string(),
        };

        let limits = ResourceSpec {
            memory: "1Gi".to_string(),
            cpu: "1000m".to_string(),
        };

        assert!(validator.validate_resource_limits(&requests, &limits));
        assert!(validator.is_production_grade(&requests));
    }

    #[test]
    fn test_invalid_resource_limits() {
        let validator = K8sResourceValidator::new("investor-os");

        let requests = ResourceSpec {
            memory: "1Gi".to_string(),
            cpu: "1000m".to_string(),
        };

        let limits = ResourceSpec {
            memory: "512Mi".to_string(),
            cpu: "500m".to_string(),
        };

        assert!(!validator.validate_resource_limits(&requests, &limits));
    }

    #[test]
    fn test_parse_memory() {
        assert_eq!(
            K8sResourceValidator::parse_memory("512Mi"),
            512 * 1024 * 1024
        );
        assert_eq!(
            K8sResourceValidator::parse_memory("1Gi"),
            1024 * 1024 * 1024
        );
        assert_eq!(K8sResourceValidator::parse_memory("1024Ki"), 1024 * 1024);
    }

    #[test]
    fn test_parse_cpu() {
        assert_eq!(K8sResourceValidator::parse_cpu("500m"), 500.0);
        assert_eq!(K8sResourceValidator::parse_cpu("1"), 1000.0);
        assert_eq!(K8sResourceValidator::parse_cpu("2.5"), 2500.0);
    }

    #[test]
    fn test_canary_tester() {
        let tester = CanaryTester::new();

        // Test traffic split calculation
        let split = tester.calculate_traffic_split(1, 10);
        assert!((split - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_canary_promotion() {
        let tester = CanaryTester::new();

        let good_metrics = CanaryMetrics {
            total_requests: 1000,
            success_rate: 0.999,
            error_rate: 0.0001,
            p99_latency_ms: 500,
        };

        assert!(tester.can_promote(&good_metrics));
    }

    #[test]
    fn test_canary_not_ready_for_promotion() {
        let tester = CanaryTester::new();

        let bad_metrics = CanaryMetrics {
            total_requests: 50,
            success_rate: 0.95,
            error_rate: 0.05,
            p99_latency_ms: 2000,
        };

        assert!(!tester.can_promote(&bad_metrics));
    }

    #[test]
    fn test_rollback_detector() {
        let detector = RollbackDetector::new();

        let bad_metrics = CanaryMetrics {
            total_requests: 1000,
            success_rate: 0.90,
            error_rate: 0.10,
            p99_latency_ms: 5000,
        };

        assert!(detector.should_rollback(&bad_metrics));
    }

    #[test]
    fn test_no_rollback_needed() {
        let detector = RollbackDetector::new();

        let good_metrics = CanaryMetrics {
            total_requests: 1000,
            success_rate: 0.99,
            error_rate: 0.001,
            p99_latency_ms: 500,
        };

        assert!(!detector.should_rollback(&good_metrics));
    }

    #[test]
    fn test_non_production_grade_resources() {
        let validator = K8sResourceValidator::new("investor-os");

        let small_requests = ResourceSpec {
            memory: "256Mi".to_string(),
            cpu: "250m".to_string(),
        };

        assert!(!validator.is_production_grade(&small_requests));
    }

    #[test]
    fn test_production_grade_resources() {
        let validator = K8sResourceValidator::new("investor-os");

        let prod_requests = ResourceSpec {
            memory: "1Gi".to_string(),
            cpu: "1000m".to_string(),
        };

        assert!(validator.is_production_grade(&prod_requests));
    }

    #[test]
    fn test_health_score_empty_checks() {
        let validator = DeploymentValidator::new();

        let health = HealthResponse {
            status: "unknown".to_string(),
            version: "1.0.0".to_string(),
            uptime_seconds: 0,
            checks: HashMap::new(),
        };

        let score = validator.calculate_score(&health);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_set_min_health_score() {
        let mut validator = DeploymentValidator::new();

        validator.set_min_health_score(0.85);
        assert!((validator.min_health_score - 0.85).abs() < 0.01);

        // Test clamping
        validator.set_min_health_score(1.5);
        assert!((validator.min_health_score - 1.0).abs() < 0.01);

        validator.set_min_health_score(-0.5);
        assert!((validator.min_health_score - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_set_traffic_split() {
        let mut tester = CanaryTester::new();

        tester.set_traffic_split(25);
        assert_eq!(tester.traffic_split_percent, 25);

        // Test clamping
        tester.set_traffic_split(150);
        assert_eq!(tester.traffic_split_percent, 100);

        tester.set_traffic_split(0);
        assert_eq!(tester.traffic_split_percent, 0);
    }
}
