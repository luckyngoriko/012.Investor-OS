//! GDPR Compliance Module
//!
//! Implements EU GDPR requirements:
//! - Article 17: Right to erasure ("Right to be forgotten")
//! - Article 20: Right to data portability

use crate::compliance::types::*;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// GDPR Manager
#[derive(Debug, Clone)]
pub struct GdprManager {
    db_pool: Option<sqlx::PgPool>,
}

impl GdprManager {
    /// Create new GDPR manager
    pub fn new(db_pool: Option<sqlx::PgPool>) -> Self {
        Self { db_pool }
    }

    /// Create manager without database (for testing)
    pub fn new_without_db() -> Self {
        Self { db_pool: None }
    }

    /// Request user data deletion (GDPR Article 17)
    ///
    /// # Arguments
    /// * `user_id` - UUID of the user to delete
    ///
    /// # Returns
    /// Deletion request with scheduled deletion date (30 days from now)
    pub async fn forget_user(&self, user_id: &str) -> Result<DeletionRequest, GdprError> {
        info!("Processing GDPR deletion request for user: {}", user_id);

        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|_| GdprError::InvalidUserId)?;

        // Schedule deletion 30 days from now (GDPR allows reasonable time)
        let scheduled_deletion = Utc::now() + Duration::days(30);

        let request = DeletionRequest {
            user_id: user_uuid,
            requested_at: Utc::now(),
            scheduled_deletion,
            status: DeletionStatus::Pending,
        };

        if let Some(ref pool) = self.db_pool {
            // Store deletion request in database
            sqlx::query(
                r#"
                INSERT INTO gdpr_deletion_requests 
                (user_id, requested_at, scheduled_deletion, status)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (user_id) DO UPDATE SET
                    requested_at = EXCLUDED.requested_at,
                    scheduled_deletion = EXCLUDED.scheduled_deletion,
                    status = EXCLUDED.status
                "#
            )
            .bind(user_uuid)
            .bind(request.requested_at)
            .bind(request.scheduled_deletion)
            .bind("pending")
            .execute(pool)
            .await
            .map_err(GdprError::Database)?;
        }

        info!(
            "GDPR deletion scheduled for user {} at {}",
            user_id, scheduled_deletion
        );

        Ok(request)
    }

    /// Export user data (GDPR Article 20 - Data Portability)
    ///
    /// # Arguments
    /// * `user_id` - UUID of the user
    /// * `format` - Export format (JSON, XML, CSV)
    ///
    /// # Returns
    /// Data export containing all user data
    pub async fn export_user_data(
        &self,
        user_id: &str,
        format: ExportFormat,
    ) -> Result<DataExport, GdprError> {
        info!("Processing GDPR data export for user: {}", user_id);

        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|_| GdprError::InvalidUserId)?;

        // Collect user data from all relevant tables
        let user_data = self.collect_user_data(user_uuid).await?;

        let export = DataExport {
            user_id: user_uuid,
            exported_at: Utc::now(),
            data: user_data,
            format,
        };

        info!("GDPR data export completed for user: {}", user_id);

        Ok(export)
    }

    /// Collect all user data for export
    async fn collect_user_data(
        &self,
        user_id: Uuid,
    ) -> Result<serde_json::Value, GdprError> {
        let mut data = serde_json::Map::new();

        if let Some(ref pool) = self.db_pool {
            // User profile
            let profile: Option<(serde_json::Value,)> = sqlx::query_as(
                r#"SELECT to_jsonb(u.*) as profile FROM users u WHERE u.id = $1"#
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(GdprError::Database)?;

            if let Some((profile,)) = profile {
                data.insert("profile".to_string(), profile);
            }

            // Trading history
            let trades: Vec<(serde_json::Value,)> = sqlx::query_as(
                r#"SELECT to_jsonb(t.*) as trade FROM trades t WHERE t.user_id = $1"#
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(GdprError::Database)?;

            data.insert("trades".to_string(), json!(trades.into_iter().map(|(t,)| t).collect::<Vec<_>>()));

            // Portfolio positions
            let positions: Vec<(serde_json::Value,)> = sqlx::query_as(
                r#"SELECT to_jsonb(p.*) as position FROM positions p WHERE p.user_id = $1"#
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(GdprError::Database)?;

            data.insert("positions".to_string(), json!(positions.into_iter().map(|(p,)| p).collect::<Vec<_>>()));

            // Audit logs
            let audit_logs: Vec<(serde_json::Value,)> = sqlx::query_as(
                r#"SELECT to_jsonb(a.*) as log FROM audit_logs a WHERE a.user_id = $1"#
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(GdprError::Database)?;

            data.insert("audit_logs".to_string(), json!(audit_logs.into_iter().map(|(l,)| l).collect::<Vec<_>>()));

            // Activity logs
            let activity: Vec<(serde_json::Value,)> = sqlx::query_as(
                r#"SELECT to_jsonb(l.*) as activity FROM activity_logs l WHERE l.user_id = $1"#
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .map_err(GdprError::Database)?;

            data.insert("activity_logs".to_string(), json!(activity.into_iter().map(|(a,)| a).collect::<Vec<_>>()));
        } else {
            // Mock data for testing
            data.insert("profile".to_string(), json!({
                "id": user_id,
                "email": "user@example.com",
                "created_at": Utc::now(),
            }));
            data.insert("trades".to_string(), json!([]));
            data.insert("positions".to_string(), json!([]));
            data.insert("audit_logs".to_string(), json!([]));
        }

        data.insert("export_metadata".to_string(), json!({
            "exported_at": Utc::now(),
            "system": "Investor OS",
            "version": env!("CARGO_PKG_VERSION"),
            "regulation": "GDPR Article 20",
        }));

        Ok(serde_json::Value::Object(data))
    }

    /// Get deletion status for a user
    pub async fn get_deletion_status(&self, user_id: &str) -> Result<Option<DeletionRequest>, GdprError> {
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|_| GdprError::InvalidUserId)?;

        if let Some(ref pool) = self.db_pool {
            let record: Option<(Uuid, chrono::DateTime<Utc>, chrono::DateTime<Utc>, String)> = sqlx::query_as(
                r#"
                SELECT user_id, requested_at, scheduled_deletion, status
                FROM gdpr_deletion_requests
                WHERE user_id = $1
                "#
            )
            .bind(user_uuid)
            .fetch_optional(pool)
            .await
            .map_err(GdprError::Database)?;

            if let Some((user_id, requested_at, scheduled_deletion, status)) = record {
                let status = match status.as_str() {
                    "pending" => DeletionStatus::Pending,
                    "in_progress" => DeletionStatus::InProgress,
                    "completed" => DeletionStatus::Completed,
                    "failed" => DeletionStatus::Failed,
                    _ => DeletionStatus::Pending,
                };

                return Ok(Some(DeletionRequest {
                    user_id,
                    requested_at,
                    scheduled_deletion,
                    status,
                }));
            }
        }

        Ok(None)
    }

    /// Process pending deletions (called by background job)
    pub async fn process_pending_deletions(&self) -> Result<usize, GdprError> {
        info!("Processing pending GDPR deletions");

        let mut processed = 0;

        if let Some(ref pool) = self.db_pool {
            // Get pending deletions that are due
            let due_deletions: Vec<(Uuid,)> = sqlx::query_as(
                r#"
                SELECT user_id FROM gdpr_deletion_requests
                WHERE status = 'pending'
                AND scheduled_deletion <= NOW()
                "#
            )
            .fetch_all(pool)
            .await
            .map_err(GdprError::Database)?;

            for (user_id,) in due_deletions {
                match self.execute_deletion(user_id).await {
                    Ok(_) => {
                        processed += 1;
                        info!("Successfully deleted user data: {}", user_id);
                    }
                    Err(e) => {
                        error!("Failed to delete user {}: {}", user_id, e);
                        
                        // Mark as failed
                        sqlx::query(
                            "UPDATE gdpr_deletion_requests SET status = 'failed' WHERE user_id = $1"
                        )
                        .bind(user_id)
                        .execute(pool)
                        .await
                        .ok();
                    }
                }
            }
        }

        info!("Processed {} pending deletions", processed);
        Ok(processed)
    }

    /// Execute actual deletion of user data
    async fn execute_deletion(&self, user_id: Uuid) -> Result<(), GdprError> {
        if let Some(ref pool) = self.db_pool {
            let mut tx = pool.begin().await.map_err(GdprError::Database)?;

            // Delete user data from all tables
            // Note: Actual records are soft-deleted, personal data is hard-deleted
            
            sqlx::query("DELETE FROM trades WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(GdprError::Database)?;

            sqlx::query("DELETE FROM positions WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(GdprError::Database)?;

            sqlx::query("UPDATE users SET email = NULL, name = '[deleted]', deleted_at = NOW() WHERE id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(GdprError::Database)?;

            // Mark deletion as completed
            sqlx::query(
                "UPDATE gdpr_deletion_requests SET status = 'completed' WHERE user_id = $1"
            )
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(GdprError::Database)?;

            tx.commit().await.map_err(GdprError::Database)?;
        }

        Ok(())
    }
}

/// GDPR Error type
#[derive(Debug, thiserror::Error)]
pub enum GdprError {
    #[error("Invalid user ID format")]
    InvalidUserId,
    
    #[error("User not found")]
    UserNotFound,
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Export failed: {0}")]
    ExportFailed(String),
    
    #[error("Deletion already pending")]
    DeletionAlreadyPending,
}

/// HTTP handlers for GDPR endpoints
pub mod handlers {
    use super::*;
    use axum::{
        extract::{Query, State},
        http::StatusCode,
        Json,
    };
    use serde::Deserialize;
    use std::sync::Arc;

    #[derive(Deserialize)]
    pub struct ExportQuery {
        pub format: Option<String>,
    }

    /// DELETE /api/v1/gdpr/forget-me
    /// GDPR Article 17: Right to erasure
    pub async fn forget_me(
        State(_state): State<std::sync::Arc<crate::api::AppState>>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        // In real implementation, get user_id from JWT claims
        let user_id = "test-user-id"; // Placeholder

        let manager = GdprManager::new_without_db();
        
        match manager.forget_user(user_id).await {
            Ok(request) => Ok(Json(json!({
                "success": true,
                "message": "User data deletion scheduled",
                "data": {
                    "user_id": request.user_id,
                    "requested_at": request.requested_at,
                    "scheduled_deletion": request.scheduled_deletion,
                    "days_until_deletion": 30,
                }
            }))),
            Err(e) => {
                error!("GDPR forget-me failed: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// GET /api/v1/gdpr/export-data
    /// GDPR Article 20: Right to data portability
    pub async fn export_data(
        State(_state): State<std::sync::Arc<crate::api::AppState>>,
        Query(query): Query<ExportQuery>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        // In real implementation, get user_id from JWT claims
        let user_id = "test-user-id"; // Placeholder

        let format = match query.format.as_deref() {
            Some("xml") => ExportFormat::Xml,
            Some("csv") => ExportFormat::Csv,
            _ => ExportFormat::Json,
        };

        let manager = GdprManager::new_without_db();
        
        match manager.export_user_data(user_id, format).await {
            Ok(export) => Ok(Json(json!({
                "success": true,
                "message": "Data export completed",
                "data": export,
            }))),
            Err(e) => {
                error!("GDPR export failed: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// GET /api/v1/gdpr/data-portability
    /// Alias for export-data with default JSON format
    pub async fn data_portability(
        State(state): State<std::sync::Arc<crate::api::AppState>>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        export_data(State(state), Query(ExportQuery { format: Some("json".to_string()) })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gdpr_manager_creation() {
        let manager = GdprManager::new_without_db();
        assert!(manager.db_pool.is_none());
    }

    #[tokio::test]
    async fn test_forget_user() {
        let manager = GdprManager::new_without_db();
        let user_id = Uuid::new_v4().to_string();
        
        let result = manager.forget_user(&user_id).await;
        assert!(result.is_ok());
        
        let request = result.unwrap();
        assert_eq!(request.user_id.to_string(), user_id);
        assert_eq!(request.status, DeletionStatus::Pending);
        
        // Check that deletion is scheduled 30 days from now
        let days_diff = (request.scheduled_deletion - request.requested_at).num_days();
        assert_eq!(days_diff, 30);
    }

    #[tokio::test]
    async fn test_export_user_data() {
        let manager = GdprManager::new_without_db();
        let user_id = Uuid::new_v4().to_string();
        
        let result = manager.export_user_data(&user_id, ExportFormat::Json).await;
        assert!(result.is_ok());
        
        let export = result.unwrap();
        assert_eq!(export.user_id.to_string(), user_id);
        assert!(export.data.get("profile").is_some());
        assert!(export.data.get("export_metadata").is_some());
    }

    #[test]
    fn test_invalid_user_id() {
        let manager = GdprManager::new_without_db();
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        let result = rt.block_on(manager.forget_user("invalid-uuid"));
        assert!(matches!(result, Err(GdprError::InvalidUserId)));
    }
}
