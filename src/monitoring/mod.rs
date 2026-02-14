//! Monitoring & Observability Module
//! Sprint 46: Performance Monitoring

pub mod metrics;

use tracing::info;

/// Initialize monitoring subsystem
pub fn init() {
    info!("Initializing monitoring subsystem...");
    metrics::init_metrics();
    info!("Monitoring subsystem initialized");
}

/// Shutdown monitoring
pub fn shutdown() {
    info!("Shutting down monitoring subsystem...");
    // Any cleanup needed
}
