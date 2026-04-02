//! Backtesting Module (Wave 2 Task 14).
//!
//! Historical strategy backtesting engine that loads price data from
//! the `prices` table and simulates a simple momentum strategy
//! (price vs. 20-period SMA).

pub mod engine;

pub use engine::{run_backtest, BacktestRequest, BacktestResult};
