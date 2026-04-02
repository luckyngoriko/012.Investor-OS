//! Types for the Strategy Engine (Wave 1b Task 5).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user-defined trading strategy stored in `user_strategies`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub symbols: Vec<String>,
    pub models: Vec<String>,
    pub mode: String, // signal, semi_auto, full_auto
    pub risk_limits: serde_json::Value,
    pub is_active: bool,
}

/// Request body for creating a new strategy.
#[derive(Debug, Deserialize)]
pub struct CreateStrategyRequest {
    pub name: String,
    pub symbols: Vec<String>,
    pub models: Vec<String>,
    /// Defaults to "signal" if not provided.
    pub mode: Option<String>,
    /// Defaults to the table default if not provided.
    pub risk_limits: Option<serde_json::Value>,
}

/// Request body for updating an existing strategy (all fields optional).
#[derive(Debug, Deserialize)]
pub struct UpdateStrategyRequest {
    pub name: Option<String>,
    pub symbols: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub mode: Option<String>,
    pub risk_limits: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}
