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

// Clean builds - suppress non-critical warnings
// For strict mode: RUSTFLAGS="-W warnings" cargo build
#![cfg_attr(not(debug_assertions), allow(warnings))]
#![cfg_attr(debug_assertions, allow(warnings))]

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
/// - Performance attribution
/// - Risk analysis
/// - Trade analytics
/// - Real-time P&L tracking
pub mod analytics;

/// Machine Learning module
///
/// Sprint 23: ML Training Pipeline
/// - Feature engineering
/// - Model training
/// - Model versioning
/// - A/B testing
pub mod ml;

/// Real-time streaming data
///
/// Sprint 7: Streaming Data Pipeline
/// - Market data ingestion
/// - Signal processing
/// - Event-driven execution
pub mod streaming;

/// Risk Management module
///
/// Sprint 12: Risk Management
/// - Portfolio risk
/// - Drawdown limits
/// - Correlation monitoring
/// - Stress testing
pub mod risk;

/// Data collectors
///
/// Market data collection from various sources
pub mod collectors;

/// Phoenix recovery system
///
/// Self-healing and state recovery
/// - RAG-based experience memory
/// - LLM-powered decision making
/// - Stress testing (8 historical crises)
pub mod phoenix;

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

/// LangChain-inspired AI Component Framework
///
/// Sprint 1-2: Composable LLM chains, prompts, tools, parsers
/// - Chain trait for composable AI operations
/// - Prompt templates with variable substitution
/// - Tool registry for agent capabilities
/// - Structured output parsing
pub mod langchain;

/// LangGraph-inspired State Machine Framework
///
/// Sprint 3-4: Trading decision graphs
/// - Nodes: Data collection, CQ calculation, execution
/// - Edges: Conditional transitions based on market regime
/// - State: Shared mutable state across graph execution
/// - Loops: Self-improvement and re-evaluation
pub mod langgraph;

/// Temporal-inspired Durable Workflow Engine
///
/// Sprint 4: Resilient workflow execution
/// - Deterministic state machines
/// - Automatic recovery from crashes
/// - Saga pattern for distributed transactions
/// - Workflow versioning
pub mod temporal;

/// Configuration management
///
/// Sprint 1: Environment Configuration
/// - Environment variables
/// - Feature flags
/// - Settings validation
/// - Secrets management
pub mod config;

/// Resilience Patterns
///
/// Sprint 8: Circuit breakers and fault tolerance
/// - Circuit breaker for external APIs
/// - Bulkhead pattern
/// - Retry policies
pub mod resilience;

/// Observability
///
/// Sprint 8: Metrics, tracing, and logging
/// - Prometheus metrics
/// - Distributed tracing
/// - Structured logging
pub mod observability;

/// Treasury Module
///
/// Sprint 15: Capital Management
/// - Multi-currency wallet (fiat + crypto)
/// - Deposits and withdrawals
/// - FX conversion
/// - Yield optimization
/// - Cross-collateralization
pub mod treasury;

/// HRM (Hierarchical Reasoning Model) Module
///
/// Sprints 36-43: AI Trading Engine
/// - Native Rust ML with Burn
/// - Multi-source conviction calculation
/// - Market regime detection
/// - SafeTensors weight loading
pub mod hrm;

/// Distributed Inference Module
///
/// Sprint 49: Distributed HRM Inference
/// - gRPC cluster nodes
/// - Load balancing
/// - Service discovery
/// - Fault tolerance
pub mod distributed;

/// Monitoring Module
///
/// Sprint 46: Performance Monitoring
/// - Prometheus metrics
/// - Grafana dashboards
/// - Health checks
/// - Alerting
pub mod monitoring;

/// EU AI Act & GDPR Compliance Module
///
/// Sprint 52: EU Compliance Integration
/// - EU AI Act compliance tracking via AI-OS.NET
/// - GDPR "Right to be forgotten" and "Data portability"
/// - Audit logging for AI decisions (Article 12 requirement)
/// - DLP (Data Loss Prevention) via AI-OS-PG
#[cfg(feature = "eu_compliance")]
pub mod compliance;
