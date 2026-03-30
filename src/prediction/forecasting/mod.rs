//! Time series forecasting via augurs (Sprint 123).
//!
//! Provides ETS (exponential smoothing) and MSTL (seasonal-trend decomposition)
//! forecasting natively in Rust — no Python sidecar needed.

pub mod ets;
pub mod mstl;

pub use ets::{forecast_ets, EtsForecast};
pub use mstl::{forecast_mstl, MstlForecast};
