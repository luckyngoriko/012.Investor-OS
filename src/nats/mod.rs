//! NATS Event-Driven ML Pipeline (Sprint N1).
//!
//! Connects all ML models through NATS JetStream for real-time,
//! event-driven predictions. A single price tick triggers the
//! entire chain: features → predictions → consensus → trade proposal.
//!
//! # Subjects
//! ```text
//! ios.prices.{symbol}          — OHLCV candle
//! ios.features.{symbol}        — technical indicators
//! ios.predict.{model}.{symbol} — model prediction
//! ios.sentiment.{symbol}       — sentiment score
//! ios.consensus.{symbol}       — combined consensus
//! ios.trade.proposal.{symbol}  — trade proposal
//! ```

pub mod client;
pub mod consensus_worker;
pub mod feature_service;
pub mod hrm_worker;
pub mod messages;
pub mod publisher;
pub mod streams;
pub mod trade_proposer;

pub use client::NatsClient;
pub use messages::*;
pub use publisher::NatsPublisher;
