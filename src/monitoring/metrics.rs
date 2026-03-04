//! Prometheus Metrics for Investor OS
//! Sprint 46: Performance Monitoring

use prometheus::{
    Counter, CounterVec, Gauge, Histogram, HistogramVec, IntCounter, IntGauge, Opts, Registry,
};
use std::sync::OnceLock;

/// Global metrics registry
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// HRM inference counter
static HRM_INFERENCE_COUNT: OnceLock<Counter> = OnceLock::new();

/// HRM inference latency histogram
static HRM_INFERENCE_LATENCY: OnceLock<Histogram> = OnceLock::new();

/// HRM inference errors
static HRM_INFERENCE_ERRORS: OnceLock<Counter> = OnceLock::new();

/// HRM model loaded status
static HRM_MODEL_LOADED: OnceLock<Gauge> = OnceLock::new();

/// API requests counter
static API_REQUESTS: OnceLock<CounterVec> = OnceLock::new();

/// API request latency
static API_LATENCY: OnceLock<HistogramVec> = OnceLock::new();

/// Active WebSocket connections
static WS_CONNECTIONS: OnceLock<IntGauge> = OnceLock::new();

/// WebSocket messages counter
static WS_MESSAGES: OnceLock<CounterVec> = OnceLock::new();

/// WebSocket errors
static WS_ERRORS: OnceLock<IntCounter> = OnceLock::new();

/// Initialize all metrics
pub fn init_metrics() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let registry = Registry::new();

        // HRM Inference Counter
        let hrm_count = Counter::new("hrm_inference_total", "Total HRM inferences performed")
            .expect("metric can be created");
        HRM_INFERENCE_COUNT.set(hrm_count.clone()).ok();
        registry.register(Box::new(hrm_count)).ok();

        // HRM Inference Latency
        let hrm_latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "hrm_inference_duration_seconds",
                "HRM inference latency in seconds",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.002, 0.005, 0.01, 0.025, 0.05]),
        )
        .expect("metric can be created");
        HRM_INFERENCE_LATENCY.set(hrm_latency.clone()).ok();
        registry.register(Box::new(hrm_latency)).ok();

        // HRM Inference Errors
        let hrm_errors = Counter::new("hrm_inference_errors_total", "Total HRM inference errors")
            .expect("metric can be created");
        HRM_INFERENCE_ERRORS.set(hrm_errors.clone()).ok();
        registry.register(Box::new(hrm_errors)).ok();

        // HRM Model Loaded Status
        let hrm_loaded = Gauge::new("hrm_model_loaded", "1 if HRM model is loaded, 0 otherwise")
            .expect("metric can be created");
        HRM_MODEL_LOADED.set(hrm_loaded.clone()).ok();
        registry.register(Box::new(hrm_loaded)).ok();

        // API Requests Counter
        let api_requests = CounterVec::new(
            Opts::new("api_requests_total", "Total API HTTP requests"),
            &["method", "endpoint", "status"],
        )
        .expect("metric can be created");
        API_REQUESTS.set(api_requests.clone()).ok();
        registry.register(Box::new(api_requests)).ok();

        // API Latency Histogram
        let api_latency = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "api_request_duration_seconds",
                "API HTTP request latency in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["method", "endpoint"],
        )
        .expect("metric can be created");
        API_LATENCY.set(api_latency.clone()).ok();
        registry.register(Box::new(api_latency)).ok();

        // WebSocket Connections
        let ws_connections = IntGauge::new(
            "websocket_connections_active",
            "Number of active WebSocket connections",
        )
        .expect("metric can be created");
        WS_CONNECTIONS.set(ws_connections.clone()).ok();
        registry.register(Box::new(ws_connections)).ok();

        // WebSocket Messages
        let ws_messages = CounterVec::new(
            Opts::new("websocket_messages_total", "Total WebSocket messages"),
            &["direction", "message_type"],
        )
        .expect("metric can be created");
        WS_MESSAGES.set(ws_messages.clone()).ok();
        registry.register(Box::new(ws_messages)).ok();

        // WebSocket Errors
        let ws_errors = IntCounter::new("websocket_errors_total", "Total WebSocket errors")
            .expect("metric can be created");
        WS_ERRORS.set(ws_errors.clone()).ok();
        registry.register(Box::new(ws_errors)).ok();

        // Process metrics are automatically collected when "process" feature is enabled

        registry
    })
}

/// Get the global registry
pub fn registry() -> &'static Registry {
    init_metrics()
}

/// Record HRM inference
pub fn record_hrm_inference(duration: std::time::Duration) {
    if let Some(counter) = HRM_INFERENCE_COUNT.get() {
        counter.inc();
    }
    if let Some(hist) = HRM_INFERENCE_LATENCY.get() {
        hist.observe(duration.as_secs_f64());
    }
}

/// Record HRM inference error
pub fn record_hrm_inference_error() {
    if let Some(counter) = HRM_INFERENCE_ERRORS.get() {
        counter.inc();
    }
}

/// Set HRM model loaded status
pub fn set_hrm_model_loaded(loaded: bool) {
    if let Some(gauge) = HRM_MODEL_LOADED.get() {
        gauge.set(if loaded { 1.0 } else { 0.0 });
    }
}

/// Record API request
pub fn record_api_request(method: &str, endpoint: &str, status: u16) {
    if let Some(counter) = API_REQUESTS.get() {
        counter
            .with_label_values(&[method, endpoint, &status.to_string()])
            .inc();
    }
}

/// Start API request timer
pub fn start_api_timer(method: &str, endpoint: &str) -> Option<prometheus::HistogramTimer> {
    API_LATENCY
        .get()
        .map(|h| h.with_label_values(&[method, endpoint]).start_timer())
}

/// Increment active WebSocket connections
pub fn ws_connection_opened() {
    if let Some(gauge) = WS_CONNECTIONS.get() {
        gauge.inc();
    }
}

/// Decrement active WebSocket connections
pub fn ws_connection_closed() {
    if let Some(gauge) = WS_CONNECTIONS.get() {
        gauge.dec();
    }
}

/// Record WebSocket message
pub fn record_ws_message(direction: &str, msg_type: &str) {
    if let Some(counter) = WS_MESSAGES.get() {
        counter.with_label_values(&[direction, msg_type]).inc();
    }
}

/// Record WebSocket error
pub fn record_ws_error() {
    if let Some(counter) = WS_ERRORS.get() {
        counter.inc();
    }
}

/// Encode metrics to Prometheus text format
pub fn encode_metrics() -> Result<String, prometheus::Error> {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = registry().gather();
    encoder.encode_to_string(&metric_families)
}
