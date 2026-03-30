//! Feature computation and storage (Sprint 118).
//!
//! Provides real-time technical indicator computation in Rust
//! and a feature store backed by TimescaleDB.

pub mod store;
pub mod technical;

pub use technical::{compute_technical_features, Candle, TechnicalFeatures};
