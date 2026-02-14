//! Human Override Control
//!
//! Allows human operators to approve or deny AI actions

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

use super::Action;

/// Human override controller
#[derive(Debug)]
pub struct HumanOverride {
    pending_requests: HashMap<Uuid, OverrideRequest>,
    request_history: Vec<OverrideRequest>,
    auto_approve_below: Option<rust_decimal::Decimal>, // Auto-approve small trades
}

/// Override request status
#[derive(Debug, Clone, PartialEq)]
pub enum OverrideStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

/// Types of overrides that can be requested
#[derive(Debug, Clone, PartialEq)]
pub enum OverrideType {
    LimitExceeded,
    LargeTrade,
    UnusualActivity,
    HighRisk,
    ManualReview,
}

/// A request for human override
#[derive(Debug, Clone)]
pub struct OverrideRequest {
    pub id: Uuid,
    pub override_type: OverrideType,
    pub status: OverrideStatus,
    pub reason: String,
    pub action: Action,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub denial_reason: Option<String>,
}

impl HumanOverride {
    /// Create new override controller
    pub fn new() -> Self {
        Self {
            pending_requests: HashMap::new(),
            request_history: Vec::new(),
            auto_approve_below: None,
        }
    }

    /// Create with auto-approve threshold
    pub fn with_auto_approve(threshold: rust_decimal::Decimal) -> Self {
        Self {
            pending_requests: HashMap::new(),
            request_history: Vec::new(),
            auto_approve_below: Some(threshold),
        }
    }

    /// Request human override for an action
    pub fn request_override(
        &mut self,
        override_type: OverrideType,
        reason: String,
        action: Action,
    ) -> OverrideRequest {
        // Check auto-approve
        if let Some(threshold) = self.auto_approve_below {
            let notional = action.quantity * action.price.unwrap_or_default();
            if notional < threshold {
                let request = OverrideRequest {
                    id: Uuid::new_v4(),
                    override_type,
                    status: OverrideStatus::Approved,
                    reason: format!("{} (Auto-approved below threshold)", reason),
                    action,
                    created_at: Utc::now(),
                    expires_at: Utc::now() + chrono::Duration::minutes(5),
                    approved_by: Some("SYSTEM_AUTO".to_string()),
                    approved_at: Some(Utc::now()),
                    denial_reason: None,
                };
                info!("Auto-approved override request: {}", request.id);
                return request;
            }
        }

        let request = OverrideRequest {
            id: Uuid::new_v4(),
            override_type,
            status: OverrideStatus::Pending,
            reason,
            action,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::minutes(10),
            approved_by: None,
            approved_at: None,
            denial_reason: None,
        };

        info!(
            "Override requested: {} - {} - {:?}",
            request.id, request.reason, request.override_type
        );

        self.pending_requests.insert(request.id, request.clone());
        request
    }

    /// Approve an override request
    pub fn approve_override(&mut self, request_id: Uuid, approver: String) -> Result<(), String> {
        let request = self.pending_requests.get_mut(&request_id)
            .ok_or_else(|| "Request not found".to_string())?;

        if request.status != OverrideStatus::Pending {
            return Err(format!("Request is not pending, current status: {:?}", request.status));
        }

        if Utc::now() > request.expires_at {
            request.status = OverrideStatus::Expired;
            return Err("Request has expired".to_string());
        }

        request.status = OverrideStatus::Approved;
        request.approved_by = Some(approver.clone());
        request.approved_at = Some(Utc::now());

        info!(
            "Override approved: {} by {}",
            request_id, approver
        );

        // Move to history
        let request = self.pending_requests.remove(&request_id).unwrap();
        self.request_history.push(request);

        Ok(())
    }

    /// Deny an override request
    pub fn deny_override(&mut self, request_id: Uuid, reason: String) -> Result<(), String> {
        let request = self.pending_requests.get_mut(&request_id)
            .ok_or_else(|| "Request not found".to_string())?;

        if request.status != OverrideStatus::Pending {
            return Err(format!("Request is not pending, current status: {:?}", request.status));
        }

        request.status = OverrideStatus::Denied;
        request.denial_reason = Some(reason.clone());

        warn!(
            "Override denied: {} - Reason: {}",
            request_id, reason
        );

        // Move to history
        let request = self.pending_requests.remove(&request_id).unwrap();
        self.request_history.push(request);

        Ok(())
    }

    /// Get pending request
    pub fn get_request(&self, request_id: Uuid) -> Option<&OverrideRequest> {
        self.pending_requests.get(&request_id)
    }

    /// Get all pending requests
    pub fn pending_requests(&self) -> Vec<&OverrideRequest> {
        self.pending_requests.values().collect()
    }

    /// Count of pending requests
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Clean expired requests
    pub fn clean_expired(&mut self) {
        let now = Utc::now();
        let expired: Vec<Uuid> = self.pending_requests.iter()
            .filter(|(_, req)| now > req.expires_at)
            .map(|(id, _)| *id)
            .collect();

        for id in expired {
            if let Some(mut req) = self.pending_requests.remove(&id) {
                req.status = OverrideStatus::Expired;
                warn!("Override request expired: {}", id);
                self.request_history.push(req);
            }
        }
    }

    /// Get request history
    pub fn history(&self) -> &[OverrideRequest] {
        &self.request_history
    }

    /// Check if action is approved (either auto-approved or manually approved)
    pub fn is_approved(&self, request_id: Uuid) -> bool {
        // Check pending
        if let Some(req) = self.pending_requests.get(&request_id) {
            return req.status == OverrideStatus::Approved;
        }
        
        // Check history
        self.request_history.iter()
            .any(|req| req.id == request_id && req.status == OverrideStatus::Approved)
    }
}

impl Default for HumanOverride {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn create_test_action() -> Action {
        Action {
            action_type: super::super::ActionType::PlaceOrder,
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            price: Some(Decimal::from(150)),
            side: crate::broker::OrderSide::Buy,
            strategy: "test".to_string(),
            confidence: 0.8,
        }
    }

    #[test]
    fn test_request_override() {
        let mut ctrl = HumanOverride::new();
        let action = create_test_action();

        let request = ctrl.request_override(
            OverrideType::LargeTrade,
            "Trade exceeds threshold".to_string(),
            action,
        );

        assert_eq!(request.status, OverrideStatus::Pending);
        assert_eq!(ctrl.pending_count(), 1);
    }

    #[test]
    fn test_approve_override() {
        let mut ctrl = HumanOverride::new();
        let action = create_test_action();

        let request = ctrl.request_override(
            OverrideType::LimitExceeded,
            "Daily limit".to_string(),
            action,
        );

        ctrl.approve_override(request.id, "admin".to_string()).unwrap();

        assert_eq!(ctrl.pending_count(), 0);
        assert!(ctrl.is_approved(request.id));
    }

    #[test]
    fn test_deny_override() {
        let mut ctrl = HumanOverride::new();
        let action = create_test_action();

        let request = ctrl.request_override(
            OverrideType::HighRisk,
            "Too risky".to_string(),
            action,
        );

        ctrl.deny_override(request.id, "Risk limits".to_string()).unwrap();

        assert_eq!(ctrl.pending_count(), 0);
        assert!(!ctrl.is_approved(request.id));
    }

    #[test]
    fn test_auto_approve() {
        let mut ctrl = HumanOverride::with_auto_approve(Decimal::from(1000));
        
        let small_action = Action {
            action_type: super::super::ActionType::PlaceOrder,
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(1),
            price: Some(Decimal::from(100)), // $100 < $1000 threshold
            side: crate::broker::OrderSide::Buy,
            strategy: "test".to_string(),
            confidence: 0.8,
        };

        let request = ctrl.request_override(
            OverrideType::ManualReview,
            "Review".to_string(),
            small_action,
        );

        assert_eq!(request.status, OverrideStatus::Approved);
        assert_eq!(request.approved_by, Some("SYSTEM_AUTO".to_string()));
    }

    #[test]
    fn test_not_found() {
        let mut ctrl = HumanOverride::new();
        
        let result = ctrl.approve_override(Uuid::new_v4(), "admin".to_string());
        assert!(result.is_err());
    }
}
