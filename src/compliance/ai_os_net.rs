//! AI-OS.NET Integration Client
//!
//! HTTP client for AI-OS.NET compliance API

use crate::compliance::ai_os_net_url;
use crate::compliance::types::*;
use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

/// AI-OS.NET Compliance Client
#[derive(Debug, Clone)]
pub struct ComplianceClient {
    client: Client,
    base_url: String,
    system_id: Uuid,
}

/// Error type for compliance operations
#[derive(Debug, thiserror::Error)]
pub enum ComplianceError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    Api { status: StatusCode, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("System not registered")]
    NotRegistered,

    #[error("Compliance score too low: {0}")]
    LowComplianceScore(u8),
}

impl ComplianceClient {
    /// Create new compliance client
    pub fn new(base_url: impl Into<String>, system_name: &str) -> Result<Self, ComplianceError> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        let base_url = base_url.into();
        let system_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, system_name.as_bytes());

        info!(
            "ComplianceClient created for system: {} (ID: {})",
            system_name, system_id
        );

        Ok(Self {
            client,
            base_url,
            system_id,
        })
    }

    /// Create client from environment variables
    pub fn from_env() -> Result<Self, ComplianceError> {
        let url = ai_os_net_url();
        let system_name =
            std::env::var("AI_SYSTEM_NAME").unwrap_or_else(|_| "investor-os".to_string());

        Self::new(url, &system_name)
    }

    /// Register AI system with AI-OS.NET
    pub async fn register_system(
        &self,
        name: &str,
        description: &str,
        risk_level: RiskLevel,
    ) -> Result<AISystemRegistration, ComplianceError> {
        let url = format!("{}/api/v1/compliance/register", self.base_url);

        let payload = json!({
            "id": self.system_id,
            "name": name,
            "description": description,
            "risk_level": risk_level,
            "provider": "Investor OS",
            "registered_at": Utc::now(),
        });

        let response = self.client.post(&url).json(&payload).send().await?;

        if response.status().is_success() {
            let registration: AISystemRegistration = response.json().await?;
            info!("AI system registered: {}", registration.id);
            Ok(registration)
        } else {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            error!("Failed to register system: {} - {}", status, message);
            Err(ComplianceError::Api { status, message })
        }
    }

    /// Get compliance score for the system
    pub async fn get_compliance_score(&self) -> Result<ComplianceScore, ComplianceError> {
        let url = format!(
            "{}/api/v1/compliance/models/{}/score",
            self.base_url, self.system_id
        );

        let response = self.client.get(&url).send().await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ComplianceError::NotRegistered);
        }

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct ScoreResponse {
                success: bool,
                data: serde_json::Value,
            }

            let result: ScoreResponse = response.json().await?;
            let score = result.data["score"].as_u64().unwrap_or(0) as u8;

            info!("Compliance score retrieved: {}", score);
            Ok(ComplianceScore::new(score))
        } else {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            Err(ComplianceError::Api { status, message })
        }
    }

    /// Calculate compliance score
    pub async fn calculate_score(&self) -> Result<ComplianceScore, ComplianceError> {
        let url = format!(
            "{}/api/v1/compliance/models/{}/score/calculate",
            self.base_url, self.system_id
        );

        let response = self.client.post(&url).send().await?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct ScoreResponse {
                success: bool,
                data: serde_json::Value,
            }

            let result: ScoreResponse = response.json().await?;
            let score = result.data["score"].as_u64().unwrap_or(0) as u8;

            info!("Compliance score calculated: {}", score);
            Ok(ComplianceScore::new(score))
        } else {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            Err(ComplianceError::Api { status, message })
        }
    }

    /// Log AI decision (EU AI Act Article 12 requirement)
    pub async fn log_ai_decision(
        &self,
        decision_type: DecisionType,
        input_hash: &str,
        output: &serde_json::Value,
        confidence: f64,
        explanation: &str,
    ) -> Result<AIDecisionLog, ComplianceError> {
        let url = format!("{}/audit/events", self.base_url);

        let event = json!({
            "model_id": self.system_id,
            "event_type": "ai_decision",
            "timestamp": Utc::now(),
            "input_data": input_hash,
            "output_data": output,
            "confidence": confidence,
            "metadata": {
                "decision_type": decision_type,
                "explanation": explanation,
                "system_version": env!("CARGO_PKG_VERSION"),
            }
        });

        let response = self.client.post(&url).json(&event).send().await?;

        if response.status().is_success() {
            let log_entry = AIDecisionLog {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                system_id: self.system_id,
                decision_type,
                input_data_hash: input_hash.to_string(),
                output_data: output.clone(),
                confidence,
                explanation: explanation.to_string(),
                human_reviewed: false,
                human_decision: None,
            };

            info!("AI decision logged: {:?}", decision_type);
            Ok(log_entry)
        } else {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            error!("Failed to log AI decision: {} - {}", status, message);
            Err(ComplianceError::Api { status, message })
        }
    }

    /// Get compliance report
    pub async fn get_compliance_report(&self) -> Result<ComplianceReport, ComplianceError> {
        let url = format!(
            "{}/api/v1/compliance/models/{}/report",
            self.base_url, self.system_id
        );

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct ReportResponse {
                success: bool,
                data: ComplianceReport,
            }

            let result: ReportResponse = response.json().await?;
            info!("Compliance report retrieved");
            Ok(result.data)
        } else {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            Err(ComplianceError::Api { status, message })
        }
    }

    /// Check compliance and warn if score is low
    pub async fn check_compliance(&self) -> Result<ComplianceScore, ComplianceError> {
        let score = self.get_compliance_score().await?;

        if !score.is_acceptable() {
            warn!(
                "Compliance score is below threshold: {} (minimum: 70)",
                score.value()
            );
        }

        Ok(score)
    }

    /// Add human oversight decision
    pub async fn add_human_decision(
        &self,
        event_id: Uuid,
        decision: HumanOversightDecision,
        reason: &str,
        verifier_id: Uuid,
    ) -> Result<HumanDecision, ComplianceError> {
        let url = format!("{}/audit/events/{}/decisions", self.base_url, event_id);

        let payload = json!({
            "decision": decision,
            "reason": reason,
            "verifier_id": verifier_id,
        });

        let response = self.client.post(&url).json(&payload).send().await?;

        if response.status().is_success() {
            let human_decision = HumanDecision {
                verifier_id,
                decision,
                reason: reason.to_string(),
                timestamp: Utc::now(),
            };

            info!("Human decision added for event: {}", event_id);
            Ok(human_decision)
        } else {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            Err(ComplianceError::Api { status, message })
        }
    }

    /// Get pending human decisions
    pub async fn get_pending_decisions(&self) -> Result<Vec<AIDecisionLog>, ComplianceError> {
        let url = format!("{}/audit/pending-decisions", self.base_url);

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct PendingResponse {
                success: bool,
                data: serde_json::Value,
            }

            let result: PendingResponse = response.json().await?;
            let events: Vec<AIDecisionLog> = serde_json::from_value(result.data)?;
            Ok(events)
        } else {
            let status = response.status();
            let message = response.text().await.unwrap_or_default();
            Err(ComplianceError::Api { status, message })
        }
    }
}

/// HTTP handlers for compliance endpoints
pub mod handlers {
    use super::*;
    use axum::{extract::State, http::StatusCode, Json};
    use std::sync::Arc;

    /// GET /api/v1/compliance/score
    pub async fn get_compliance_score(
        State(_state): State<std::sync::Arc<crate::api::AppState>>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        let client = ComplianceClient::from_env().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        match client.get_compliance_score().await {
            Ok(score) => Ok(Json(json!({
                "success": true,
                "data": {
                    "score": score.value(),
                    "acceptable": score.is_acceptable(),
                }
            }))),
            Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    }

    /// GET /api/v1/compliance/report
    pub async fn get_compliance_report(
        State(_state): State<std::sync::Arc<crate::api::AppState>>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        let client = ComplianceClient::from_env().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        match client.get_compliance_report().await {
            Ok(report) => Ok(Json(json!({
                "success": true,
                "data": report
            }))),
            Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_client_creation() {
        let client = ComplianceClient::new("http://localhost:8080", "test-system");
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_system_id_deterministic() {
        // Same name should produce same ID
        let client1 = ComplianceClient::new("http://localhost:8080", "test-system").unwrap();
        let client2 = ComplianceClient::new("http://localhost:8080", "test-system").unwrap();

        assert_eq!(client1.system_id, client2.system_id);
    }

    #[test]
    fn test_compliance_error_display() {
        let err = ComplianceError::NotRegistered;
        assert_eq!(err.to_string(), "System not registered");

        let err = ComplianceError::LowComplianceScore(50);
        assert!(err.to_string().contains("50"));
    }
}
