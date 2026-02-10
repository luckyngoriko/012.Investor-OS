//! Investor OS v3.5 - Autonomous AI Trading System
//!
//! Production-ready entry point with graceful shutdown

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};

use investor_os::config::AppConfig;
use investor_os::health::HealthChecker;
use investor_os::observability::{init_metrics, init_observability, MetricsCollector};
use investor_os::resilience::CircuitBreakerRegistry;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub health: Arc<HealthChecker>,
    pub circuits: Arc<CircuitBreakerRegistry>,
    pub metrics: Arc<MetricsCollector>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = match AppConfig::load() {
        Ok(cfg) => {
            info!(
                environment = ?cfg.environment,
                "Configuration loaded successfully"
            );
            Arc::new(cfg)
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize observability
    init_observability("investor-os", config.logging.json_format);
    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting Investor OS"
    );

    // Initialize metrics
    let metrics = init_metrics();

    // Initialize health checker
    let health = Arc::new(HealthChecker::new());

    // Initialize circuit breakers
    let circuits = CircuitBreakerRegistry::new();

    // Create application state
    let state = AppState {
        config: config.clone(),
        health: health.clone(),
        circuits,
        metrics,
    };

    // Build the API router
    let app = create_router(state.clone()).await;

    // Create socket address
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .expect("Invalid server address");

    info!(%addr, "Server starting");

    // Create shutdown channel
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Spawn server with graceful shutdown
    let server = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_rx.recv().await;
                info!("Received shutdown signal, starting graceful shutdown");
            })
            .await
            .unwrap();
    });

    // Setup signal handlers
    tokio::spawn(handle_signals(shutdown_tx, health.clone()));

    // Wait for server to complete
    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }

    // Graceful shutdown sequence
    info!("Graceful shutdown initiated");
    
    // Signal health checker
    health.shutdown();
    
    // Wait for in-flight requests
    tokio::time::sleep(Duration::from_secs(config.server.shutdown_timeout_secs)).await;
    
    // Export final metrics
    info!("Final metrics:\n{}", state.metrics.export_prometheus());
    
    info!("Shutdown complete");
    Ok(())
}

/// Create API router
async fn create_router(state: AppState) -> axum::Router {
    use axum::routing::get;
    use axum::{middleware, Router};
    
    // Import handlers from the library
    use investor_os::api::handlers;

    Router::new()
        // Health endpoints
        .route("/api/health", get(handlers::health))
        .route("/api/ready", get(handlers::readiness))
        // Metrics endpoint
        .route("/metrics", get(|| async move {
            let metrics = init_metrics();
            metrics.export_prometheus()
        }))
        // API routes
        .nest("/api/v1", api_routes())
        // Add middleware
        .layer(middleware::from_fn(investor_os::middleware::logging_middleware))
        .with_state(state)
}

/// API v1 routes
fn api_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    use axum::Router;

    Router::new()
        .route("/trades", get(|| async { "Trades endpoint" }))
        .route("/positions", get(|| async { "Positions endpoint" }))
        .route("/orders", post(|| async { "Create order endpoint" }))
}

/// Handle OS signals for graceful shutdown
async fn handle_signals(shutdown_tx: tokio::sync::mpsc::Sender<()>, health: Arc<HealthChecker>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C");
        }
        _ = terminate => {
            info!("Received SIGTERM");
        }
    }

    // Signal shutdown
    let _ = shutdown_tx.send(()).await;
    health.shutdown();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_state_creation() {
        let config = Arc::new(AppConfig::default());
        let health = Arc::new(HealthChecker::new());
        let circuits = CircuitBreakerRegistry::new();
        let metrics = init_metrics();

        let state = AppState {
            config,
            health,
            circuits,
            metrics,
        };

        assert!(!state.health.is_shutting_down());
    }
}
