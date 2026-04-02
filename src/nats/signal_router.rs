//! Signal Router NATS worker.
//!
//! Subscribes to `ios.consensus.*`. For each consensus result:
//! - Finds all active user strategies matching the symbol
//! - For each strategy, publishes a personalized signal to
//!   `ios.user.signal.{user_id}` based on their model selection.

use std::sync::Arc;

use futures::StreamExt;
use sqlx::{PgPool, Row};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::messages::{ConsensusMsg, NatsEnvelope, UserSignalMsg};
use super::publisher::NatsPublisher;
use super::NatsClient;

/// Run the signal router worker.
pub async fn run(nats: Arc<NatsClient>, publisher: Arc<NatsPublisher>, pool: PgPool) {
    info!("Signal router started -- subscribing to ios.consensus.*");

    let mut sub = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.consensus.*"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!("Signal router subscribe failed: {e}");
            return;
        }
    };

    while let Some(msg) = sub.next().await {
        let env: NatsEnvelope<ConsensusMsg> = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let symbol = &env.symbol;
        let consensus = &env.data;

        // Find all active strategies that include this symbol
        let strategies = match fetch_active_strategies_for_symbol(&pool, symbol).await {
            Ok(s) => s,
            Err(e) => {
                warn!("{symbol}: failed to fetch strategies for signal routing: {e}");
                continue;
            }
        };

        if strategies.is_empty() {
            debug!("{symbol}: no active strategies for signal routing");
            continue;
        }

        let mut routed = 0usize;

        for strat in &strategies {
            let user_id_str = strat.user_id.to_string();

            // Determine which models from the consensus contributed
            // and are also selected by this user's strategy
            let relevant_models: Vec<String> = consensus
                .model_contributions
                .iter()
                .filter(|contribution| {
                    // contribution format: "model_name:weight%"
                    let model_name = contribution.split(':').next().unwrap_or("");
                    strat.models.contains(&model_name.to_string())
                })
                .cloned()
                .collect();

            // Build suggested action based on direction and confidence
            let suggested_action = if consensus.confidence >= 0.8 {
                format!("Strong {} signal for {}", consensus.direction, symbol)
            } else if consensus.confidence >= 0.6 {
                format!("Moderate {} signal for {}", consensus.direction, symbol)
            } else {
                format!(
                    "Weak {} signal for {} — consider waiting",
                    consensus.direction, symbol
                )
            };

            let signal = UserSignalMsg {
                strategy_id: strat.id.to_string(),
                strategy_name: strat.name.clone(),
                direction: consensus.direction.clone(),
                confidence: consensus.confidence,
                predicted_return: consensus.predicted_return,
                models_used: if relevant_models.is_empty() {
                    strat.models.clone()
                } else {
                    relevant_models
                },
                suggested_action,
            };

            if let Err(e) = publisher
                .publish_user_signal(symbol, &user_id_str, signal)
                .await
            {
                warn!("Signal publish failed for user {}: {e}", &user_id_str[..8]);
                continue;
            }

            routed += 1;
        }

        if routed > 0 {
            info!(
                "SIGNAL ROUTED {symbol}: {} {} to {routed} users (conf={:.0}%)",
                consensus.direction,
                symbol,
                consensus.confidence * 100.0,
            );
        }
    }

    warn!("Signal router subscription ended");
}

/// Minimal strategy info for signal routing.
struct SignalStrategyInfo {
    id: Uuid,
    user_id: Uuid,
    name: String,
    models: Vec<String>,
}

/// Fetch all active strategies whose `symbols` array contains the given symbol.
async fn fetch_active_strategies_for_symbol(
    pool: &PgPool,
    symbol: &str,
) -> Result<Vec<SignalStrategyInfo>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, name, models
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
        .map(|r| SignalStrategyInfo {
            id: r.try_get::<Uuid, _>("id").unwrap_or_default(),
            user_id: r.try_get::<Uuid, _>("user_id").unwrap_or_default(),
            name: r.try_get::<String, _>("name").unwrap_or_default(),
            models: r.try_get::<Vec<String>, _>("models").unwrap_or_default(),
        })
        .collect();

    Ok(strategies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_strategy_info_default() {
        let info = SignalStrategyInfo {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "My Strategy".to_string(),
            models: vec!["catboost".to_string(), "garch".to_string()],
        };
        assert_eq!(info.name, "My Strategy");
        assert_eq!(info.models.len(), 2);
    }

    #[test]
    fn test_model_contribution_filtering() {
        let contributions = vec![
            "catboost:40%".to_string(),
            "garch:30%".to_string(),
            "finbert:30%".to_string(),
        ];
        let user_models = vec!["catboost".to_string(), "finbert".to_string()];

        let relevant: Vec<String> = contributions
            .iter()
            .filter(|c| {
                let model_name = c.split(':').next().unwrap_or("");
                user_models.contains(&model_name.to_string())
            })
            .cloned()
            .collect();

        assert_eq!(relevant.len(), 2);
        assert!(relevant.contains(&"catboost:40%".to_string()));
        assert!(relevant.contains(&"finbert:30%".to_string()));
    }

    #[test]
    fn test_suggested_action_strong() {
        let confidence = 0.85;
        let direction = "long";
        let symbol = "BTCUSDT";

        let action = if confidence >= 0.8 {
            format!("Strong {} signal for {}", direction, symbol)
        } else if confidence >= 0.6 {
            format!("Moderate {} signal for {}", direction, symbol)
        } else {
            format!(
                "Weak {} signal for {} — consider waiting",
                direction, symbol
            )
        };

        assert_eq!(action, "Strong long signal for BTCUSDT");
    }

    #[test]
    fn test_suggested_action_weak() {
        let confidence = 0.45;
        let direction = "short";
        let symbol = "ETHUSDT";

        let action = if confidence >= 0.8 {
            format!("Strong {} signal for {}", direction, symbol)
        } else if confidence >= 0.6 {
            format!("Moderate {} signal for {}", direction, symbol)
        } else {
            format!(
                "Weak {} signal for {} — consider waiting",
                direction, symbol
            )
        };

        assert!(action.contains("Weak"));
        assert!(action.contains("consider waiting"));
    }
}
