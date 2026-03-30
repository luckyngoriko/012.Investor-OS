//! HTTP client for the ML sidecar service (Sprint 114).
//!
//! Handles communication with the Python FastAPI sidecar over ios-net.
//! Includes timeout, retry with exponential backoff, and health checks.

use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use tracing::{debug, warn};

use super::error::PredictionError;
use super::types::{PredictionRequest, PredictionResponse, SidecarHealth};

/// Client for the ML sidecar FastAPI service.
#[derive(Clone)]
pub struct MlSidecarClient {
    http: Client,
    base_url: String,
    max_retries: u32,
}

impl MlSidecarClient {
    /// Create a new client pointing at the sidecar base URL.
    ///
    /// `base_url` should be like `http://ml-sidecar:9000`.
    pub fn new(base_url: &str, timeout_ms: u64) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            max_retries: 2,
        }
    }

    /// Check if the sidecar is healthy.
    pub async fn health_check(&self) -> super::error::Result<SidecarHealth> {
        let url = format!("{}/health", self.base_url);
        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(PredictionError::SidecarError {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        resp.json::<SidecarHealth>()
            .await
            .map_err(|e| PredictionError::InvalidResponse(e.to_string()))
    }

    /// Returns true if the sidecar is reachable and healthy.
    pub async fn is_available(&self) -> bool {
        match self.health_check().await {
            Ok(h) => h.status == "healthy",
            Err(_) => false,
        }
    }

    /// Send a prediction request with retry logic.
    pub async fn predict(
        &self,
        request: &PredictionRequest,
    ) -> super::error::Result<PredictionResponse> {
        let url = format!("{}/v1/predict/{}", self.base_url, request.model);
        let mut last_err = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                debug!(attempt, ?backoff, "retrying prediction request");
                tokio::time::sleep(backoff).await;
            }

            match self.http.post(&url).json(request).send().await {
                Ok(resp) if resp.status().is_success() => {
                    return resp
                        .json::<PredictionResponse>()
                        .await
                        .map_err(|e| PredictionError::InvalidResponse(e.to_string()));
                }
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let body = resp.text().await.unwrap_or_default();
                    warn!(status, body = %body, "sidecar returned error");
                    last_err = Some(PredictionError::SidecarError {
                        status,
                        message: body,
                    });
                }
                Err(e) => {
                    warn!(error = %e, attempt, "sidecar request failed");
                    last_err = Some(e.into());
                }
            }
        }

        Err(last_err
            .unwrap_or_else(|| PredictionError::SidecarUnavailable("all retries exhausted".into())))
    }

    /// Get the base URL (for logging/diagnostics).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Create an MlSidecarClient from environment variables.
///
/// Returns `None` if `ML_SIDECAR_URL` is not set (sidecar is optional).
pub fn from_env() -> Option<MlSidecarClient> {
    let url = std::env::var("ML_SIDECAR_URL").ok()?;
    if url.is_empty() {
        return None;
    }
    let timeout: u64 = std::env::var("ML_SIDECAR_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000);

    Some(MlSidecarClient::new(&url, timeout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_trims_trailing_slash() {
        let c = MlSidecarClient::new("http://localhost:9000/", 5000);
        assert_eq!(c.base_url(), "http://localhost:9000");
    }

    #[test]
    fn from_env_returns_none_when_unset() {
        // ML_SIDECAR_URL not set in test env
        std::env::remove_var("ML_SIDECAR_URL");
        assert!(from_env().is_none());
    }
}
