//! KYC/AML Verification via Sumsub
//!
//! Wave 4 Task 23: Identity verification for investor onboarding.
//!
//! - SumsubClient: creates applicants, retrieves access tokens, checks verification status
//! - HMAC-SHA256 request signing per Sumsub API requirements
//! - KycStatus enum with DB persistence via runtime sqlx

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::{PgPool, Row};
use tracing::{error, info, warn};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

// ── KYC Status ───────────────────────────────────────────────────────

/// KYC verification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KycStatus {
    NotStarted,
    Pending,
    Approved,
    Rejected,
    Retry,
}

impl KycStatus {
    /// Parse from database string value
    pub fn from_db(s: &str) -> Self {
        match s {
            "not_started" => Self::NotStarted,
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            "retry" => Self::Retry,
            _ => Self::NotStarted,
        }
    }

    /// Convert to database string value
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Retry => "retry",
        }
    }
}

impl std::fmt::Display for KycStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_db_str())
    }
}

// ── Error type ───────────────────────────────────────────────────────

/// KYC operation errors
#[derive(Debug, thiserror::Error)]
pub enum KycError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("HMAC error: {0}")]
    Hmac(String),

    #[error("Sumsub API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("KYC not found for user {0}")]
    NotFound(Uuid),
}

// ── Sumsub API response types ────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SumsubApplicant {
    pub id: String,
    #[serde(default)]
    pub review_status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SumsubAccessToken {
    pub token: String,
    #[serde(rename = "userId")]
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SumsubReviewResult {
    pub review_answer: Option<String>,
    pub review_reject_type: Option<String>,
    #[serde(default)]
    pub reject_labels: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SumsubApplicantStatus {
    pub id: String,
    pub review_status: Option<String>,
    pub review_result: Option<SumsubReviewResult>,
}

// ── Webhook payload ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SumsubWebhookPayload {
    pub applicant_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub review_result: Option<SumsubReviewResult>,
    pub review_status: Option<String>,
    #[serde(default)]
    pub external_user_id: Option<String>,
}

// ── Sumsub Client ────────────────────────────────────────────────────

/// HTTP client for the Sumsub KYC/AML verification API.
///
/// Authenticates requests using HMAC-SHA256 signatures as required
/// by the Sumsub REST API.
#[derive(Clone)]
pub struct SumsubClient {
    client: Client,
    app_token: String,
    secret_key: String,
    base_url: String,
}

impl std::fmt::Debug for SumsubClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SumsubClient")
            .field("base_url", &self.base_url)
            .field("app_token", &"[REDACTED]")
            .field("secret_key", &"[REDACTED]")
            .finish()
    }
}

impl SumsubClient {
    /// Create a new Sumsub client.
    ///
    /// - `app_token`: Sumsub application token
    /// - `secret_key`: Sumsub secret key for HMAC signing
    /// - `base_url`: API base URL (default: `https://api.sumsub.com`)
    pub fn new(app_token: String, secret_key: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            app_token,
            secret_key,
            base_url,
        }
    }

    /// Create client from environment variables.
    ///
    /// Reads `SUMSUB_APP_TOKEN`, `SUMSUB_SECRET_KEY`, and optionally
    /// `SUMSUB_BASE_URL` (defaults to `https://api.sumsub.com`).
    ///
    /// Returns `None` if required env vars are missing.
    pub fn from_env() -> Option<Self> {
        let app_token = std::env::var("SUMSUB_APP_TOKEN").ok()?;
        let secret_key = std::env::var("SUMSUB_SECRET_KEY").ok()?;
        let base_url = std::env::var("SUMSUB_BASE_URL")
            .unwrap_or_else(|_| "https://api.sumsub.com".to_string());
        Some(Self::new(app_token, secret_key, base_url))
    }

    /// Generate HMAC-SHA256 signature for a Sumsub API request.
    fn sign(&self, ts: i64, method: &str, path: &str, body: &[u8]) -> Result<String, KycError> {
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .map_err(|e| KycError::Hmac(e.to_string()))?;

        // Sumsub signing: ts + method + path + body
        mac.update(ts.to_string().as_bytes());
        mac.update(method.as_bytes());
        mac.update(path.as_bytes());
        mac.update(body);

        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }

    /// Create a new applicant in Sumsub.
    ///
    /// The `user_id` is used as the externalUserId for correlation.
    pub async fn create_applicant(
        &self,
        user_id: Uuid,
        email: &str,
    ) -> Result<SumsubApplicant, KycError> {
        let path = "/resources/applicants?levelName=basic-kyc-level";
        let ts = Utc::now().timestamp();
        let body = serde_json::json!({
            "externalUserId": user_id.to_string(),
            "email": email,
        });
        let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
        let sig = self.sign(ts, "POST", path, &body_bytes)?;

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .header("X-App-Token", &self.app_token)
            .header("X-App-Access-Ts", ts.to_string())
            .header("X-App-Access-Sig", &sig)
            .header("Content-Type", "application/json")
            .body(body_bytes)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let msg = resp.text().await.unwrap_or_default();
            error!(status = %status, "Sumsub create_applicant failed: {}", msg);
            return Err(KycError::Api {
                status: status.as_u16(),
                message: msg,
            });
        }

        let applicant: SumsubApplicant = resp.json().await?;
        info!(applicant_id = %applicant.id, user_id = %user_id, "Sumsub applicant created");
        Ok(applicant)
    }

    /// Get an SDK access token for a Sumsub applicant.
    ///
    /// The returned token is used by the frontend Sumsub WebSDK
    /// to render the verification flow.
    pub async fn get_access_token(
        &self,
        applicant_id: &str,
    ) -> Result<SumsubAccessToken, KycError> {
        let path = format!(
            "/resources/accessTokens?userId={}&levelName=basic-kyc-level",
            applicant_id
        );
        let ts = Utc::now().timestamp();
        let sig = self.sign(ts, "POST", &path, &[])?;

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .post(&url)
            .header("X-App-Token", &self.app_token)
            .header("X-App-Access-Ts", ts.to_string())
            .header("X-App-Access-Sig", &sig)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(KycError::Api {
                status: status.as_u16(),
                message: msg,
            });
        }

        Ok(resp.json().await?)
    }

    /// Get the current verification status of a Sumsub applicant.
    pub async fn get_verification_status(
        &self,
        applicant_id: &str,
    ) -> Result<SumsubApplicantStatus, KycError> {
        let path = format!(
            "/resources/applicants/{}/requiredIdDocsStatus",
            applicant_id
        );
        let ts = Utc::now().timestamp();
        let sig = self.sign(ts, "GET", &path, &[])?;

        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .header("X-App-Token", &self.app_token)
            .header("X-App-Access-Ts", ts.to_string())
            .header("X-App-Access-Sig", &sig)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let msg = resp.text().await.unwrap_or_default();
            return Err(KycError::Api {
                status: status.as_u16(),
                message: msg,
            });
        }

        Ok(resp.json().await?)
    }
}

// ── Database helpers ─────────────────────────────────────────────────

/// Get the KYC status for a user.
///
/// Returns `KycStatus::NotStarted` if no record exists.
pub async fn get_user_kyc_status(pool: &PgPool, user_id: Uuid) -> Result<KycStatus, KycError> {
    let row = sqlx::query("SELECT status FROM kyc_verifications WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(r) => {
            let status_str: String = r.try_get("status")?;
            Ok(KycStatus::from_db(&status_str))
        }
        None => Ok(KycStatus::NotStarted),
    }
}

/// Check whether a user's KYC is approved.
pub async fn is_kyc_approved(pool: &PgPool, user_id: Uuid) -> Result<bool, KycError> {
    let status = get_user_kyc_status(pool, user_id).await?;
    Ok(status == KycStatus::Approved)
}

/// Insert or update a KYC verification record.
pub async fn upsert_kyc(
    pool: &PgPool,
    user_id: Uuid,
    applicant_id: &str,
    status: KycStatus,
    rejection_reason: Option<&str>,
) -> Result<(), KycError> {
    let verified_at: Option<DateTime<Utc>> = if status == KycStatus::Approved {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query(
        r#"
        INSERT INTO kyc_verifications (user_id, sumsub_applicant_id, status, rejection_reason, verified_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        ON CONFLICT (user_id) DO UPDATE SET
            sumsub_applicant_id = EXCLUDED.sumsub_applicant_id,
            status = EXCLUDED.status,
            rejection_reason = EXCLUDED.rejection_reason,
            verified_at = EXCLUDED.verified_at,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(applicant_id)
    .bind(status.as_db_str())
    .bind(rejection_reason)
    .bind(verified_at)
    .execute(pool)
    .await?;

    info!(user_id = %user_id, status = %status, "KYC record upserted");
    Ok(())
}

/// Get the Sumsub applicant ID for a user (if exists).
pub async fn get_applicant_id(pool: &PgPool, user_id: Uuid) -> Result<Option<String>, KycError> {
    let row = sqlx::query("SELECT sumsub_applicant_id FROM kyc_verifications WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(r) => {
            let id: Option<String> = r.try_get("sumsub_applicant_id")?;
            Ok(id)
        }
        None => Ok(None),
    }
}

// ── Unit tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyc_status_from_db() {
        assert_eq!(KycStatus::from_db("not_started"), KycStatus::NotStarted);
        assert_eq!(KycStatus::from_db("pending"), KycStatus::Pending);
        assert_eq!(KycStatus::from_db("approved"), KycStatus::Approved);
        assert_eq!(KycStatus::from_db("rejected"), KycStatus::Rejected);
        assert_eq!(KycStatus::from_db("retry"), KycStatus::Retry);
        assert_eq!(KycStatus::from_db("unknown"), KycStatus::NotStarted);
    }

    #[test]
    fn test_kyc_status_as_db_str() {
        assert_eq!(KycStatus::NotStarted.as_db_str(), "not_started");
        assert_eq!(KycStatus::Pending.as_db_str(), "pending");
        assert_eq!(KycStatus::Approved.as_db_str(), "approved");
        assert_eq!(KycStatus::Rejected.as_db_str(), "rejected");
        assert_eq!(KycStatus::Retry.as_db_str(), "retry");
    }

    #[test]
    fn test_kyc_status_roundtrip() {
        let statuses = [
            KycStatus::NotStarted,
            KycStatus::Pending,
            KycStatus::Approved,
            KycStatus::Rejected,
            KycStatus::Retry,
        ];
        for s in &statuses {
            assert_eq!(KycStatus::from_db(s.as_db_str()), *s);
        }
    }

    #[test]
    fn test_kyc_status_display() {
        assert_eq!(format!("{}", KycStatus::Approved), "approved");
        assert_eq!(format!("{}", KycStatus::Pending), "pending");
    }

    #[test]
    fn test_sumsub_client_new() {
        let client = SumsubClient::new(
            "test_token".to_string(),
            "test_secret".to_string(),
            "https://api.sumsub.com".to_string(),
        );
        assert_eq!(client.base_url, "https://api.sumsub.com");
        // Debug output should redact secrets
        let debug = format!("{:?}", client);
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("test_secret"));
    }

    #[test]
    fn test_sumsub_client_sign() {
        let client = SumsubClient::new(
            "token".to_string(),
            "secret".to_string(),
            "https://api.sumsub.com".to_string(),
        );
        let sig = client.sign(1700000000, "GET", "/resources/applicants", &[]);
        assert!(sig.is_ok());
        let sig = sig.unwrap();
        // HMAC-SHA256 produces 64 hex chars
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn test_sumsub_client_sign_deterministic() {
        let client = SumsubClient::new(
            "token".to_string(),
            "my_secret".to_string(),
            "https://api.sumsub.com".to_string(),
        );
        let sig1 = client
            .sign(1700000000, "POST", "/resources/applicants", b"{}")
            .unwrap();
        let sig2 = client
            .sign(1700000000, "POST", "/resources/applicants", b"{}")
            .unwrap();
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_sumsub_client_sign_different_body() {
        let client = SumsubClient::new(
            "token".to_string(),
            "my_secret".to_string(),
            "https://api.sumsub.com".to_string(),
        );
        let sig1 = client
            .sign(1700000000, "POST", "/resources/applicants", b"{}")
            .unwrap();
        let sig2 = client
            .sign(1700000000, "POST", "/resources/applicants", b"{\"a\":1}")
            .unwrap();
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_sumsub_client_from_env_missing() {
        // Ensure env vars are not set
        std::env::remove_var("SUMSUB_APP_TOKEN");
        std::env::remove_var("SUMSUB_SECRET_KEY");
        assert!(SumsubClient::from_env().is_none());
    }

    #[test]
    fn test_webhook_payload_deserialize() {
        let json = r#"{
            "applicantId": "abc123",
            "type": "applicantReviewed",
            "reviewResult": {
                "reviewAnswer": "GREEN"
            },
            "reviewStatus": "completed",
            "externalUserId": "550e8400-e29b-41d4-a716-446655440000"
        }"#;
        let payload: SumsubWebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.applicant_id, "abc123");
        assert_eq!(payload.event_type, "applicantReviewed");
        assert_eq!(
            payload.review_result.as_ref().unwrap().review_answer,
            Some("GREEN".to_string())
        );
        assert_eq!(
            payload.external_user_id,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_kyc_error_display() {
        let err = KycError::NotFound(Uuid::nil());
        assert!(format!("{}", err).contains("KYC not found"));

        let err = KycError::Api {
            status: 400,
            message: "bad request".to_string(),
        };
        assert!(format!("{}", err).contains("400"));
    }
}
