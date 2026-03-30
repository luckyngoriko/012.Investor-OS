//! Prediction module errors (Sprint 114).

use thiserror::Error;

/// Errors from the ML prediction pipeline.
#[derive(Error, Debug)]
pub enum PredictionError {
    #[error("ML sidecar unavailable: {0}")]
    SidecarUnavailable(String),

    #[error("ML sidecar request failed: {0}")]
    RequestFailed(String),

    #[error("ML sidecar returned error: {status} — {message}")]
    SidecarError { status: u16, message: String },

    #[error("Prediction timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Invalid prediction response: {0}")]
    InvalidResponse(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Database error: {0}")]
    Database(String),
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, PredictionError>;

impl From<reqwest::Error> for PredictionError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout { timeout_ms: 5000 }
        } else if err.is_connect() {
            Self::SidecarUnavailable(err.to_string())
        } else {
            Self::RequestFailed(err.to_string())
        }
    }
}
