//! Health Checks
//!
//! S8-D5: Health checks and graceful shutdown

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Health checker
pub struct HealthChecker {
    shutting_down: AtomicBool,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new() -> Self {
        Self {
            shutting_down: AtomicBool::new(false),
        }
    }

    /// Liveness probe
    pub async fn liveness(&self) -> HealthStatus {
        HealthStatus {
            status: if self.shutting_down.load(Ordering::SeqCst) {
                "shutting_down".to_string()
            } else {
                "healthy".to_string()
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Readiness probe
    pub async fn readiness(&self) -> HealthStatus {
        self.liveness().await
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        info!("Health checker received shutdown signal");
        self.shutting_down.store(true, Ordering::SeqCst);
    }

    /// Check if shutting down
    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::SeqCst)
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}
