//! Trade execution router — dispatches proposals based on trading mode.

use tracing::{info, warn};

use super::modes::{TradeProposal, TradingMode};

/// Routes trade proposals to the correct handler based on their trading mode.
pub struct TradeExecutor;

impl TradeExecutor {
    /// Signal mode: log the signal for user awareness, no execution.
    pub fn handle_signal(proposal: &TradeProposal) {
        info!(
            symbol = %proposal.symbol,
            direction = %proposal.direction,
            confidence = proposal.confidence * 100.0,
            reason = %proposal.reason,
            "SIGNAL: {} {} conf={:.0}%",
            proposal.symbol,
            proposal.direction,
            proposal.confidence * 100.0,
        );
    }

    /// Semi-auto mode: store proposal as pending, await user confirmation.
    pub fn handle_semi_auto(proposal: &TradeProposal) {
        info!(
            symbol = %proposal.symbol,
            direction = %proposal.direction,
            reason = %proposal.reason,
            expires_at = %proposal.expires_at,
            "PROPOSAL: {} {} — awaiting confirmation (expires {})",
            proposal.symbol,
            proposal.direction,
            proposal.expires_at,
        );
        // TODO: Store in DB, notify via WebSocket
    }

    /// Full-auto mode: execute immediately within risk limits.
    pub fn handle_full_auto(proposal: &TradeProposal) {
        if proposal.confidence < 0.5 {
            warn!(
                symbol = %proposal.symbol,
                confidence = proposal.confidence * 100.0,
                "AUTO-EXECUTE skipped: confidence {:.0}% below 50% threshold",
                proposal.confidence * 100.0,
            );
            return;
        }

        info!(
            symbol = %proposal.symbol,
            direction = %proposal.direction,
            size_pct = proposal.suggested_size_pct,
            reason = %proposal.reason,
            "AUTO-EXECUTE: {} {} size={:.1}%",
            proposal.symbol,
            proposal.direction,
            proposal.suggested_size_pct,
        );
        // TODO: Call broker gateway to place order
    }

    /// Route a proposal to the correct handler based on its mode.
    pub fn execute(proposal: &TradeProposal) {
        match proposal.mode {
            TradingMode::Signal => Self::handle_signal(proposal),
            TradingMode::SemiAuto => Self::handle_semi_auto(proposal),
            TradingMode::FullAuto => Self::handle_full_auto(proposal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::modes::{ProposalStatus, TradeProposal, TradingMode};

    fn make_proposal(mode: TradingMode, confidence: f64) -> TradeProposal {
        let now = chrono::Utc::now();
        TradeProposal {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            strategy_id: uuid::Uuid::new_v4(),
            symbol: "ETHUSDT".to_string(),
            direction: "long".to_string(),
            confidence,
            suggested_size_pct: 1.5,
            stop_loss_pct: 2.0,
            take_profit_pct: 4.0,
            reason: "Test proposal".to_string(),
            mode,
            status: ProposalStatus::Pending,
            created_at: now,
            expires_at: now + chrono::Duration::minutes(10),
        }
    }

    #[test]
    fn test_execute_routes_signal() {
        let proposal = make_proposal(TradingMode::Signal, 0.75);
        // Should not panic
        TradeExecutor::execute(&proposal);
    }

    #[test]
    fn test_execute_routes_semi_auto() {
        let proposal = make_proposal(TradingMode::SemiAuto, 0.80);
        TradeExecutor::execute(&proposal);
    }

    #[test]
    fn test_execute_routes_full_auto() {
        let proposal = make_proposal(TradingMode::FullAuto, 0.90);
        TradeExecutor::execute(&proposal);
    }

    #[test]
    fn test_full_auto_low_confidence_skipped() {
        let proposal = make_proposal(TradingMode::FullAuto, 0.30);
        // Should not panic — logs a warning and skips
        TradeExecutor::handle_full_auto(&proposal);
    }
}
