//! Session Manager

use super::MarketId;
use chrono::{DateTime, Utc};

/// Session manager
#[derive(Debug)]
pub struct SessionManager {
    last_session: Option<MarketSession>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            last_session: None,
        }
    }
    
    /// Check for session transitions
    pub fn check_transitions(&self, _now: DateTime<Utc>) -> Option<SessionTransition> {
        // Simplified - would check if primary session changed
        None
    }
    
    /// Get current session
    pub fn current_session(&self) -> Option<&MarketSession> {
        self.last_session.as_ref()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Market session
#[derive(Debug, Clone)]
pub struct MarketSession {
    pub market: MarketId,
    pub start: DateTime<Utc>,
    pub session_type: super::SessionType,
}

/// Session transition
#[derive(Debug, Clone)]
pub struct SessionTransition {
    pub from_market: String,
    pub to_market: String,
    pub timestamp: DateTime<Utc>,
}
