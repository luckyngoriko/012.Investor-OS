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

/// ML Module - APIs and Pipeline
pub mod ml;

/// Real-Time Streaming - Sprint 12
pub mod streaming;

/// Risk Management - Sprint 13
pub mod risk;

/// Alternative Data Collectors - Sprint 14
///
/// News NLP analysis, social sentiment, web scraping
pub mod collectors;

/// Phoenix Mode: Autonomous Learning System
///
/// Sprint 9: Self-learning trading with RAG memory and LLM strategist
/// - Paper trading simulator
/// - Realistic graduation criteria (15-30% CAGR, not 82%)
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
/// Sprint 5-6: Reliable execution guarantees
/// - Workflow trait for durable processes
/// - Activity trait for idempotent operations
/// - Saga pattern for compensation
/// - Signals and queries for external communication
pub mod temporal;

/// Configuration Management
///
/// Sprint 8: Environment-based configuration
/// - Environment variables
/// - Validation
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

