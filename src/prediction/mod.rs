//! ML Prediction Pipeline (Sprint 114).
//!
//! Client for the Python ML sidecar service with graceful fallback
//! to HRM deterministic policy when the sidecar is unavailable.
//!
//! # Architecture
//! ```text
//! Rust API  →  MlSidecarClient  →  ios-ml-sidecar:9000 (FastAPI)
//!                    ↓ (if unavailable)
//!              fallback::fallback_predict()
//! ```

pub mod client;
pub mod consensus;
pub mod error;
pub mod fallback;
pub mod features;
pub mod forecasting;
pub mod handlers;
pub mod metrics;
pub mod repository;
pub mod types;

pub use client::MlSidecarClient;
pub use error::PredictionError;
pub use handlers::{
    get_history_handler, get_prediction_handler, list_models_handler, predict_handler,
    HistoryQuery, PredictBody,
};
pub use types::{PredictionRequest, PredictionResponse, PredictionResult};
