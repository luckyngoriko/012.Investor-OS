//! Production-grade RBAC & Multi-User Authentication (Sprint 101)
//!
//! PostgreSQL-backed users/sessions, Argon2id password hashing, JWT access tokens.

pub mod api_keys;
pub mod audit;
pub mod cleanup;
pub mod error;
pub mod jwt;
pub mod password;
pub mod repository;
pub mod seed;
pub mod service;
pub mod totp;
pub mod types;

pub use error::AuthError;
pub use jwt::JwtConfig;
pub use service::{AuthService, LockoutConfig};
pub use types::{
    AccessClaims, AuthUser, CreateApiKeyRequest, CreateUserRequest, LoginData, LoginRequest,
    LoginResponse, LogoutRequest, RefreshRequest, TotpChallenge, TotpCodeRequest,
    TotpVerifyRequest, UpdateUserRequest, UserInfo, UserRole,
};
