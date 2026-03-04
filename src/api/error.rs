//! Standardized API error envelope (Sprint 103).
//!
//! All error responses follow the shape:
//! ```json
//! {
//!   "success": false,
//!   "error": { "code": "AUTH_LOCKOUT", "message": "...", "details": {...} }
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

/// Standardized API error with HTTP status, machine-readable code, and human-readable message.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// Envelope returned in the HTTP body.
#[derive(Serialize)]
struct ErrorEnvelope {
    success: bool,
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorEnvelope {
            success: false,
            error: ErrorBody {
                code: self.code,
                message: self.message,
                details: self.details,
            },
        };
        (self.status, Json(body)).into_response()
    }
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "BAD_REQUEST",
            message: message.into(),
            details: None,
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "UNAUTHORIZED",
            message: message.into(),
            details: None,
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "FORBIDDEN",
            message: message.into(),
            details: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "NOT_FOUND",
            message: message.into(),
            details: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_ERROR",
            message: message.into(),
            details: None,
        }
    }
}

/// Convert `AuthError` into a standardized `ApiError`.
impl From<crate::auth::AuthError> for ApiError {
    fn from(err: crate::auth::AuthError) -> Self {
        use crate::auth::AuthError;
        match err {
            AuthError::InvalidCredentials => Self {
                status: StatusCode::UNAUTHORIZED,
                code: "INVALID_CREDENTIALS",
                message: "Invalid email or password".to_string(),
                details: None,
            },
            AuthError::UserNotFound => Self::not_found("User not found"),
            AuthError::UserDisabled => Self {
                status: StatusCode::FORBIDDEN,
                code: "USER_DISABLED",
                message: "Account is disabled".to_string(),
                details: None,
            },
            AuthError::AccountLocked(until) => Self {
                status: StatusCode::TOO_MANY_REQUESTS,
                code: "AUTH_LOCKOUT",
                message: "Account locked due to too many failed attempts".to_string(),
                details: Some(json!({ "locked_until": until.to_rfc3339() })),
            },
            AuthError::WeakPassword(reason) => Self {
                status: StatusCode::BAD_REQUEST,
                code: "WEAK_PASSWORD",
                message: format!("Password does not meet policy: {reason}"),
                details: None,
            },
            AuthError::InvalidToken => Self::unauthorized("Invalid or expired token"),
            AuthError::RefreshTokenInvalid => Self {
                status: StatusCode::UNAUTHORIZED,
                code: "REFRESH_TOKEN_INVALID",
                message: "Refresh token expired or revoked".to_string(),
                details: None,
            },
            AuthError::DuplicateEmail(email) => Self {
                status: StatusCode::CONFLICT,
                code: "DUPLICATE_EMAIL",
                message: format!("Email already registered: {email}"),
                details: None,
            },
            AuthError::PermissionDenied => Self::forbidden("Permission denied"),
            AuthError::PasswordHash(_) | AuthError::Database(_) | AuthError::Jwt(_) => {
                Self::internal("Internal server error")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_error_serializes_to_envelope() {
        let err = ApiError::bad_request("missing field");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn api_error_with_details() {
        let err = ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "AUTH_LOCKOUT",
            message: "locked".to_string(),
            details: Some(json!({"locked_until": "2026-03-04T12:00:00Z"})),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn auth_error_converts_to_api_error() {
        let auth_err = crate::auth::AuthError::InvalidCredentials;
        let api_err: ApiError = auth_err.into();
        assert_eq!(api_err.status, StatusCode::UNAUTHORIZED);
        assert_eq!(api_err.code, "INVALID_CREDENTIALS");
    }
}
