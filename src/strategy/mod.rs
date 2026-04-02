//! Strategy Engine (Wave 1b Task 5).
//!
//! CRUD operations for user-defined trading strategies stored in `user_strategies`.
//! Each strategy specifies symbols, ML models, execution mode, and risk limits.

pub mod repository;
pub mod types;

pub use types::{CreateStrategyRequest, Strategy, UpdateStrategyRequest};
