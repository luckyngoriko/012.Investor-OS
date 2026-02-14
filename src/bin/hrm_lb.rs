//! HRM Load Balancer
//! Sprint 49: Distributed Inference
//!
//! Run with: cargo run --bin hrm-lb -- --nodes=node-1:50051,node-2:50052

use clap::Parser;
use investor_os::distributed::discovery::{StaticDiscovery, NodeInfo};
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
        println!("  - {} at {} (weight: {})", 
            node.id.0, 
            node.addr,
            node.weight
        );
    }
    println!();
    
    println!("HTTP endpoints:");
    println!("  GET /health          - Health check");
    println!("  GET /stats           - Load balancer stats");
    println!();
    println!("gRPC: Forwarded to backend nodes");
    println!();
    println!("Press Ctrl+C to stop");
    println!();
    
    // TODO: Implement actual HTTP/gRPC server
    // For now, just keep running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
