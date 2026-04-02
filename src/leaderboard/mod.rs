//! Performance Leaderboard Module (Wave 3 Task 18).
//!
//! Queries `user_trades` grouped by `strategy_id` to compute performance
//! metrics: total return, Sharpe ratio, max drawdown, win rate, trade count.
//! Results are ranked by Sharpe ratio descending.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tracing::info;

/// A single row on the performance leaderboard.
#[derive(Debug, Clone, Serialize)]
pub struct LeaderboardEntry {
    /// 1-based rank (by Sharpe ratio descending).
    pub rank: u32,
    /// Strategy name (from `user_strategies.name`).
    pub strategy_name: String,
    /// Anonymised creator label (e.g. "Trader #3").
    pub creator_anonymous: String,
    /// Total return as a percentage.
    pub total_return: f64,
    /// Sharpe ratio (annualised, 0% risk-free).
    pub sharpe: f64,
    /// Maximum drawdown as a percentage.
    pub drawdown: f64,
    /// Win rate as a percentage.
    pub win_rate: f64,
    /// Total number of closed trades.
    pub trades: u32,
}

/// Supported timeframe filters.
#[derive(Debug, Clone, Deserialize)]
pub struct LeaderboardQuery {
    /// Timeframe string: "7d", "30d", or "all".  Defaults to "30d".
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
}

fn default_timeframe() -> String {
    "30d".to_string()
}

/// Raw per-strategy aggregate loaded from the database.
#[derive(Debug)]
struct StrategyStats {
    strategy_id: uuid::Uuid,
    strategy_name: String,
    creator_index: i64,
    total_pnl: f64,
    total_cost: f64,
    winning: i64,
    losing: i64,
    returns: Vec<f64>,
}

/// Fetch the leaderboard for the given timeframe.
///
/// Queries `user_trades` joined with `user_strategies`, groups by
/// strategy, computes metrics, and returns entries sorted by Sharpe
/// ratio descending.
pub async fn get_leaderboard(
    pool: &PgPool,
    timeframe: &str,
) -> Result<Vec<LeaderboardEntry>, String> {
    // Determine the cutoff timestamp
    let cutoff = match timeframe {
        "7d" => Some(Utc::now() - Duration::days(7)),
        "30d" => Some(Utc::now() - Duration::days(30)),
        "all" | _ if timeframe == "all" => None,
        _ => Some(Utc::now() - Duration::days(30)),
    };

    // -----------------------------------------------------------------
    // Query: aggregate trades per strategy
    // -----------------------------------------------------------------
    let query = if let Some(since) = cutoff {
        sqlx::query(
            r#"
            SELECT
                t.strategy_id,
                COALESCE(s.name, 'Unknown') AS strategy_name,
                DENSE_RANK() OVER (ORDER BY s.user_id) AS creator_idx,
                SUM(CASE WHEN t.side = 'sell' THEN t.quantity * t.price ELSE -(t.quantity * t.price) END)::double precision AS total_pnl,
                SUM(t.quantity * t.price)::double precision AS total_cost,
                COUNT(*) FILTER (WHERE t.pnl > 0) AS winning,
                COUNT(*) FILTER (WHERE t.pnl <= 0) AS losing,
                ARRAY_AGG(t.pnl::double precision ORDER BY t.executed_at ASC) AS returns_arr
            FROM user_trades t
            LEFT JOIN user_strategies s ON s.id = t.strategy_id
            WHERE t.strategy_id IS NOT NULL
              AND t.executed_at >= $1
            GROUP BY t.strategy_id, s.name, s.user_id
            HAVING COUNT(*) >= 2
            "#,
        )
        .bind(since)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            r#"
            SELECT
                t.strategy_id,
                COALESCE(s.name, 'Unknown') AS strategy_name,
                DENSE_RANK() OVER (ORDER BY s.user_id) AS creator_idx,
                SUM(CASE WHEN t.side = 'sell' THEN t.quantity * t.price ELSE -(t.quantity * t.price) END)::double precision AS total_pnl,
                SUM(t.quantity * t.price)::double precision AS total_cost,
                COUNT(*) FILTER (WHERE t.pnl > 0) AS winning,
                COUNT(*) FILTER (WHERE t.pnl <= 0) AS losing,
                ARRAY_AGG(t.pnl::double precision ORDER BY t.executed_at ASC) AS returns_arr
            FROM user_trades t
            LEFT JOIN user_strategies s ON s.id = t.strategy_id
            WHERE t.strategy_id IS NOT NULL
            GROUP BY t.strategy_id, s.name, s.user_id
            HAVING COUNT(*) >= 2
            "#,
        )
        .fetch_all(pool)
        .await
    };

    let rows = query.map_err(|e| format!("Leaderboard query failed: {e}"))?;

    // -----------------------------------------------------------------
    // Parse rows into StrategyStats
    // -----------------------------------------------------------------
    let mut stats: Vec<StrategyStats> = Vec::new();

    for row in &rows {
        let strategy_id: uuid::Uuid = row.try_get("strategy_id").unwrap_or_default();
        let strategy_name: String = row
            .try_get("strategy_name")
            .unwrap_or_else(|_| "Unknown".to_string());
        let creator_index: i64 = row.try_get("creator_idx").unwrap_or(0);
        let total_pnl: f64 = row.try_get("total_pnl").unwrap_or(0.0);
        let total_cost: f64 = row.try_get("total_cost").unwrap_or(1.0);
        let winning: i64 = row.try_get("winning").unwrap_or(0);
        let losing: i64 = row.try_get("losing").unwrap_or(0);
        let returns: Vec<f64> = row.try_get("returns_arr").unwrap_or_default();

        stats.push(StrategyStats {
            strategy_id,
            strategy_name,
            creator_index,
            total_pnl,
            total_cost,
            winning,
            losing,
            returns,
        });
    }

    // -----------------------------------------------------------------
    // Compute metrics per strategy
    // -----------------------------------------------------------------
    let mut entries: Vec<LeaderboardEntry> = stats
        .iter()
        .map(|s| {
            let total_return = if s.total_cost.abs() > 0.0 {
                (s.total_pnl / s.total_cost) * 100.0
            } else {
                0.0
            };

            let trade_count = (s.winning + s.losing) as u32;
            let win_rate = if trade_count > 0 {
                s.winning as f64 / trade_count as f64 * 100.0
            } else {
                0.0
            };

            // Sharpe ratio from per-trade returns
            let sharpe = compute_sharpe(&s.returns);

            // Max drawdown from cumulative PnL
            let drawdown = compute_max_drawdown(&s.returns);

            LeaderboardEntry {
                rank: 0, // will be assigned after sorting
                strategy_name: s.strategy_name.clone(),
                creator_anonymous: format!("Trader #{}", s.creator_index),
                total_return: round2(total_return),
                sharpe: round2(sharpe),
                drawdown: round2(drawdown),
                win_rate: round2(win_rate),
                trades: trade_count,
            }
        })
        .collect();

    // Sort by Sharpe descending
    entries.sort_by(|a, b| {
        b.sharpe
            .partial_cmp(&a.sharpe)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Assign ranks
    for (i, entry) in entries.iter_mut().enumerate() {
        entry.rank = (i + 1) as u32;
    }

    info!(
        timeframe = %timeframe,
        strategies = entries.len(),
        "Leaderboard computed"
    );

    Ok(entries)
}

/// Annualised Sharpe ratio from a series of per-trade returns.
fn compute_sharpe(returns: &[f64]) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std_dev = variance.sqrt();
    if std_dev <= 0.0 {
        return 0.0;
    }
    // Annualise assuming ~252 trading days
    (mean / std_dev) * (252.0_f64).sqrt()
}

/// Max drawdown as a percentage from a series of per-trade PnL values.
fn compute_max_drawdown(pnl_series: &[f64]) -> f64 {
    if pnl_series.is_empty() {
        return 0.0;
    }
    let mut cumulative = 0.0_f64;
    let mut peak = 0.0_f64;
    let mut max_dd = 0.0_f64;

    for &pnl in pnl_series {
        cumulative += pnl;
        if cumulative > peak {
            peak = cumulative;
        }
        if peak > 0.0 {
            let dd = (peak - cumulative) / peak * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// Round to 2 decimal places.
fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sharpe_basic() {
        let returns = vec![0.01, 0.02, -0.005, 0.015, 0.01, -0.002, 0.008];
        let sharpe = compute_sharpe(&returns);
        // Should be positive for a generally positive series
        assert!(sharpe > 0.0, "Sharpe should be positive, got {sharpe}");
    }

    #[test]
    fn test_compute_sharpe_insufficient() {
        assert_eq!(compute_sharpe(&[0.01]), 0.0);
        assert_eq!(compute_sharpe(&[]), 0.0);
    }

    #[test]
    fn test_compute_sharpe_zero_variance() {
        let returns = vec![0.01, 0.01, 0.01, 0.01];
        assert_eq!(compute_sharpe(&returns), 0.0);
    }

    #[test]
    fn test_max_drawdown_basic() {
        // Goes up 10, then loses 4 → drawdown = 4/10 = 40%
        let pnl = vec![5.0, 5.0, -4.0];
        let dd = compute_max_drawdown(&pnl);
        assert!((dd - 40.0).abs() < 0.01, "Expected ~40%, got {dd}");
    }

    #[test]
    fn test_max_drawdown_no_loss() {
        let pnl = vec![1.0, 2.0, 3.0];
        assert_eq!(compute_max_drawdown(&pnl), 0.0);
    }

    #[test]
    fn test_max_drawdown_empty() {
        assert_eq!(compute_max_drawdown(&[]), 0.0);
    }

    #[test]
    fn test_round2() {
        assert_eq!(round2(1.2345), 1.23);
        assert_eq!(round2(1.235), 1.24);
        assert_eq!(round2(-0.005), -0.01);
    }

    #[test]
    fn test_default_timeframe() {
        assert_eq!(default_timeframe(), "30d");
    }
}
