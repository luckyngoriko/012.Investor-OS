//! Trading Execution Module (Wave 1b Task 6)
//!
//! Consumes trade proposals from the NATS consensus pipeline
//! and routes them through mode-specific handlers:
//! - Signal: log/notify only, user trades manually
//! - SemiAuto: AI proposes, user confirms/rejects
//! - FullAuto: AI executes automatically within risk limits

pub mod executor;
pub mod modes;
