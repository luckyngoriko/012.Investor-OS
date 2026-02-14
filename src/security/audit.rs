//! Audit Trail Module
//!
//! Security audit logging for all security events

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Audit logger
#[derive(Debug)]
pub struct AuditLogger {
    events: Vec<AuditEvent>,
    max_events: usize,
}

/// Audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEvent,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub details: HashMap<String, String>,
}

/// Security event types
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    // Authentication events
    LoginSuccess { user_id: Uuid },
    LoginFailed { user_id: Uuid, reason: String },
    LoginDenied { user_id: Uuid, reason: String },
    Logout { user_id: Uuid },
    
    // 2FA events
    TwoFactorSetup { user_id: Uuid },
    TwoFactorEnabled { user_id: Uuid },
    TwoFactorDisabled { user_id: Uuid },
    TwoFactorVerified { user_id: Uuid },
    TwoFactorFailed { user_id: Uuid },
    
    // API key events
    ApiKeyCreated { user_id: Uuid, key_id: Uuid, clearance: super::ClearanceLevel },
    ApiKeyRevoked { user_id: Uuid, key_id: Uuid },
    ApiKeyValidated { key_id: Uuid },
    ApiKeyExpired { key_id: Uuid },
    
    // Policy events
    PolicyViolation { user_id: Uuid, policy: String, action: String },
    AccessDenied { user_id: Uuid, resource: String, reason: String },
    AccessGranted { user_id: Uuid, resource: String },
    
    // Encryption events
    KeyRotation { timestamp: DateTime<Utc> },
    EncryptionFailed { reason: String },
    
    // System events
    ConfigChanged { user_id: Uuid, setting: String },
    SuspiciousActivity { user_id: Uuid, description: String },
}

impl SecurityEvent {
    pub fn name(&self) -> &'static str {
        match self {
            SecurityEvent::LoginSuccess { .. } => "LOGIN_SUCCESS",
            SecurityEvent::LoginFailed { .. } => "LOGIN_FAILED",
            SecurityEvent::LoginDenied { .. } => "LOGIN_DENIED",
            SecurityEvent::Logout { .. } => "LOGOUT",
            SecurityEvent::TwoFactorSetup { .. } => "2FA_SETUP",
            SecurityEvent::TwoFactorEnabled { .. } => "2FA_ENABLED",
            SecurityEvent::TwoFactorDisabled { .. } => "2FA_DISABLED",
            SecurityEvent::TwoFactorVerified { .. } => "2FA_VERIFIED",
            SecurityEvent::TwoFactorFailed { .. } => "2FA_FAILED",
            SecurityEvent::ApiKeyCreated { .. } => "API_KEY_CREATED",
            SecurityEvent::ApiKeyRevoked { .. } => "API_KEY_REVOKED",
            SecurityEvent::ApiKeyValidated { .. } => "API_KEY_VALIDATED",
            SecurityEvent::ApiKeyExpired { .. } => "API_KEY_EXPIRED",
            SecurityEvent::PolicyViolation { .. } => "POLICY_VIOLATION",
            SecurityEvent::AccessDenied { .. } => "ACCESS_DENIED",
            SecurityEvent::AccessGranted { .. } => "ACCESS_GRANTED",
            SecurityEvent::KeyRotation { .. } => "KEY_ROTATION",
            SecurityEvent::EncryptionFailed { .. } => "ENCRYPTION_FAILED",
            SecurityEvent::ConfigChanged { .. } => "CONFIG_CHANGED",
            SecurityEvent::SuspiciousActivity { .. } => "SUSPICIOUS_ACTIVITY",
        }
    }
    
    pub fn severity(&self) -> AuditSeverity {
        match self {
            SecurityEvent::LoginSuccess { .. } => AuditSeverity::Info,
            SecurityEvent::Logout { .. } => AuditSeverity::Info,
            SecurityEvent::TwoFactorVerified { .. } => AuditSeverity::Info,
            SecurityEvent::ApiKeyValidated { .. } => AuditSeverity::Info,
            SecurityEvent::AccessGranted { .. } => AuditSeverity::Info,
            
            SecurityEvent::LoginFailed { .. } => AuditSeverity::Warning,
            SecurityEvent::TwoFactorFailed { .. } => AuditSeverity::Warning,
            SecurityEvent::ApiKeyExpired { .. } => AuditSeverity::Warning,
            SecurityEvent::PolicyViolation { .. } => AuditSeverity::Warning,
            
            SecurityEvent::LoginDenied { .. } => AuditSeverity::Error,
            SecurityEvent::AccessDenied { .. } => AuditSeverity::Error,
            SecurityEvent::EncryptionFailed { .. } => AuditSeverity::Error,
            
            SecurityEvent::ApiKeyRevoked { .. } => AuditSeverity::Notice,
            SecurityEvent::TwoFactorDisabled { .. } => AuditSeverity::Notice,
            
            SecurityEvent::SuspiciousActivity { .. } => AuditSeverity::Critical,
            
            _ => AuditSeverity::Info,
        }
    }
}

/// Audit severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditSeverity {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// Audit trail
#[derive(Debug, Clone)]
pub struct AuditTrail {
    pub user_id: Uuid,
    pub events: Vec<AuditEvent>,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            max_events: 10000,
        }
    }
    
    /// Log security event
    pub fn log(&mut self, event: SecurityEvent) {
        let audit_event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: event.clone(),
            user_id: Self::extract_user_id(&event),
            ip_address: None,
            user_agent: None,
            success: Self::is_success(&event),
            details: Self::extract_details(&event),
        };
        
        // Log to tracing
        match event.severity() {
            AuditSeverity::Debug => tracing::debug!("Audit: {:?}", event),
            AuditSeverity::Info => tracing::info!("Audit: {:?}", event),
            AuditSeverity::Notice => tracing::info!("Audit: {:?}", event),
            AuditSeverity::Warning => tracing::warn!("Audit: {:?}", event),
            AuditSeverity::Error => tracing::error!("Audit: {:?}", event),
            AuditSeverity::Critical => tracing::error!("Audit: {:?}", event),
            AuditSeverity::Alert => tracing::error!("Audit: {:?}", event),
            AuditSeverity::Emergency => tracing::error!("Audit: {:?}", event),
        }
        
        self.events.push(audit_event);
        
        // Trim old events
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }
    
    /// Get events for user
    pub fn get_events_for_user(&self, user_id: Uuid, days: i64) -> Vec<&AuditEvent> {
        let cutoff = Utc::now() - Duration::days(days);
        
        self.events.iter()
            .filter(|e| {
                e.user_id == Some(user_id) && e.timestamp > cutoff
            })
            .collect()
    }
    
    /// Get events by type
    pub fn get_events_by_type(&self, event_name: &str) -> Vec<&AuditEvent> {
        self.events.iter()
            .filter(|e| e.event_type.name() == event_name)
            .collect()
    }
    
    /// Get failed login attempts for user
    pub fn get_failed_logins(&self, user_id: Uuid, hours: i64) -> Vec<&AuditEvent> {
        let cutoff = Utc::now() - Duration::hours(hours);
        
        self.events.iter()
            .filter(|e| {
                matches!(e.event_type, SecurityEvent::LoginFailed { user_id: id, .. } if id == user_id)
                    && e.timestamp > cutoff
            })
            .collect()
    }
    
    /// Generate audit trail for user
    pub fn generate_trail(&self, user_id: Uuid, days: i64) -> AuditTrail {
        let events = self.get_events_for_user(user_id, days);
        
        let start_date = events.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let end_date = events.last().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        
        AuditTrail {
            user_id,
            events: events.iter().map(|&e| e.clone()).collect(),
            start_date,
            end_date,
        }
    }
    
    /// Get suspicious activity
    pub fn get_suspicious_activity(&self, hours: i64) -> Vec<&AuditEvent> {
        let cutoff = Utc::now() - Duration::hours(hours);
        
        self.events.iter()
            .filter(|e| {
                matches!(e.event_type, SecurityEvent::SuspiciousActivity { .. })
                    || (matches!(e.event_type, SecurityEvent::LoginFailed { .. }) && !e.success)
            })
            .filter(|e| e.timestamp > cutoff)
            .collect()
    }
    
    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> AuditStats {
        let mut stats = AuditStats::default();
        
        for event in &self.events {
            stats.total_events += 1;
            
            match event.event_type.severity() {
                AuditSeverity::Info => stats.info_count += 1,
                AuditSeverity::Warning => stats.warning_count += 1,
                AuditSeverity::Error => stats.error_count += 1,
                AuditSeverity::Critical => stats.critical_count += 1,
                _ => {}
            }
            
            if !event.success {
                stats.failed_events += 1;
            }
        }
        
        stats
    }
    
    /// Clean old events
    pub fn cleanup(&mut self, max_age: Duration) {
        let cutoff = Utc::now() - max_age;
        self.events.retain(|e| e.timestamp > cutoff);
    }
    
    /// Set max events
    pub fn set_max_events(&mut self, max: usize) {
        self.max_events = max.max(100);
    }
    
    /// Extract user ID from event
    fn extract_user_id(event: &SecurityEvent) -> Option<Uuid> {
        match event {
            SecurityEvent::LoginSuccess { user_id } => Some(*user_id),
            SecurityEvent::LoginFailed { user_id, .. } => Some(*user_id),
            SecurityEvent::LoginDenied { user_id, .. } => Some(*user_id),
            SecurityEvent::Logout { user_id } => Some(*user_id),
            SecurityEvent::TwoFactorSetup { user_id } => Some(*user_id),
            SecurityEvent::TwoFactorEnabled { user_id } => Some(*user_id),
            SecurityEvent::TwoFactorDisabled { user_id } => Some(*user_id),
            SecurityEvent::TwoFactorVerified { user_id } => Some(*user_id),
            SecurityEvent::TwoFactorFailed { user_id } => Some(*user_id),
            SecurityEvent::ApiKeyCreated { user_id, .. } => Some(*user_id),
            SecurityEvent::ApiKeyRevoked { user_id, .. } => Some(*user_id),
            SecurityEvent::PolicyViolation { user_id, .. } => Some(*user_id),
            SecurityEvent::AccessDenied { user_id, .. } => Some(*user_id),
            SecurityEvent::AccessGranted { user_id, .. } => Some(*user_id),
            SecurityEvent::ConfigChanged { user_id, .. } => Some(*user_id),
            SecurityEvent::SuspiciousActivity { user_id, .. } => Some(*user_id),
            _ => None,
        }
    }
    
    /// Check if event represents success
    fn is_success(event: &SecurityEvent) -> bool {
        matches!(event,
            SecurityEvent::LoginSuccess { .. }
                | SecurityEvent::TwoFactorVerified { .. }
                | SecurityEvent::ApiKeyValidated { .. }
                | SecurityEvent::AccessGranted { .. }
        )
    }
    
    /// Extract details from event
    fn extract_details(event: &SecurityEvent) -> HashMap<String, String> {
        let mut details = HashMap::new();
        
        match event {
            SecurityEvent::LoginFailed { reason, .. } => {
                details.insert("reason".to_string(), reason.clone());
            }
            SecurityEvent::PolicyViolation { policy, action, .. } => {
                details.insert("policy".to_string(), policy.clone());
                details.insert("action".to_string(), action.clone());
            }
            SecurityEvent::SuspiciousActivity { description, .. } => {
                details.insert("description".to_string(), description.clone());
            }
            _ => {}
        }
        
        details
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit statistics
#[derive(Debug, Default, Clone)]
pub struct AuditStats {
    pub total_events: usize,
    pub info_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub critical_count: usize,
    pub failed_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_creation() {
        let logger = AuditLogger::new();
        assert_eq!(logger.event_count(), 0);
    }

    #[test]
    fn test_log_event() {
        let mut logger = AuditLogger::new();
        
        logger.log(SecurityEvent::LoginSuccess {
            user_id: Uuid::new_v4(),
        });
        
        assert_eq!(logger.event_count(), 1);
    }

    #[test]
    fn test_get_events_for_user() {
        let mut logger = AuditLogger::new();
        let user_id = Uuid::new_v4();
        
        logger.log(SecurityEvent::LoginSuccess { user_id });
        logger.log(SecurityEvent::LoginSuccess {
            user_id: Uuid::new_v4(),
        });
        
        let events = logger.get_events_for_user(user_id, 1);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_get_failed_logins() {
        let mut logger = AuditLogger::new();
        let user_id = Uuid::new_v4();
        
        logger.log(SecurityEvent::LoginFailed {
            user_id,
            reason: "Invalid password".to_string(),
        });
        logger.log(SecurityEvent::LoginSuccess { user_id });
        
        let failed = logger.get_failed_logins(user_id, 1);
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn test_event_severity() {
        let success = SecurityEvent::LoginSuccess { user_id: Uuid::new_v4() };
        assert_eq!(success.severity(), AuditSeverity::Info);
        
        let failed = SecurityEvent::LoginFailed {
            user_id: Uuid::new_v4(),
            reason: "Invalid".to_string(),
        };
        assert_eq!(failed.severity(), AuditSeverity::Warning);
        
        let suspicious = SecurityEvent::SuspiciousActivity {
            user_id: Uuid::new_v4(),
            description: "Test".to_string(),
        };
        assert_eq!(suspicious.severity(), AuditSeverity::Critical);
    }

    #[test]
    fn test_audit_trail() {
        let mut logger = AuditLogger::new();
        let user_id = Uuid::new_v4();
        
        logger.log(SecurityEvent::LoginSuccess { user_id });
        logger.log(SecurityEvent::Logout { user_id });
        
        let trail = logger.generate_trail(user_id, 1);
        assert_eq!(trail.user_id, user_id);
        assert_eq!(trail.events.len(), 2);
    }

    #[test]
    fn test_cleanup() {
        let mut logger = AuditLogger::new();
        
        // Add old event manually
        let old_event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now() - Duration::days(100),
            event_type: SecurityEvent::LoginSuccess { user_id: Uuid::new_v4() },
            user_id: None,
            ip_address: None,
            user_agent: None,
            success: true,
            details: HashMap::new(),
        };
        logger.events.push(old_event);
        
        // Add recent event
        logger.log(SecurityEvent::LoginSuccess { user_id: Uuid::new_v4() });
        
        assert_eq!(logger.event_count(), 2);
        
        // Cleanup events older than 30 days
        logger.cleanup(Duration::days(30));
        
        assert_eq!(logger.event_count(), 1);
    }

    #[test]
    fn test_stats() {
        let mut logger = AuditLogger::new();
        
        logger.log(SecurityEvent::LoginSuccess { user_id: Uuid::new_v4() });
        logger.log(SecurityEvent::LoginFailed {
            user_id: Uuid::new_v4(),
            reason: "Invalid".to_string(),
        });
        logger.log(SecurityEvent::SuspiciousActivity {
            user_id: Uuid::new_v4(),
            description: "Test".to_string(),
        });
        
        let stats = logger.get_stats();
        
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.info_count, 1);
        assert_eq!(stats.warning_count, 1);
        assert_eq!(stats.critical_count, 1);
        assert_eq!(stats.failed_events, 2);
    }
}
