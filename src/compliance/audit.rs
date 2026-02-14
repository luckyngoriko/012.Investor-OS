//! EU AI Act Audit Logging
//!
//! Implements Article 12 requirements for logging AI decisions:
//! - Automatic logging of all AI system outputs
//! - Human oversight tracking
//! - Immutable audit trail

use crate::compliance::types::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Audit logger for AI decisions
#[derive(Debug, Clone)]
pub struct AuditLogger {
    db_pool: Option<sqlx::PgPool>,
    buffer: Vec<AIDecisionLog>,
    buffer_size: usize,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(db_pool: Option<sqlx::PgPool>) -> Self {
        Self {
            db_pool,
            buffer: Vec::with_capacity(100),
            buffer_size: 100,
        }
    }

    /// Create logger without database (for testing)
    pub fn new_without_db() -> Self {
        Self {
            db_pool: None,
            buffer: Vec::with_capacity(100),
            buffer_size: 100,
        }
    }

    /// Get buffer size (for testing)
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty (for testing)
    pub fn is_buffer_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Log an AI decision (EU AI Act Article 12 requirement)
    ///
    /// # Arguments
    /// * `system_id` - UUID of the AI system
    /// * `decision_type` - Type of decision (trading, risk, etc.)
    /// * `input_data` - Input data (will be hashed for privacy)
    /// * `output_data` - AI system output
    /// * `confidence` - Confidence score (0.0 - 1.0)
    /// * `explanation` - Human-readable explanation
    pub async fn log_decision(
        &mut self,
        system_id: Uuid,
        decision_type: DecisionType,
        input_data: &serde_json::Value,
        output_data: &serde_json::Value,
        confidence: f64,
        explanation: &str,
    ) -> Result<AIDecisionLog, AuditError> {
        let input_hash = self.hash_input(input_data);
        
        let log_entry = AIDecisionLog {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            system_id,
            decision_type,
            input_data_hash: input_hash,
            output_data: output_data.clone(),
            confidence,
            explanation: explanation.to_string(),
            human_reviewed: false,
            human_decision: None,
        };

        // Add to buffer
        self.buffer.push(log_entry.clone());

        // Flush if buffer is full
        if self.buffer.len() >= self.buffer_size {
            self.flush().await?;
        }

        info!(
            "AI decision logged: system={}, type={:?}, confidence={:.2}",
            system_id, decision_type, confidence
        );

        Ok(log_entry)
    }

    /// Log HRM (Hierarchical Reasoning Model) decision
    #[cfg(feature = "eu_compliance")]
    pub async fn log_hrm_decision(
        &mut self,
        signals: &crate::hrm::HrmInput,
        output: &crate::hrm::HrmOutput,
    ) -> Result<AIDecisionLog, AuditError> {
        let system_id = Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            b"investor-os-hrm",
        );

        let input_data = json!({
            "pegy": signals.pegy,
            "insider_sentiment": signals.insider_sentiment,
            "social_sentiment": signals.social_sentiment,
            "vix": signals.vix,
        });

        let output_data = json!({
            "conviction": output.conviction,
            "recommended_action": format!("{:?}", output.recommended_action),
            "confidence": output.confidence,
        });

        let explanation = format!(
            "HRM generated trading signal with {:.1}% conviction for {:?} action",
            output.conviction * 100.0,
            output.recommended_action
        );

        self.log_decision(
            system_id,
            DecisionType::TradingSignal,
            &input_data,
            &output_data,
            output.confidence,
            &explanation,
        ).await
    }

    /// Log risk assessment decision
    pub async fn log_risk_assessment(
        &mut self,
        portfolio_value: f64,
        risk_metrics: &serde_json::Value,
        assessment: &str,
    ) -> Result<AIDecisionLog, AuditError> {
        let system_id = Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            b"investor-os-risk",
        );

        let input_data = json!({
            "portfolio_value": portfolio_value,
            "risk_metrics": risk_metrics,
        });

        let output_data = json!({
            "assessment": assessment,
            "timestamp": Utc::now(),
        });

        let explanation = format!("Risk assessment: {}", assessment);

        self.log_decision(
            system_id,
            DecisionType::RiskAssessment,
            &input_data,
            &output_data,
            0.95,
            &explanation,
        ).await
    }

    /// Add human oversight decision to existing log entry
    pub async fn add_human_decision(
        &mut self,
        log_id: Uuid,
        decision: HumanOversightDecision,
        reason: &str,
        verifier_id: Uuid,
    ) -> Result<(), AuditError> {
        let human_decision = HumanDecision {
            verifier_id,
            decision,
            reason: reason.to_string(),
            timestamp: Utc::now(),
        };

        if let Some(ref pool) = self.db_pool {
            sqlx::query(
                r#"
                UPDATE ai_decision_logs
                SET human_reviewed = true,
                    human_decision = $1,
                    updated_at = NOW()
                WHERE id = $2
                "#
            )
            .bind(sqlx::types::Json(&human_decision))
            .bind(log_id)
            .execute(pool)
            .await
            .map_err(AuditError::Database)?;
        }

        info!(
            "Human decision added to log {}: {:?}",
            log_id, decision
        );

        Ok(())
    }

    /// Flush buffered logs to database
    pub async fn flush(&mut self) -> Result<(), AuditError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        if let Some(ref pool) = self.db_pool {
            let mut tx = pool.begin().await.map_err(AuditError::Database)?;

            for log in &self.buffer {
                sqlx::query(
                    r#"
                    INSERT INTO ai_decision_logs
                    (id, timestamp, system_id, decision_type, input_data_hash,
                     output_data, confidence, explanation, human_reviewed)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    ON CONFLICT (id) DO NOTHING
                    "#
                )
                .bind(log.id)
                .bind(log.timestamp)
                .bind(log.system_id)
                .bind(format!("{:?}", log.decision_type))
                .bind(&log.input_data_hash)
                .bind(&log.output_data)
                .bind(log.confidence)
                .bind(&log.explanation)
                .bind(log.human_reviewed)
                .execute(&mut *tx)
                .await
                .map_err(AuditError::Database)?;
            }

            tx.commit().await.map_err(AuditError::Database)?;
        }

        info!("Flushed {} audit log entries", self.buffer.len());
        self.buffer.clear();

        Ok(())
    }

    /// Query audit logs
    pub async fn query_logs(
        &self,
        system_id: Option<Uuid>,
        decision_type: Option<DecisionType>,
        start_date: Option<chrono::DateTime<Utc>>,
        end_date: Option<chrono::DateTime<Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AIDecisionLog>, AuditError> {
        let mut logs = Vec::new();

        if let Some(ref pool) = self.db_pool {
            let records: Vec<(Uuid, chrono::DateTime<Utc>, Uuid, String, String, serde_json::Value, f64, String, bool)> = sqlx::query_as(
                r#"
                SELECT id, timestamp, system_id, decision_type, input_data_hash,
                       output_data, confidence, explanation, human_reviewed
                FROM ai_decision_logs
                WHERE ($1::uuid IS NULL OR system_id = $1)
                  AND ($2::text IS NULL OR decision_type = $2)
                  AND ($3::timestamptz IS NULL OR timestamp >= $3)
                  AND ($4::timestamptz IS NULL OR timestamp <= $4)
                ORDER BY timestamp DESC
                LIMIT $5 OFFSET $6
                "#
            )
            .bind(system_id)
            .bind(decision_type.map(|dt| format!("{:?}", dt)))
            .bind(start_date)
            .bind(end_date)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(AuditError::Database)?;

            for (id, timestamp, system_id, decision_type, input_hash, output, confidence, explanation, human_reviewed) in records {
                let decision_type = match decision_type.as_str() {
                    "TradingSignal" => DecisionType::TradingSignal,
                    "RiskAssessment" => DecisionType::RiskAssessment,
                    "PortfolioRebalancing" => DecisionType::PortfolioRebalancing,
                    "MarketRegimeDetection" => DecisionType::MarketRegimeDetection,
                    _ => DecisionType::Other,
                };

                logs.push(AIDecisionLog {
                    id,
                    timestamp,
                    system_id,
                    decision_type,
                    input_data_hash: input_hash,
                    output_data: output,
                    confidence,
                    explanation,
                    human_reviewed,
                    human_decision: None, // Would need separate query
                });
            }
        }

        Ok(logs)
    }

    /// Get logs requiring human review (for high-risk decisions)
    pub async fn get_pending_review(&self) -> Result<Vec<AIDecisionLog>, AuditError> {
        if let Some(ref pool) = self.db_pool {
            let records: Vec<(Uuid, chrono::DateTime<Utc>, Uuid, String, String, serde_json::Value, f64, String, bool)> = sqlx::query_as(
                r#"
                SELECT id, timestamp, system_id, decision_type, input_data_hash,
                       output_data, confidence, explanation, human_reviewed
                FROM ai_decision_logs
                WHERE human_reviewed = false
                  AND decision_type IN ('TradingSignal', 'RiskAssessment')
                  AND confidence < 0.8
                ORDER BY timestamp DESC
                LIMIT 100
                "#
            )
            .fetch_all(pool)
            .await
            .map_err(AuditError::Database)?;

            let mut logs = Vec::new();
            for (id, timestamp, system_id, decision_type, input_hash, output, confidence, explanation, human_reviewed) in records {
                let decision_type = match decision_type.as_str() {
                    "TradingSignal" => DecisionType::TradingSignal,
                    "RiskAssessment" => DecisionType::RiskAssessment,
                    _ => DecisionType::Other,
                };

                logs.push(AIDecisionLog {
                    id,
                    timestamp,
                    system_id,
                    decision_type,
                    input_data_hash: input_hash,
                    output_data: output,
                    confidence,
                    explanation,
                    human_reviewed,
                    human_decision: None,
                });
            }

            return Ok(logs);
        }

        Ok(Vec::new())
    }

    /// Hash input data for privacy (store hash, not raw data)
    fn hash_input(&self, input: &serde_json::Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        input.to_string().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Audit error type
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Buffer overflow")]
    BufferOverflow,
    
    #[error("Log entry not found: {0}")]
    NotFound(Uuid),
}

/// HTTP handlers for audit endpoints
pub mod handlers {
    use super::*;
    use axum::{
        extract::{Query, State},
        http::StatusCode,
        Json,
    };
    use serde::Deserialize;
    use chrono::{DateTime, Utc};
    use std::sync::Arc;

    #[derive(Deserialize)]
    pub struct QueryParams {
        pub system_id: Option<Uuid>,
        pub decision_type: Option<String>,
        pub start_date: Option<DateTime<Utc>>,
        pub end_date: Option<DateTime<Utc>>,
        pub limit: Option<i64>,
        pub offset: Option<i64>,
    }

    #[derive(Deserialize)]
    pub struct LogEventRequest {
        pub system_id: Uuid,
        pub decision_type: String,
        pub input_data: serde_json::Value,
        pub output_data: serde_json::Value,
        pub confidence: f64,
        pub explanation: String,
    }

    /// POST /api/v1/compliance/audit-log
    pub async fn log_event(
        State(_state): State<std::sync::Arc<crate::api::AppState>>,
        Json(req): Json<LogEventRequest>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        let mut logger = AuditLogger::new_without_db();
        
        let decision_type = match req.decision_type.as_str() {
            "trading_signal" => DecisionType::TradingSignal,
            "risk_assessment" => DecisionType::RiskAssessment,
            "portfolio_rebalancing" => DecisionType::PortfolioRebalancing,
            "market_regime" => DecisionType::MarketRegimeDetection,
            _ => DecisionType::Other,
        };

        match logger.log_decision(
            req.system_id,
            decision_type,
            &req.input_data,
            &req.output_data,
            req.confidence,
            &req.explanation,
        ).await {
            Ok(log) => Ok(Json(json!({
                "success": true,
                "data": {
                    "log_id": log.id,
                    "timestamp": log.timestamp,
                    "message": "Audit log created",
                }
            }))),
            Err(e) => {
                error!("Failed to create audit log: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// GET /api/v1/compliance/audit-log
    pub async fn query_events(
        State(_state): State<std::sync::Arc<crate::api::AppState>>,
        Query(params): Query<QueryParams>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        let logger = AuditLogger::new_without_db();

        let decision_type = params.decision_type.and_then(|dt| match dt.as_str() {
            "trading_signal" => Some(DecisionType::TradingSignal),
            "risk_assessment" => Some(DecisionType::RiskAssessment),
            _ => None,
        });

        match logger.query_logs(
            params.system_id,
            decision_type,
            params.start_date,
            params.end_date,
            params.limit.unwrap_or(100),
            params.offset.unwrap_or(0),
        ).await {
            Ok(logs) => Ok(Json(json!({
                "success": true,
                "data": {
                    "logs": logs,
                    "count": logs.len(),
                }
            }))),
            Err(e) => {
                error!("Failed to query audit logs: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let logger = AuditLogger::new_without_db();
        assert!(logger.db_pool.is_none());
        assert!(logger.buffer.is_empty());
    }

    #[tokio::test]
    async fn test_log_decision() {
        let mut logger = AuditLogger::new_without_db();
        
        let system_id = Uuid::new_v4();
        let input = json!({"test": "input"});
        let output = json!({"test": "output"});

        let result = logger.log_decision(
            system_id,
            DecisionType::TradingSignal,
            &input,
            &output,
            0.85,
            "Test decision",
        ).await;

        assert!(result.is_ok());
        let log = result.unwrap();
        assert_eq!(log.system_id, system_id);
        assert_eq!(log.decision_type, DecisionType::TradingSignal);
        assert!(!log.human_reviewed);
    }

    #[test]
    fn test_hash_input() {
        let logger = AuditLogger::new_without_db();
        let input = json!({"key": "value"});
        
        let hash1 = logger.hash_input(&input);
        let hash2 = logger.hash_input(&input);
        
        // Same input should produce same hash
        assert_eq!(hash1, hash2);
        
        // Different input should produce different hash
        let different_input = json!({"key": "different"});
        let hash3 = logger.hash_input(&different_input);
        assert_ne!(hash1, hash3);
    }
}
