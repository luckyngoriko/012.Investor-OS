//! Backtest engine implementation.
//!
//! Loads historical candles from the `prices` table, applies a simple
//! momentum strategy (price > 20-period SMA → long, else flat), and
//! tracks equity, P&L, drawdown, win rate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
#[allow(unused_imports)]
use tracing::info;

/// Request payload for a backtest run.
#[derive(Debug, Clone, Deserialize)]
pub struct BacktestRequest {
    /// Trading symbol (e.g. "BTCUSDT").
    pub symbol: String,
    /// Start of the backtest window (inclusive).
    pub start_date: DateTime<Utc>,
    /// End of the backtest window (inclusive).
    pub end_date: DateTime<Utc>,
    /// Starting capital in USD.
    pub initial_capital: f64,
    /// Optional strategy config (reserved for future use).
    pub strategy_config: Option<serde_json::Value>,
}

/// Result of a completed backtest.
#[derive(Debug, Clone, Serialize)]
pub struct BacktestResult {
    /// Total return as a percentage (e.g. 12.5 means +12.5%).
    pub total_return_pct: f64,
    /// Annualised Sharpe ratio (assuming 0% risk-free rate).
    pub sharpe_ratio: f64,
    /// Maximum drawdown as a percentage (e.g. 8.3 means -8.3%).
    pub max_drawdown_pct: f64,
    /// Win rate as a percentage (e.g. 55.0 means 55%).
    pub win_rate: f64,
    /// Total number of completed round-trip trades.
    pub total_trades: u32,
    /// Equity curve: (timestamp, equity_value) pairs.
    pub equity_curve: Vec<(DateTime<Utc>, f64)>,
}

/// A single candle loaded from the database.
#[derive(Debug, Clone)]
struct Candle {
    timestamp: DateTime<Utc>,
    close: f64,
}

/// Load historical price candles from the `prices` table.
async fn load_candles(
    pool: &PgPool,
    symbol: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<Candle>, String> {
    let rows = sqlx::query(
        r#"
        SELECT "timestamp", close::double precision AS close_f64
        FROM prices
        WHERE ticker = $1
          AND "timestamp" >= $2
          AND "timestamp" <= $3
        ORDER BY "timestamp" ASC
        "#,
    )
    .bind(symbol)
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load prices: {e}"))?;

    let candles: Vec<Candle> = rows
        .iter()
        .map(|row| {
            let ts: DateTime<Utc> = row.try_get("timestamp").unwrap_or_else(|_| Utc::now());
            let close: f64 = row.try_get("close_f64").unwrap_or(0.0);
            Candle {
                timestamp: ts,
                close,
            }
        })
        .collect();

    Ok(candles)
}

/// Compute the simple moving average over the last `period` values.
fn sma(values: &[f64], period: usize) -> Option<f64> {
    if values.len() < period {
        return None;
    }
    let slice = &values[values.len() - period..];
    Some(slice.iter().sum::<f64>() / period as f64)
}

/// Run a backtest using a simple momentum strategy.
///
/// Strategy: if close > 20-period SMA → buy / hold long; else → sell / stay flat.
/// Position sizing: all-in (100% of equity when long).
pub async fn run_backtest(
    pool: &PgPool,
    request: &BacktestRequest,
) -> Result<BacktestResult, String> {
    let candles = load_candles(pool, &request.symbol, request.start_date, request.end_date).await?;

    if candles.is_empty() {
        return Err(format!(
            "No price data found for {} between {} and {}",
            request.symbol, request.start_date, request.end_date
        ));
    }

    info!(
        symbol = %request.symbol,
        candles = candles.len(),
        "Running backtest"
    );

    let sma_period: usize = 20;
    let initial_capital = request.initial_capital;

    let mut equity = initial_capital;
    let mut peak_equity = initial_capital;
    let mut max_drawdown_pct: f64 = 0.0;

    // Position tracking
    let mut in_position = false;
    let mut entry_price: f64 = 0.0;
    let mut position_shares: f64 = 0.0;

    // Trade tracking
    let mut total_trades: u32 = 0;
    let mut winning_trades: u32 = 0;

    // Daily returns for Sharpe
    let mut daily_returns: Vec<f64> = Vec::new();
    let mut prev_equity = initial_capital;

    // Equity curve
    let mut equity_curve: Vec<(DateTime<Utc>, f64)> = Vec::new();

    // Close prices for SMA computation
    let mut close_history: Vec<f64> = Vec::new();

    for candle in &candles {
        close_history.push(candle.close);

        let current_sma = sma(&close_history, sma_period);

        if let Some(sma_val) = current_sma {
            let signal_long = candle.close > sma_val;

            if signal_long && !in_position {
                // BUY
                entry_price = candle.close;
                position_shares = equity / candle.close;
                in_position = true;
            } else if !signal_long && in_position {
                // SELL
                let exit_value = position_shares * candle.close;
                let pnl = exit_value - (position_shares * entry_price);
                equity = exit_value;
                total_trades += 1;
                if pnl > 0.0 {
                    winning_trades += 1;
                }
                in_position = false;
                position_shares = 0.0;
            }
        }

        // Update equity if in position (mark-to-market)
        let current_equity = if in_position {
            position_shares * candle.close
        } else {
            equity
        };

        // Drawdown
        if current_equity > peak_equity {
            peak_equity = current_equity;
        }
        if peak_equity > 0.0 {
            let dd = (peak_equity - current_equity) / peak_equity * 100.0;
            if dd > max_drawdown_pct {
                max_drawdown_pct = dd;
            }
        }

        // Daily return
        if prev_equity > 0.0 {
            let ret = (current_equity - prev_equity) / prev_equity;
            daily_returns.push(ret);
        }
        prev_equity = current_equity;

        equity_curve.push((candle.timestamp, current_equity));
    }

    // Close any open position at the last price
    if in_position {
        if let Some(last_candle) = candles.last() {
            let exit_value = position_shares * last_candle.close;
            let pnl = exit_value - (position_shares * entry_price);
            equity = exit_value;
            total_trades += 1;
            if pnl > 0.0 {
                winning_trades += 1;
            }
        }
    }

    // Compute total return
    let total_return_pct = if initial_capital > 0.0 {
        (equity - initial_capital) / initial_capital * 100.0
    } else {
        0.0
    };

    // Compute Sharpe ratio
    let sharpe_ratio = if daily_returns.len() > 1 {
        let mean_ret = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let variance = daily_returns
            .iter()
            .map(|r| (r - mean_ret).powi(2))
            .sum::<f64>()
            / (daily_returns.len() - 1) as f64;
        let std_dev = variance.sqrt();
        if std_dev > 0.0 {
            // Annualise: assume ~252 trading days (or ~365 for crypto)
            let annualisation = (365.0_f64).sqrt();
            (mean_ret / std_dev) * annualisation
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Win rate
    let win_rate = if total_trades > 0 {
        winning_trades as f64 / total_trades as f64 * 100.0
    } else {
        0.0
    };

    info!(
        symbol = %request.symbol,
        total_return_pct = format!("{:.2}", total_return_pct),
        sharpe_ratio = format!("{:.2}", sharpe_ratio),
        max_drawdown_pct = format!("{:.2}", max_drawdown_pct),
        win_rate = format!("{:.1}", win_rate),
        total_trades,
        "Backtest completed"
    );

    Ok(BacktestResult {
        total_return_pct: (total_return_pct * 100.0).round() / 100.0,
        sharpe_ratio: (sharpe_ratio * 100.0).round() / 100.0,
        max_drawdown_pct: (max_drawdown_pct * 100.0).round() / 100.0,
        win_rate: (win_rate * 100.0).round() / 100.0,
        total_trades,
        equity_curve,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(sma(&values, 3), Some(4.0)); // (3+4+5)/3
        assert_eq!(sma(&values, 5), Some(3.0)); // (1+2+3+4+5)/5
        assert_eq!(sma(&values, 6), None); // not enough data
    }

    #[test]
    fn test_sma_single() {
        let values = vec![10.0];
        assert_eq!(sma(&values, 1), Some(10.0));
        assert_eq!(sma(&values, 2), None);
    }

    #[test]
    fn test_sma_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(sma(&values, 1), None);
    }
}
