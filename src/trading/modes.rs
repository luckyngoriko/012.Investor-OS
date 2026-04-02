//! Trading mode types and trade proposal model.

use serde::{Deserialize, Serialize};

/// Trading execution mode — determines how trade proposals are handled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradingMode {
    /// Show signals only, user trades manually.
    Signal,
    /// AI proposes, user confirms or rejects.
    SemiAuto,
    /// AI executes automatically within risk limits.
    FullAuto,
}

impl TradingMode {
    /// Parse a string into a `TradingMode`, defaulting to `Signal`.
    pub fn from_str_mode(s: &str) -> Self {
        match s {
            "semi_auto" => Self::SemiAuto,
            "full_auto" => Self::FullAuto,
            _ => Self::Signal,
        }
    }

    /// Returns `true` if this mode requires explicit user confirmation.
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Self::SemiAuto)
    }

    /// Returns `true` if this mode executes trades automatically.
    pub fn auto_execute(&self) -> bool {
        matches!(self, Self::FullAuto)
    }
}

/// Status of a trade proposal through its lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    /// Awaiting user decision (semi-auto mode).
    Pending,
    /// User confirmed the proposal.
    Confirmed,
    /// User rejected the proposal.
    Rejected,
    /// Trade was executed (full-auto or after confirmation).
    Executed,
    /// Proposal expired without action.
    Expired,
}

/// A trade proposal generated from the consensus pipeline.
#[derive(Debug, Clone, Serialize)]
pub struct TradeProposal {
    /// Unique proposal identifier.
    pub id: uuid::Uuid,
    /// User who owns the strategy.
    pub user_id: uuid::Uuid,
    /// Strategy that generated this proposal.
    pub strategy_id: uuid::Uuid,
    /// Trading symbol (e.g., "BTCUSDT").
    pub symbol: String,
    /// Direction: "long" or "short".
    pub direction: String,
    /// Model confidence (0.0 to 1.0).
    pub confidence: f64,
    /// Suggested position size as percentage of portfolio.
    pub suggested_size_pct: f64,
    /// Stop-loss distance as percentage.
    pub stop_loss_pct: f64,
    /// Take-profit distance as percentage.
    pub take_profit_pct: f64,
    /// Human-readable reason for the trade.
    pub reason: String,
    /// Execution mode for this proposal.
    pub mode: TradingMode,
    /// Current status.
    pub status: ProposalStatus,
    /// When the proposal was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the proposal expires.
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_mode_no_confirmation() {
        let mode = TradingMode::Signal;
        assert!(!mode.requires_confirmation());
        assert!(!mode.auto_execute());
    }

    #[test]
    fn test_semi_auto_requires_confirmation() {
        let mode = TradingMode::SemiAuto;
        assert!(mode.requires_confirmation());
        assert!(!mode.auto_execute());
    }

    #[test]
    fn test_full_auto_executes() {
        let mode = TradingMode::FullAuto;
        assert!(!mode.requires_confirmation());
        assert!(mode.auto_execute());
    }

    #[test]
    fn test_mode_from_str() {
        assert_eq!(TradingMode::from_str_mode("signal"), TradingMode::Signal);
        assert_eq!(
            TradingMode::from_str_mode("semi_auto"),
            TradingMode::SemiAuto
        );
        assert_eq!(
            TradingMode::from_str_mode("full_auto"),
            TradingMode::FullAuto
        );
        // Unknown strings default to Signal
        assert_eq!(TradingMode::from_str_mode("unknown"), TradingMode::Signal);
        assert_eq!(TradingMode::from_str_mode(""), TradingMode::Signal);
    }

    #[test]
    fn test_proposal_status_variants() {
        let statuses = vec![
            ProposalStatus::Pending,
            ProposalStatus::Confirmed,
            ProposalStatus::Rejected,
            ProposalStatus::Executed,
            ProposalStatus::Expired,
        ];
        // Ensure all variants are distinct
        for (i, a) in statuses.iter().enumerate() {
            for (j, b) in statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_trade_proposal_creation() {
        let now = chrono::Utc::now();
        let proposal = TradeProposal {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            strategy_id: uuid::Uuid::new_v4(),
            symbol: "BTCUSDT".to_string(),
            direction: "long".to_string(),
            confidence: 0.85,
            suggested_size_pct: 2.5,
            stop_loss_pct: 3.0,
            take_profit_pct: 6.0,
            reason: "Strong bullish consensus".to_string(),
            mode: TradingMode::SemiAuto,
            status: ProposalStatus::Pending,
            created_at: now,
            expires_at: now + chrono::Duration::minutes(15),
        };
        assert_eq!(proposal.symbol, "BTCUSDT");
        assert_eq!(proposal.direction, "long");
        assert!((proposal.confidence - 0.85).abs() < f64::EPSILON);
        assert!(proposal.mode.requires_confirmation());
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert!(proposal.expires_at > proposal.created_at);
    }
}
