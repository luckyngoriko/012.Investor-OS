//! Investor OS v3.0 — Autonomous AI Trading System
//!
//! Uses neurocod-rag (L1-L35) for:
//! - L29 ModelRouter: Route queries to optimal LLM
//! - L30 CognitiveKernel: Priority scheduling for trading decisions
//! - L31 ContextBudget: Token allocation for evidence-heavy trade proofs
//! - L32 StreamingIndexer: Real-time dark pool & options flow
//! - L33 Ontology: Trading domain concepts (CQ, PEGY, GEX, etc.)
//! - L34 CausalTracer: Regulatory-grade audit trails
//! - L35 AutonomousMonitor: Self-healing 24/7 operation

pub mod prelude {
    //! Re-export the most commonly used RAG types for Investor OS
    pub use neurocod_rag::{
        // L29
        ModelRouter, ModelProfile, RoutingDecision,
        // L30
        CognitiveKernel, QueryPriority, RagStrategy,
        // L31
        ContextBudget, BudgetPreset,
        // L32
        StreamingIndexer, StreamEvent,
        // L33
        Ontology, RelationType,
        // L34
        CausalTracer, TraceRecord,
        // L35
        AutonomousMonitor, HealthStatus,
    };
}

/// RAG (Retrieval-Augmented Generation) module for financial document analysis
/// 
/// Sprint 5: PostgreSQL Optimization + RAG Integration
/// - SEC filings parsing (10-K, 10-Q)
/// - Earnings call transcript analysis
/// - Semantic search on decision journal
pub mod rag;

/// HTTP API handlers
pub mod api;

/// Broker Integration module
///
/// Sprint 6: Interactive Brokers Integration
/// - Order management
/// - Position tracking
/// - Risk management
/// - Execution engine
pub mod broker;

/// Analytics module
///
/// Sprint 7: Advanced Analytics & Backtesting
/// - Backtesting engine
/// - Risk metrics
/// - Performance attribution
/// - ML predictions
pub mod analytics;

/// Signals module
///
/// Trading signals and CQ calculation
pub mod signals;

/// Health checks
///
/// Sprint 8: Health checks and graceful shutdown
pub mod health;

/// HTTP Middleware
///
/// Sprint 8: Rate limiting, logging, auth
pub mod middleware;

