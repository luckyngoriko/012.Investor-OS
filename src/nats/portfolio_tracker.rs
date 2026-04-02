//! Portfolio Tracker NATS worker.
//!
//! Subscribes to `ios.trade.fill.*`. For each fill:
//! - Inserts the trade into `user_trades` table
//! - Publishes `ios.portfolio.update.{user_id}` with updated portfolio data

use std::sync::Arc;

use futures::StreamExt;
use sqlx::{PgPool, Row};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::messages::{NatsEnvelope, PortfolioUpdateMsg, TradeFillMsg};
use super::publisher::NatsPublisher;
use super::NatsClient;

/// Run the portfolio tracker worker.
pub async fn run(nats: Arc<NatsClient>, publisher: Arc<NatsPublisher>, pool: PgPool) {
    info!("Portfolio tracker started -- subscribing to ios.trade.fill.*");

    let mut sub = match nats
        .client()
        .subscribe(async_nats::Subject::from("ios.trade.fill.*"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!("Portfolio tracker subscribe failed: {e}");
            return;
        }
    };

    while let Some(msg) = sub.next().await {
        let env: NatsEnvelope<TradeFillMsg> = match serde_json::from_slice(&msg.payload) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let symbol = &env.symbol;
        let fill = &env.data;

        // Extract user_id from subject: ios.trade.fill.{user_id}
        let user_id_str = msg.subject.as_str().rsplit('.').next().unwrap_or_default();

        let user_id = match Uuid::parse_str(user_id_str) {
            Ok(id) => id,
            Err(_) => {
                debug!("Portfolio tracker: invalid user_id in subject: {user_id_str}");
                continue;
            }
        };

        let strategy_id = match Uuid::parse_str(&fill.strategy_id) {
            Ok(id) => id,
            Err(_) => {
                debug!(
                    "Portfolio tracker: invalid strategy_id: {}",
                    fill.strategy_id
                );
                continue;
            }
        };

        // Insert trade into user_trades
        let trade_id = Uuid::new_v4();
        let side = if fill.direction == "long" {
            "buy"
        } else {
            "sell"
        };

        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO user_trades (id, user_id, strategy_id, symbol, side, quantity, price, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(trade_id)
        .bind(user_id)
        .bind(strategy_id)
        .bind(symbol)
        .bind(side)
        .bind(fill.size_pct) // quantity as size_pct for now
        .bind(fill.entry_price)
        .bind(&fill.fill_status)
        .execute(&pool)
        .await
        {
            warn!("Failed to insert trade for user {}: {e}", &user_id_str[..8]);
            // Don't stop — still try to publish update
        } else {
            debug!(
                "Trade recorded: {} {} {} for user {}",
                side,
                symbol,
                fill.size_pct,
                &user_id_str[..8],
            );
        }

        // Calculate portfolio value (sum of all fills for this user)
        let portfolio_value = match sqlx::query(
            r#"
            SELECT COALESCE(SUM(quantity * price), 0.0) AS total_value
            FROM user_trades
            WHERE user_id = $1 AND status != 'cancelled'
            "#,
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await
        {
            Ok(row) => row.try_get::<f64, _>("total_value").unwrap_or(0.0),
            Err(e) => {
                debug!("Portfolio value query failed: {e}");
                0.0
            }
        };

        // Publish portfolio update
        let update = PortfolioUpdateMsg {
            trade_id: trade_id.to_string(),
            strategy_id: fill.strategy_id.clone(),
            direction: fill.direction.clone(),
            size_pct: fill.size_pct,
            entry_price: fill.entry_price,
            portfolio_value,
        };

        if let Err(e) = publisher
            .publish_portfolio_update(symbol, user_id_str, update)
            .await
        {
            warn!(
                "Portfolio update publish failed for user {}: {e}",
                &user_id_str[..8]
            );
        }

        info!(
            "PORTFOLIO UPDATE user {}: {} {} {:.2}% — portfolio=${:.2}",
            &user_id_str[..8],
            fill.direction,
            symbol,
            fill.size_pct,
            portfolio_value,
        );
    }

    warn!("Portfolio tracker subscription ended");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_mapping_long() {
        let direction = "long";
        let side = if direction == "long" { "buy" } else { "sell" };
        assert_eq!(side, "buy");
    }

    #[test]
    fn test_side_mapping_short() {
        let direction = "short";
        let side = if direction == "long" { "buy" } else { "sell" };
        assert_eq!(side, "sell");
    }

    #[test]
    fn test_user_id_extraction_from_subject() {
        let subject = "ios.trade.fill.550e8400-e29b-41d4-a716-446655440000";
        let user_id_str = subject.rsplit('.').next().unwrap_or_default();
        assert_eq!(user_id_str, "550e8400-e29b-41d4-a716-446655440000");
        assert!(Uuid::parse_str(user_id_str).is_ok());
    }

    #[test]
    fn test_portfolio_update_msg_creation() {
        let update = PortfolioUpdateMsg {
            trade_id: Uuid::new_v4().to_string(),
            strategy_id: Uuid::new_v4().to_string(),
            direction: "long".to_string(),
            size_pct: 2.5,
            entry_price: 67000.0,
            portfolio_value: 125000.0,
        };
        assert_eq!(update.direction, "long");
        assert!((update.size_pct - 2.5).abs() < f64::EPSILON);
        assert!((update.portfolio_value - 125000.0).abs() < f64::EPSILON);
    }
}
