//! Trade Executor NATS worker.
//!
//! Subscribes to `ios.trade.proposal.*`. For each proposal:
//! - Looks up all active user strategies matching the symbol
//! - Routes based on strategy mode:
//!   - Signal   -> publish to `ios.user.signal.{user_id}`
//!   - SemiAuto -> publish to `ios.user.proposal.{user_id}` (wait for confirm)
//!   - FullAuto -> check risk limits -> if OK -> publish to `ios.trade.execute.{user_id}`
//! - After execution -> publish to `ios.trade.fill.{user_id}`

use std::sync::Arc;

use futures::StreamExt;
use sqlx::{PgPool, Row};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::messages::{
    NatsEnvelope, TradeExecuteMsg, TradeFillMsg, TradeProposalMsg, UserProposalMsg, UserSignalMsg,
};
use super::publisher::NatsPublisher;
use super::NatsClient;
use crate::trading::modes::TradingMode;

/// Run the trade executor worker.
pub async fn run(nats: Arc<NatsClient>, publisher: Arc<NatsPublisher>, pool: PgPool) {
    info!("Trade executor started -- subscribing to ios.trade.proposal.*");

    let mut sub = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.trade.proposal.*"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!("Trade executor subscribe failed: {e}");
            return;
        }
    };

    while let Some(msg) = sub.next().await {
        let env: NatsEnvelope<TradeProposalMsg> = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let symbol = &env.symbol;
        let proposal = &env.data;

        // Find all active strategies that include this symbol
        let strategies = match fetch_strategies_for_symbol(&pool, symbol).await {
            Ok(s) => s,
            Err(e) => {
                warn!("{symbol}: failed to fetch strategies: {e}");
                continue;
            }
        };

        if strategies.is_empty() {
            debug!("{symbol}: no active strategies — skipping proposal");
            continue;
        }

        for strat in &strategies {
            let mode = TradingMode::from_str_mode(&strat.mode);
            let user_id_str = strat.user_id.to_string();

            match mode {
                TradingMode::Signal => {
                    let signal = UserSignalMsg {
                        strategy_id: strat.id.to_string(),
                        strategy_name: strat.name.clone(),
                        direction: proposal.direction.clone(),
                        confidence: proposal.confidence,
                        predicted_return: proposal.suggested_size_pct,
                        models_used: strat.models.clone(),
                        suggested_action: format!(
                            "{} {} (confidence {:.0}%)",
                            proposal.direction,
                            symbol,
                            proposal.confidence * 100.0
                        ),
                    };

                    if let Err(e) = publisher
                        .publish_user_signal(symbol, &user_id_str, signal)
                        .await
                    {
                        warn!("User signal publish failed for {user_id_str}: {e}");
                    }

                    info!(
                        "SIGNAL -> user {}: {} {} conf={:.0}%",
                        &user_id_str[..8],
                        proposal.direction,
                        symbol,
                        proposal.confidence * 100.0,
                    );
                }

                TradingMode::SemiAuto => {
                    let user_proposal = UserProposalMsg {
                        proposal_id: Uuid::new_v4().to_string(),
                        strategy_id: strat.id.to_string(),
                        direction: proposal.direction.clone(),
                        confidence: proposal.confidence,
                        suggested_size_pct: proposal.suggested_size_pct,
                        stop_loss_pct: proposal.stop_loss_pct,
                        take_profit_pct: proposal.take_profit_pct,
                        reason: proposal.reason.clone(),
                        expires_in_secs: 900, // 15 minutes
                    };

                    if let Err(e) = publisher
                        .publish_user_proposal(symbol, &user_id_str, user_proposal)
                        .await
                    {
                        warn!("User proposal publish failed for {user_id_str}: {e}");
                    }

                    info!(
                        "PROPOSAL -> user {}: {} {} — awaiting confirmation",
                        &user_id_str[..8],
                        proposal.direction,
                        symbol,
                    );
                }

                TradingMode::FullAuto => {
                    // Check risk limits before execution
                    if !check_risk_limits(&strat.risk_limits, proposal) {
                        warn!(
                            "FULL_AUTO blocked for user {}: risk limits exceeded",
                            &user_id_str[..8],
                        );
                        continue;
                    }

                    // Confidence floor: 50%
                    if proposal.confidence < 0.5 {
                        debug!(
                            "FULL_AUTO skipped for user {}: confidence {:.0}% < 50%",
                            &user_id_str[..8],
                            proposal.confidence * 100.0,
                        );
                        continue;
                    }

                    let execute_msg = TradeExecuteMsg {
                        strategy_id: strat.id.to_string(),
                        direction: proposal.direction.clone(),
                        size_pct: proposal.suggested_size_pct,
                        stop_loss_pct: proposal.stop_loss_pct,
                        take_profit_pct: proposal.take_profit_pct,
                        confidence: proposal.confidence,
                        reason: proposal.reason.clone(),
                    };

                    if let Err(e) = publisher
                        .publish_trade_execute(symbol, &user_id_str, execute_msg)
                        .await
                    {
                        warn!("Trade execute publish failed for {user_id_str}: {e}");
                        continue;
                    }

                    // Publish fill (simulated for now — real broker integration happens downstream)
                    let fill = TradeFillMsg {
                        trade_id: Uuid::new_v4().to_string(),
                        strategy_id: strat.id.to_string(),
                        direction: proposal.direction.clone(),
                        size_pct: proposal.suggested_size_pct,
                        entry_price: 0.0, // real price from broker
                        fill_status: "simulated".to_string(),
                        reason: proposal.reason.clone(),
                    };

                    if let Err(e) = publisher
                        .publish_trade_fill(symbol, &user_id_str, fill)
                        .await
                    {
                        warn!("Trade fill publish failed for {user_id_str}: {e}");
                    }

                    info!(
                        "AUTO-EXECUTE -> user {}: {} {} size={:.1}%",
                        &user_id_str[..8],
                        proposal.direction,
                        symbol,
                        proposal.suggested_size_pct,
                    );
                }
            }
        }
    }

    warn!("Trade executor subscription ended");
}

/// Minimal strategy info needed for routing decisions.
struct StrategyInfo {
    id: Uuid,
    user_id: Uuid,
    name: String,
    models: Vec<String>,
    mode: String,
    risk_limits: serde_json::Value,
}

/// Fetch all active strategies whose `symbols` array contains the given symbol.
async fn fetch_strategies_for_symbol(
    pool: &PgPool,
    symbol: &str,
) -> Result<Vec<StrategyInfo>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, name, models, mode, risk_limits
        FROM user_strategies
        WHERE is_active = true
          AND $1 = ANY(symbols)
        "#,
    )
    .bind(symbol)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let strategies = rows
        .iter()
        .map(|r| StrategyInfo {
            id: r.try_get::<Uuid, _>("id").unwrap_or_default(),
            user_id: r.try_get::<Uuid, _>("user_id").unwrap_or_default(),
            name: r.try_get::<String, _>("name").unwrap_or_default(),
            models: r.try_get::<Vec<String>, _>("models").unwrap_or_default(),
            mode: r.try_get::<String, _>("mode").unwrap_or_default(),
            risk_limits: r
                .try_get::<serde_json::Value, _>("risk_limits")
                .unwrap_or_default(),
        })
        .collect();

    Ok(strategies)
}

/// Check whether the proposal respects the strategy's risk limits.
fn check_risk_limits(risk_limits: &serde_json::Value, proposal: &TradeProposalMsg) -> bool {
    let max_position_pct = risk_limits
        .get("max_position_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(5.0);

    if proposal.suggested_size_pct > max_position_pct {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_risk_limits_within() {
        let limits = serde_json::json!({"max_position_pct": 5.0});
        let proposal = TradeProposalMsg {
            direction: "long".to_string(),
            confidence: 0.8,
            suggested_size_pct: 3.0,
            stop_loss_pct: 2.0,
            take_profit_pct: 4.0,
            reason: "test".to_string(),
        };
        assert!(check_risk_limits(&limits, &proposal));
    }

    #[test]
    fn test_check_risk_limits_exceeded() {
        let limits = serde_json::json!({"max_position_pct": 2.0});
        let proposal = TradeProposalMsg {
            direction: "long".to_string(),
            confidence: 0.8,
            suggested_size_pct: 3.5,
            stop_loss_pct: 2.0,
            take_profit_pct: 4.0,
            reason: "test".to_string(),
        };
        assert!(!check_risk_limits(&limits, &proposal));
    }

    #[test]
    fn test_check_risk_limits_default() {
        let limits = serde_json::json!({});
        let proposal = TradeProposalMsg {
            direction: "short".to_string(),
            confidence: 0.7,
            suggested_size_pct: 4.5,
            stop_loss_pct: 2.0,
            take_profit_pct: 4.0,
            reason: "test".to_string(),
        };
        // Default max_position_pct is 5.0, so 4.5 should pass
        assert!(check_risk_limits(&limits, &proposal));
    }

    #[test]
    fn test_check_risk_limits_boundary() {
        let limits = serde_json::json!({"max_position_pct": 5.0});
        let proposal = TradeProposalMsg {
            direction: "long".to_string(),
            confidence: 0.9,
            suggested_size_pct: 5.0,
            stop_loss_pct: 2.0,
            take_profit_pct: 4.0,
            reason: "test".to_string(),
        };
        // Exactly at limit — should pass
        assert!(check_risk_limits(&limits, &proposal));
    }
}
