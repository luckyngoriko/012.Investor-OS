//! HRM Load Balancer
//! Sprint 49: Distributed Inference
//!
//! Run with: cargo run --bin hrm-lb -- --nodes=node-1:50051,node-2:50052

use clap::Parser;
use investor_os::distributed::discovery::NodeInfo;
use investor_os::distributed::load_balancer::{LoadBalancer, LoadBalancingStrategy};
use investor_os::distributed::node::NodeId;
use std::net::SocketAddr;
use tracing::info;

#[derive(Parser)]
#[command(name = "hrm-lb")]
#[command(about = "HRM Load Balancer - Distributed HRM Load Balancer")]
struct Args {
    /// Comma-separated list of nodes (host:port)
    #[arg(short, long, value_delimiter = ',')]
    nodes: Vec<String>,

    /// Load balancing strategy
    #[arg(short, long, default_value = "least-latency")]
    strategy: String,

    /// Bind port
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Bind host
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║           HRM Load Balancer (Sprint 49)                       ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Parse nodes
    let mut nodes: Vec<NodeInfo> = Vec::new();
    for (i, addr) in args.nodes.iter().enumerate() {
        nodes.push(NodeInfo {
            id: NodeId::new(format!("node-{}", i)),
            addr: addr.to_string(),
            weight: 1,
            last_heartbeat: std::time::Instant::now(),
        });
    }

    // Parse strategy
    let strategy = match args.strategy.as_str() {
        "round-robin" => LoadBalancingStrategy::RoundRobin,
        "least-connections" => LoadBalancingStrategy::LeastConnections,
        "least-latency" => LoadBalancingStrategy::LeastLatency,
        "weighted" => LoadBalancingStrategy::Weighted,
        _ => LoadBalancingStrategy::LeastLatency,
    };

    info!("Starting load balancer with {} nodes", nodes.len());
    info!("Strategy: {:?}", strategy);

    let lb = LoadBalancer::new(nodes).with_strategy(strategy);

    // Print info
    println!("Load Balancer Configuration:");
    println!("  Nodes:     {}", lb.node_count());
    println!("  Strategy:  {:?}", strategy);
    println!("  Bind:      {}:{}", args.host, args.port);
    println!();

    println!("Backend Nodes:");
    for node in lb.nodes() {
        println!(
            "  - {} at {} (weight: {})",
            node.id.0, node.addr, node.weight
        );
    }
    println!();

    println!("HTTP endpoints:");
    println!("  POST /infer  — forwarded to backend nodes");
    println!("  GET  /health — load balancer health");
    println!("  GET  /stats  — load balancer statistics");
    println!();
    println!("Press Ctrl+C to stop");
    println!();

    let bind_addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    // Shared state
    let lb = std::sync::Arc::new(lb);
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let state = LbState {
        lb: lb.clone(),
        client: http_client,
    };

    let app = axum::Router::new()
        .route("/infer", axum::routing::post(handle_infer))
        .route("/health", axum::routing::get(handle_health))
        .route("/stats", axum::routing::get(handle_stats))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    info!("HRM LB listening on {}", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct LbState {
    lb: std::sync::Arc<LoadBalancer>,
    client: reqwest::Client,
}

async fn handle_infer(
    axum::extract::State(state): axum::extract::State<LbState>,
    body: axum::body::Bytes,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    let node = state
        .lb
        .select_node()
        .ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)?;

    let url = format!("http://{}/infer", node.addr);

    let response = state
        .client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Upstream error for node {}: {}", node.id.0, e);
            axum::http::StatusCode::BAD_GATEWAY
        })?;

    let json: serde_json::Value = response.json().await.map_err(|e| {
        tracing::error!("Upstream response parse error: {}", e);
        axum::http::StatusCode::BAD_GATEWAY
    })?;

    Ok(axum::Json(json))
}

async fn handle_health(
    axum::extract::State(state): axum::extract::State<LbState>,
) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "nodes": state.lb.node_count(),
    }))
}

async fn handle_stats(
    axum::extract::State(state): axum::extract::State<LbState>,
) -> axum::Json<serde_json::Value> {
    let nodes: Vec<serde_json::Value> = state
        .lb
        .nodes()
        .iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id.0,
                "addr": n.addr,
                "weight": n.weight,
                "active_requests": n.metrics.requests_active.load(std::sync::atomic::Ordering::Relaxed),
                "total_requests": n.metrics.requests_total.load(std::sync::atomic::Ordering::Relaxed),
                "avg_latency_us": n.metrics.avg_latency_micros() as u64,
            })
        })
        .collect();

    axum::Json(serde_json::json!({
        "node_count": state.lb.node_count(),
        "nodes": nodes,
    }))
}
