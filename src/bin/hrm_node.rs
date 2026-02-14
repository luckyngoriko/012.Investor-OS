//! HRM Node - gRPC Server
//! Sprint 49: Distributed Inference
//!
//! Run with: cargo run --bin hrm-node -- --id=node-1 --port=50051

use clap::Parser;
use investor_os::distributed::node::{HRMNode, NodeConfig, NodeId};
use std::net::SocketAddr;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "hrm-node")]
#[command(about = "HRM Inference Node - Distributed HRM Server")]
struct Args {
    /// Node ID
    #[arg(short, long, default_value = "node-1")]
    id: String,
    
    /// Bind port
    #[arg(short, long, default_value = "50051")]
    port: u16,
    
    /// Bind host
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    
    /// Max concurrent requests
    #[arg(long, default_value = "1000")]
    max_requests: usize,
    
    /// Request timeout (ms)
    #[arg(long, default_value = "5000")]
    timeout_ms: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let args = Args::parse();
    
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║           HRM Inference Node (Sprint 49)                      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();
    
    let bind_addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    
    let config = NodeConfig {
        id: NodeId::new(args.id.clone()),
        bind_addr,
        max_concurrent_requests: args.max_requests,
        request_timeout_ms: args.timeout_ms,
    };
    
    info!("Starting HRM node: {} on {}", args.id, bind_addr);
    
    let node = HRMNode::new(config);
    
    // Print info
    println!("Node ID:          {}", args.id);
    println!("Bind Address:     {}", bind_addr);
    println!("Max Requests:     {}", args.max_requests);
    println!("Timeout:          {}ms", args.timeout_ms);
    println!("Backend:          {}", node.backend.name());
    println!();
    println!("gRPC endpoints:");
    println!("  - Infer ( unary )");
    println!("  - StreamInfer ( streaming )");
    println!("  - HealthCheck ( unary )");
    println!();
    println!("Press Ctrl+C to stop");
    println!();
    
    // Start server
    if let Err(e) = node.serve().await {
        error!("Server error: {}", e);
        return Err(e.into());
    }
    
    Ok(())
}
