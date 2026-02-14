//! Alert System
//!
//! Manages and dispatches alerts

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Alert manager
#[derive(Debug)]
pub struct AlertManager {
    alerts: Vec<Alert>,
    config: AlertConfig,
    dispatchers: Vec<Box<dyn AlertDispatcher>>,
}

/// Alert
#[derive(Debug, Clone)]
pub struct Alert {
    pub id: Uuid,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    pub fn name(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "INFO",
            AlertSeverity::Warning => "WARNING",
            AlertSeverity::Critical => "CRITICAL",
        }
    }
    
    pub fn emoji(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "ℹ️",
            AlertSeverity::Warning => "⚠️",
            AlertSeverity::Critical => "🚨",
        }
    }
    
    pub fn color(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "blue",
            AlertSeverity::Warning => "yellow",
            AlertSeverity::Critical => "red",
        }
    }
}

/// Alert configuration
#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub enabled: bool,
    pub throttle_seconds: u64,
    pub max_alerts: usize,
    pub email_enabled: bool,
    pub sms_enabled: bool,
    pub webhook_enabled: bool,
    pub slack_enabled: bool,
    pub min_severity: AlertSeverity,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            throttle_seconds: 60,
            max_alerts: 1000,
            email_enabled: true,
            sms_enabled: false,
            webhook_enabled: true,
            slack_enabled: false,
            min_severity: AlertSeverity::Warning,
        }
    }
}

/// Alert dispatcher trait
pub trait AlertDispatcher: std::fmt::Debug {
    fn dispatch(&self, alert: &Alert) -> Result<(), String>;
    fn name(&self) -> &str;
}

/// Console dispatcher
#[derive(Debug)]
pub struct ConsoleDispatcher;

impl AlertDispatcher for ConsoleDispatcher {
    fn dispatch(&self, alert: &Alert) -> Result<(), String> {
        match alert.severity {
            AlertSeverity::Info => info!("{} {}", alert.severity.emoji(), alert.message),
            AlertSeverity::Warning => warn!("{} {}", alert.severity.emoji(), alert.message),
            AlertSeverity::Critical => error!("{} {}", alert.severity.emoji(), alert.message),
        }
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Console"
    }
}

/// Email dispatcher
#[derive(Debug)]
pub struct EmailDispatcher {
    recipients: Vec<String>,
}

impl EmailDispatcher {
    pub fn new(recipients: Vec<String>) -> Self {
        Self { recipients }
    }
}

impl AlertDispatcher for EmailDispatcher {
    fn dispatch(&self, alert: &Alert) -> Result<(), String> {
        // Simplified - would actually send email
        info!("Email alert to {:?}: [{}] {}", 
            self.recipients, alert.severity.name(), alert.title);
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Email"
    }
}

/// Webhook dispatcher
#[derive(Debug)]
pub struct WebhookDispatcher {
    url: String,
}

impl WebhookDispatcher {
    pub fn new(url: &str) -> Self {
        Self { url: url.to_string() }
    }
}

impl AlertDispatcher for WebhookDispatcher {
    fn dispatch(&self, alert: &Alert) -> Result<(), String> {
        // Simplified - would actually POST to webhook
        info!("Webhook alert to {}: [{}] {}", 
            self.url, alert.severity.name(), alert.title);
        Ok(())
    }
    
    fn name(&self) -> &str {
        "Webhook"
    }
}

impl AlertManager {
    /// Create new alert manager
    pub fn new() -> Self {
        let mut manager = Self {
            alerts: Vec::new(),
            config: AlertConfig::default(),
            dispatchers: Vec::new(),
        };
        
        // Add default console dispatcher
        manager.add_dispatcher(Box::new(ConsoleDispatcher));
        
        manager
    }
    
    /// Create and dispatch alert
    pub fn create_alert(&mut self, severity: AlertSeverity, title: &str, message: &str) -> Uuid {
        if !self.config.enabled {
            return Uuid::nil();
        }
        
        if severity < self.config.min_severity {
            return Uuid::nil();
        }
        
        // Check throttling
        if self.is_throttled(&severity, title) {
            return Uuid::nil();
        }
        
        let alert = Alert {
            id: Uuid::new_v4(),
            severity,
            title: title.to_string(),
            message: message.to_string(),
            timestamp: Utc::now(),
            acknowledged: false,
            acknowledged_by: None,
            acknowledged_at: None,
            metadata: HashMap::new(),
        };
        
        let id = alert.id;
        
        // Dispatch to all channels
        for dispatcher in &self.dispatchers {
            if let Err(e) = dispatcher.dispatch(&alert) {
                error!("Failed to dispatch alert via {}: {}", dispatcher.name(), e);
            }
        }
        
        // Store alert
        self.alerts.push(alert);
        
        // Trim old alerts
        if self.alerts.len() > self.config.max_alerts {
            self.alerts.remove(0);
        }
        
        id
    }
    
    /// Acknowledge alert
    pub fn acknowledge(&mut self, alert_id: Uuid) {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            alert.acknowledged_at = Some(Utc::now());
            info!("Alert {} acknowledged", alert_id);
        }
    }
    
    /// Acknowledge by user
    pub fn acknowledge_by(&mut self, alert_id: Uuid, user: &str) {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            alert.acknowledged_by = Some(user.to_string());
            alert.acknowledged_at = Some(Utc::now());
            info!("Alert {} acknowledged by {}", alert_id, user);
        }
    }
    
    /// Get active (unacknowledged) alerts
    pub fn get_active(&self) -> Vec<&Alert> {
        self.alerts.iter()
            .filter(|a| !a.acknowledged)
            .collect()
    }
    
    /// Get all alerts
    pub fn get_all(&self) -> &[Alert] {
        &self.alerts
    }
    
    /// Get alerts by severity
    pub fn get_by_severity(&self, severity: AlertSeverity) -> Vec<&Alert> {
        self.alerts.iter()
            .filter(|a| a.severity == severity)
            .collect()
    }
    
    /// Get alert by ID
    pub fn get_by_id(&self, alert_id: Uuid) -> Option<&Alert> {
        self.alerts.iter().find(|a| a.id == alert_id)
    }
    
    /// Add alert dispatcher
    pub fn add_dispatcher(&mut self, dispatcher: Box<dyn AlertDispatcher>) {
        self.dispatchers.push(dispatcher);
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: AlertConfig) {
        self.config = config;
    }
    
    /// Get configuration
    pub fn config(&self) -> &AlertConfig {
        &self.config
    }
    
    /// Get active alert count
    pub fn active_count(&self) -> usize {
        self.alerts.iter().filter(|a| !a.acknowledged).count()
    }
    
    /// Get total alert count
    pub fn total_count(&self) -> usize {
        self.alerts.len()
    }
    
    /// Clear acknowledged alerts
    pub fn clear_acknowledged(&mut self) {
        self.alerts.retain(|a| !a.acknowledged);
    }
    
    /// Clean old alerts
    pub fn cleanup(&mut self, cutoff: DateTime<Utc>) {
        self.alerts.retain(|a| a.timestamp > cutoff);
    }
    
    /// Check if alert should be throttled
    fn is_throttled(&self, severity: &AlertSeverity, title: &str) -> bool {
        let key = format!("{:?}:{}", severity, title);
        
        if let Some(last_alert) = self.alerts.iter()
            .filter(|a| a.title == title && a.severity == *severity)
            .last()
        {
            let elapsed = (Utc::now() - last_alert.timestamp).num_seconds() as u64;
            return elapsed < self.config.throttle_seconds;
        }
        
        false
    }
    
    /// Get alert statistics
    pub fn get_stats(&self) -> AlertStats {
        let mut stats = AlertStats::default();
        
        for alert in &self.alerts {
            match alert.severity {
                AlertSeverity::Info => stats.info_count += 1,
                AlertSeverity::Warning => stats.warning_count += 1,
                AlertSeverity::Critical => stats.critical_count += 1,
            }
            
            if alert.acknowledged {
                stats.acknowledged_count += 1;
            }
        }
        
        stats.total_count = self.alerts.len();
        stats.active_count = self.active_count();
        
        stats
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Alert statistics
#[derive(Debug, Default, Clone)]
pub struct AlertStats {
    pub total_count: usize,
    pub active_count: usize,
    pub acknowledged_count: usize,
    pub info_count: usize,
    pub warning_count: usize,
    pub critical_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_severity() {
        assert_eq!(AlertSeverity::Critical.name(), "CRITICAL");
        assert_eq!(AlertSeverity::Warning.emoji(), "⚠️");
        assert_eq!(AlertSeverity::Info.color(), "blue");
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AlertSeverity::Critical > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }

    #[test]
    fn test_manager_creation() {
        let manager = AlertManager::new();
        assert_eq!(manager.active_count(), 0);
        assert!(manager.config().enabled);
    }

    #[test]
    fn test_create_alert() {
        let mut manager = AlertManager::new();
        
        let id = manager.create_alert(AlertSeverity::Warning, "Test", "Test message");
        
        assert_ne!(id, Uuid::nil());
        assert_eq!(manager.total_count(), 1);
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn test_acknowledge_alert() {
        let mut manager = AlertManager::new();
        
        let id = manager.create_alert(AlertSeverity::Warning, "Test", "Test message");
        
        manager.acknowledge(id);
        
        assert_eq!(manager.active_count(), 0);
        
        let alert = manager.get_by_id(id).unwrap();
        assert!(alert.acknowledged);
    }

    #[test]
    fn test_acknowledge_by_user() {
        let mut manager = AlertManager::new();
        
        let id = manager.create_alert(AlertSeverity::Warning, "Test", "Test message");
        
        manager.acknowledge_by(id, "admin");
        
        let alert = manager.get_by_id(id).unwrap();
        assert!(alert.acknowledged);
        assert_eq!(alert.acknowledged_by, Some("admin".to_string()));
    }

    #[test]
    fn test_get_by_severity() {
        let mut manager = AlertManager::new();
        
        manager.create_alert(AlertSeverity::Info, "Info", "Info message");
        manager.create_alert(AlertSeverity::Warning, "Warn", "Warning message");
        manager.create_alert(AlertSeverity::Critical, "Crit", "Critical message");
        
        let warnings = manager.get_by_severity(AlertSeverity::Warning);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_clear_acknowledged() {
        let mut manager = AlertManager::new();
        
        let id1 = manager.create_alert(AlertSeverity::Warning, "Test1", "Message 1");
        let id2 = manager.create_alert(AlertSeverity::Warning, "Test2", "Message 2");
        
        manager.acknowledge(id1);
        manager.clear_acknowledged();
        
        assert_eq!(manager.total_count(), 1);
        assert!(manager.get_by_id(id2).is_some());
    }

    #[test]
    fn test_config_min_severity() {
        let mut manager = AlertManager::new();
        
        let mut config = AlertConfig::default();
        config.min_severity = AlertSeverity::Warning;
        manager.update_config(config);
        
        // Info alert should be filtered
        let id = manager.create_alert(AlertSeverity::Info, "Info", "Info message");
        assert_eq!(id, Uuid::nil());
        
        // Warning alert should pass
        let id2 = manager.create_alert(AlertSeverity::Warning, "Warn", "Warning message");
        assert_ne!(id2, Uuid::nil());
    }

    #[test]
    fn test_throttling() {
        let mut manager = AlertManager::new();
        
        let mut config = AlertConfig::default();
        config.throttle_seconds = 3600; // 1 hour
        manager.update_config(config);
        
        let id1 = manager.create_alert(AlertSeverity::Warning, "Same", "Same message");
        let id2 = manager.create_alert(AlertSeverity::Warning, "Same", "Same message");
        
        // Second alert should be throttled
        assert_ne!(id1, Uuid::nil());
        assert_eq!(id2, Uuid::nil());
    }

    #[test]
    fn test_stats() {
        let mut manager = AlertManager::new();
        
        // Lower min severity to include info alerts
        let mut config = AlertConfig::default();
        config.min_severity = AlertSeverity::Info;
        manager.update_config(config);
        
        manager.create_alert(AlertSeverity::Info, "I1", "Info");
        manager.create_alert(AlertSeverity::Info, "I2", "Info");
        manager.create_alert(AlertSeverity::Warning, "W1", "Warning");
        manager.create_alert(AlertSeverity::Critical, "C1", "Critical");
        
        let stats = manager.get_stats();
        
        assert_eq!(stats.total_count, 4);
        assert_eq!(stats.info_count, 2);
        assert_eq!(stats.warning_count, 1);
        assert_eq!(stats.critical_count, 1);
    }

    #[test]
    fn test_console_dispatcher() {
        let dispatcher = ConsoleDispatcher;
        let alert = Alert {
            id: Uuid::new_v4(),
            severity: AlertSeverity::Info,
            title: "Test".to_string(),
            message: "Test message".to_string(),
            timestamp: Utc::now(),
            acknowledged: false,
            acknowledged_by: None,
            acknowledged_at: None,
            metadata: HashMap::new(),
        };
        
        assert!(dispatcher.dispatch(&alert).is_ok());
    }
}
