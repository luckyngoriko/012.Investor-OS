//! Observability - metrics, tracing, and logging
//!
//! OpenTelemetry integration for distributed tracing and metrics

use std::time::Instant;
use tracing::{info, span, Level, Span};

/// Initialize observability (tracing + metrics)
pub fn init_observability(service_name: &str, json_format: bool) {
    // Initialize tracing subscriber
    if json_format {
        // JSON format for production
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    } else {
        // Pretty format for development
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    info!("Observability initialized for service: {}", service_name);
}

/// Create a span for tracking operations
pub fn create_operation_span(name: &str, operation: &str) -> Span {
    span!(
        Level::INFO,
        "operation",
        name = %name,
        operation = %operation
    )
}

/// Timer for measuring operation duration
pub struct OperationTimer {
    start: Instant,
    operation: String,
}

impl OperationTimer {
    /// Start a new timer
    pub fn start(operation: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            operation: operation.into(),
        }
    }

    /// Record elapsed time and return duration in milliseconds
    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    /// Finish the timer and log the duration
    pub fn finish(self) {
        let duration = self.start.elapsed();
        info!(
            operation = %self.operation,
            duration_ms = duration.as_millis() as u64,
            "Operation completed"
        );
    }
}

/// Metrics collector
pub struct MetricsCollector {
    counters: std::sync::RwLock<std::collections::HashMap<String, u64>>,
    histograms: std::sync::RwLock<std::collections::HashMap<String, Vec<f64>>>,
    gauges: std::sync::RwLock<std::collections::HashMap<String, f64>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            counters: std::sync::RwLock::new(std::collections::HashMap::new()),
            histograms: std::sync::RwLock::new(std::collections::HashMap::new()),
            gauges: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Increment a counter
    pub fn increment_counter(&self, name: &str) {
        let mut counters = self.counters.write().unwrap();
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Record a histogram value
    pub fn record_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().unwrap();
        histograms.entry(name.to_string()).or_default().push(value);
    }

    /// Set a gauge value
    pub fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().unwrap();
        gauges.insert(name.to_string(), value);
    }

    /// Get counter value
    pub fn get_counter(&self, name: &str) -> u64 {
        self.counters.read().unwrap().get(name).copied().unwrap_or(0)
    }

    /// Get gauge value
    pub fn get_gauge(&self, name: &str) -> f64 {
        self.gauges.read().unwrap().get(name).copied().unwrap_or(0.0)
    }

    /// Get histogram statistics
    pub fn get_histogram_stats(&self, name: &str) -> Option<HistogramStats> {
        let histograms = self.histograms.read().unwrap();
        let values = histograms.get(name)?;
        
        if values.is_empty() {
            return None;
        }

        let sum: f64 = values.iter().sum();
        let count = values.len() as u64;
        let mean = sum / count as f64;

        // Calculate percentiles
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50 = sorted[sorted.len() / 2];
        let p95 = sorted[((sorted.len() as f64 * 0.95) as usize).min(sorted.len() - 1)];
        let p99 = sorted[((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1)];

        Some(HistogramStats {
            count,
            mean,
            p50,
            p95,
            p99,
            min: sorted[0],
            max: sorted[sorted.len() - 1],
        })
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Counters
        let counters = self.counters.read().unwrap();
        for (name, value) in counters.iter() {
            output.push_str(&format!("# TYPE {} counter\n", name.replace("-", "_")));
            output.push_str(&format!("{} {}\n", name.replace("-", "_"), value));
        }

        // Gauges
        let gauges = self.gauges.read().unwrap();
        for (name, value) in gauges.iter() {
            output.push_str(&format!("# TYPE {} gauge\n", name.replace("-", "_")));
            output.push_str(&format!("{} {}\n", name.replace("-", "_"), value));
        }

        // Histograms
        let histograms = self.histograms.read().unwrap();
        for (name, values) in histograms.iter() {
            if values.is_empty() {
                continue;
            }

            let name_normalized = name.replace("-", "_");
            output.push_str(&format!("# TYPE {} histogram\n", name_normalized));

            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

            // Buckets
            let buckets = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];
            let mut cumulative_count = 0;

            for bucket in &buckets {
                let count = sorted.iter().filter(|&&v| v <= *bucket).count();
                cumulative_count = cumulative_count.max(count);
                output.push_str(&format!(
                    "{}_bucket{{le=\"{}\"}} {}\n",
                    name_normalized, bucket, cumulative_count
                ));
            }

            output.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", name_normalized, sorted.len()));
            output.push_str(&format!("{}_sum {}\n", name_normalized, sorted.iter().sum::<f64>()));
            output.push_str(&format!("{}_count {}\n", name_normalized, sorted.len()));
        }

        output
    }
}

/// Histogram statistics
#[derive(Debug, Clone, Copy)]
pub struct HistogramStats {
    pub count: u64,
    pub mean: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub min: f64,
    pub max: f64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Global metrics instance
use std::sync::{Arc, OnceLock};
static METRICS: OnceLock<Arc<MetricsCollector>> = OnceLock::new();

/// Initialize global metrics
pub fn init_metrics() -> Arc<MetricsCollector> {
    METRICS.get_or_init(|| Arc::new(MetricsCollector::new())).clone()
}

/// Macro to instrument a function with metrics
#[macro_export]
macro_rules! instrument {
    ($name:expr, $body:block) => {{
        let _span = $crate::observability::create_operation_span($name, "execute");
        let _timer = $crate::observability::OperationTimer::start($name);
        
        let result = $body;
        
        $crate::observability::init_metrics().increment_counter(&format!("{}_total", $name));
        
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counter() {
        let metrics = MetricsCollector::new();
        
        metrics.increment_counter("test_counter");
        metrics.increment_counter("test_counter");
        
        assert_eq!(metrics.get_counter("test_counter"), 2);
    }

    #[test]
    fn test_metrics_gauge() {
        let metrics = MetricsCollector::new();
        
        metrics.set_gauge("test_gauge", 42.0);
        
        assert_eq!(metrics.get_gauge("test_gauge"), 42.0);
    }

    #[test]
    fn test_metrics_histogram() {
        let metrics = MetricsCollector::new();
        
        metrics.record_histogram("test_histogram", 1.0);
        metrics.record_histogram("test_histogram", 2.0);
        metrics.record_histogram("test_histogram", 3.0);
        
        let stats = metrics.get_histogram_stats("test_histogram").unwrap();
        assert_eq!(stats.count, 3);
        assert_eq!(stats.mean, 2.0);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = MetricsCollector::new();
        
        metrics.increment_counter("requests");
        metrics.set_gauge("active_connections", 10.0);
        metrics.record_histogram("latency", 0.1);
        
        let output = metrics.export_prometheus();
        assert!(output.contains("requests"));
        assert!(output.contains("active_connections"));
        assert!(output.contains("latency"));
    }

    #[test]
    fn test_operation_timer() {
        let timer = OperationTimer::start("test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        assert!(timer.elapsed_ms() >= 10);
        timer.finish(); // Should not panic
    }
}
