use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("user not found")]
    UserNotFound,

    #[error("user is disabled")]
    UserDisabled,

    #[error("account locked until {0}")]
    AccountLocked(chrono::DateTime<chrono::Utc>),

    #[error("password does not meet policy: {0}")]
    WeakPassword(String),

    #[error("invalid or expired token")]
    InvalidToken,

    #[error("refresh token expired or revoked")]
    RefreshTokenInvalid,

    #[error("duplicate email: {0}")]
    DuplicateEmail(String),

    #[error("permission denied")]
    PermissionDenied,

    #[error("password hashing failed: {0}")]
    PasswordHash(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}
