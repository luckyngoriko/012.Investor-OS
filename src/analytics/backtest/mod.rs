//! Backtesting Engine
//!
//! S7-D1: Backtesting framework with walk-forward analysis

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use std::collections::HashMap;
use tracing::info;

use crate::analytics::{
    AnalyticsError, DailySnapshot, MarketData, PositionSnapshot, PriceBar, Result, Signal,
    Strategy, Trade,
};
use crate::broker::OrderSide;

/// Backtest configuration
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub commission_rate: Decimal, // Per trade (e.g., 0.001 = 0.1%)
    pub slippage_model: SlippageModel,
    pub rebalance_frequency: Duration,
    pub max_positions: usize,
    pub allow_short: bool,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            start_date: Utc::now() - Duration::days(365),
            end_date: Utc::now(),
            initial_capital: Decimal::from(100000),
            commission_rate: Decimal::from(1) / Decimal::from(1000), // 0.1%
            slippage_model: SlippageModel::Fixed(Decimal::from(1) / Decimal::from(1000)), // 0.1%
            rebalance_frequency: Duration::days(1),
            max_positions: 20,
            allow_short: false,
        }
    }
}

/// Slippage model for execution simulation
#[derive(Debug, Clone)]
pub enum SlippageModel {
    None,
    Fixed(Decimal),    // Fixed percentage (e.g., 0.001 = 0.1%)
    Variable(Decimal), // Percentage based on volatility
}

impl SlippageModel {
    /// Apply slippage to price
    pub fn apply(&self, price: Decimal, side: OrderSide) -> Decimal {
        match self {
            SlippageModel::None => price,
            SlippageModel::Fixed(pct) => {
                let adjustment = price * *pct;
                match side {
                    OrderSide::Buy => price + adjustment,  // Pay more
                    OrderSide::Sell => price - adjustment, // Receive less
                }
            }
            SlippageModel::Variable(pct) => {
                // Simplified - would use volatility in real implementation
                let adjustment = price * *pct;
                match side {
                    OrderSide::Buy => price + adjustment,
                    OrderSide::Sell => price - adjustment,
                }
            }
        }
    }
}

/// Backtest engine
pub struct Backtest {
    config: BacktestConfig,
    strategy: Box<dyn Strategy>,
    trades: Vec<Trade>,
    daily_snapshots: Vec<DailySnapshot>,
    current_positions: HashMap<String, Decimal>, // ticker -> quantity
    cash: Decimal,
    current_date: DateTime<Utc>,
}

/// Backtest result
#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub config: BacktestConfig,
    pub total_return: Decimal, // Total return percentage
    pub annualized_return: Decimal,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: Decimal,
    pub avg_trade_return: Decimal,
    pub max_drawdown: Decimal, // Maximum drawdown percentage
    pub sharpe_ratio: Decimal,
    pub sortino_ratio: Decimal,
    pub daily_returns: Vec<DailyReturn>,
    pub equity_curve: Vec<EquityPoint>,
    pub trades: Vec<Trade>,
}

/// Daily return
#[derive(Debug, Clone)]
pub struct DailyReturn {
    pub date: DateTime<Utc>,
    pub return_pct: Decimal,
    pub nav: Decimal,
}

/// Equity curve point
#[derive(Debug, Clone)]
pub struct EquityPoint {
    pub date: DateTime<Utc>,
    pub equity: Decimal,
    pub cash: Decimal,
    pub positions_value: Decimal,
}

/// Walk-forward configuration
#[derive(Debug, Clone)]
pub struct WalkForwardConfig {
    pub train_window: Duration, // Training period
    pub test_window: Duration,  // Testing period
    pub step_size: Duration,    // How much to advance each iteration
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            train_window: Duration::days(252), // 1 year
            test_window: Duration::days(63),   // 3 months
            step_size: Duration::days(63),     // 3 months
        }
    }
}

/// Walk-forward result
#[derive(Debug, Clone)]
pub struct WalkForwardResult {
    pub periods: Vec<PeriodResult>,
    pub aggregate_return: Decimal,
    pub aggregate_sharpe: Decimal,
}

/// Single period result in walk-forward
#[derive(Debug, Clone)]
pub struct PeriodResult {
    pub train_start: DateTime<Utc>,
    pub train_end: DateTime<Utc>,
    pub test_start: DateTime<Utc>,
    pub test_end: DateTime<Utc>,
    pub test_return: Decimal,
    pub test_sharpe: Decimal,
}

impl Backtest {
    /// Create a new backtest
    pub fn new(config: BacktestConfig, strategy: Box<dyn Strategy>) -> Self {
        let cash = config.initial_capital;
        let current_date = config.start_date;

        Self {
            config,
            strategy,
            trades: Vec::new(),
            daily_snapshots: Vec::new(),
            current_positions: HashMap::new(),
            cash,
            current_date,
        }
    }

    /// Run the backtest
    pub async fn run(
        &mut self,
        historical_data: &HashMap<String, Vec<PriceBar>>,
    ) -> Result<BacktestResult> {
        info!(
            "Starting backtest from {} to {}",
            self.config.start_date.format("%Y-%m-%d"),
            self.config.end_date.format("%Y-%m-%d")
        );

        let mut current_date = self.config.start_date;

        while current_date < self.config.end_date {
            // Get market data for current date
            let market_data = self.get_market_data(current_date, historical_data);

            if !market_data.prices.is_empty() {
                // Generate signals from strategy
                let signals = self.strategy.generate_signals(&market_data).await;

                // Execute trades based on signals
                for signal in signals {
                    self.process_signal(signal, &market_data).await?;
                }

                // Record daily snapshot
                self.record_snapshot(current_date, &market_data);
            }

            current_date += self.config.rebalance_frequency;
        }

        // Calculate results
        let result = self.calculate_result();

        info!(
            "Backtest complete. Total return: {}%, Sharpe: {}",
            result.total_return, result.sharpe_ratio
        );

        Ok(result)
    }

    /// Run walk-forward analysis (placeholder - requires Clone bound on Strategy)
    pub async fn walk_forward(
        &mut self,
        _config: &WalkForwardConfig,
        _historical_data: &HashMap<String, Vec<PriceBar>>,
    ) -> Result<WalkForwardResult> {
        info!("Walk-forward analysis requires Clone bound on Strategy");

        // Placeholder implementation
        Ok(WalkForwardResult {
            periods: Vec::new(),
            aggregate_return: Decimal::ZERO,
            aggregate_sharpe: Decimal::ZERO,
        })
    }

    // Private methods

    fn get_market_data(
        &self,
        date: DateTime<Utc>,
        historical_data: &HashMap<String, Vec<PriceBar>>,
    ) -> MarketData {
        let mut prices = std::collections::HashMap::new();

        for (ticker, bars) in historical_data {
            // Find bar closest to date
            if let Some(bar) = bars.iter().find(|b| b.timestamp >= date) {
                prices.insert(ticker.clone(), bar.clone());
            }
        }

        MarketData {
            timestamp: date,
            prices,
        }
    }

    async fn process_signal(&mut self, signal: Signal, market_data: &MarketData) -> Result<()> {
        let Some(price_bar) = market_data.prices.get(&signal.ticker) else {
            return Ok(());
        };

        let current_price = Decimal::try_from(price_bar.close)
            .map_err(|e| AnalyticsError::Calculation(e.to_string()))?;

        // Calculate position size
        let portfolio_value = self.calculate_portfolio_value(market_data);
        let target_quantity = self.strategy.position_size(&signal, portfolio_value);

        // Get current position
        let current_quantity = *self
            .current_positions
            .get(&signal.ticker)
            .unwrap_or(&Decimal::ZERO);

        // Calculate order quantity
        let order_quantity = target_quantity - current_quantity;

        if order_quantity.abs() > Decimal::ZERO {
            let side = if order_quantity > Decimal::ZERO {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };

            // Apply slippage
            let execution_price = self.config.slippage_model.apply(current_price, side);

            // Calculate commission
            let trade_value = execution_price * order_quantity.abs();
            let commission = trade_value * self.config.commission_rate;

            // Update cash
            let cost = execution_price * order_quantity + commission;
            self.cash -= cost;

            // Update position
            let new_quantity = current_quantity + order_quantity;
            if new_quantity == Decimal::ZERO {
                self.current_positions.remove(&signal.ticker);
            } else {
                self.current_positions
                    .insert(signal.ticker.clone(), new_quantity);
            }

            // Record trade
            self.trades.push(Trade {
                timestamp: market_data.timestamp,
                ticker: signal.ticker,
                side,
                quantity: order_quantity.abs(),
                price: execution_price,
                commission,
            });
        }

        Ok(())
    }

    fn calculate_portfolio_value(&self, market_data: &MarketData) -> Decimal {
        let positions_value: Decimal = self
            .current_positions
            .iter()
            .map(|(ticker, qty)| {
                if let Some(bar) = market_data.prices.get(ticker) {
                    Decimal::try_from(bar.close).unwrap_or_default() * qty
                } else {
                    Decimal::ZERO
                }
            })
            .sum();

        self.cash + positions_value
    }

    fn record_snapshot(&mut self, date: DateTime<Utc>, market_data: &MarketData) {
        let positions: Vec<PositionSnapshot> = self
            .current_positions
            .iter()
            .map(|(ticker, qty)| {
                if let Some(bar) = market_data.prices.get(ticker) {
                    let price = Decimal::try_from(bar.close).unwrap_or_default();
                    PositionSnapshot {
                        ticker: ticker.clone(),
                        quantity: *qty,
                        market_price: price,
                        market_value: price * qty,
                    }
                } else {
                    PositionSnapshot {
                        ticker: ticker.clone(),
                        quantity: *qty,
                        market_price: Decimal::ZERO,
                        market_value: Decimal::ZERO,
                    }
                }
            })
            .collect();

        let positions_value: Decimal = positions.iter().map(|p| p.market_value).sum();
        let total_value = self.cash + positions_value;

        self.daily_snapshots.push(DailySnapshot {
            date,
            nav: total_value,
            cash: self.cash,
            positions_value,
            positions,
        });
    }

    fn calculate_result(&self) -> BacktestResult {
        // Calculate returns
        let mut daily_returns = Vec::new();
        let mut equity_curve = Vec::new();

        for (i, snapshot) in self.daily_snapshots.iter().enumerate() {
            equity_curve.push(EquityPoint {
                date: snapshot.date,
                equity: snapshot.nav,
                cash: snapshot.cash,
                positions_value: snapshot.positions_value,
            });

            if i > 0 {
                let prev_nav = self.daily_snapshots[i - 1].nav;
                let return_pct = if prev_nav > Decimal::ZERO {
                    (snapshot.nav - prev_nav) / prev_nav
                } else {
                    Decimal::ZERO
                };

                daily_returns.push(DailyReturn {
                    date: snapshot.date,
                    return_pct,
                    nav: snapshot.nav,
                });
            }
        }

        // Calculate total return
        let total_return = if let (Some(first), Some(last)) =
            (self.daily_snapshots.first(), self.daily_snapshots.last())
        {
            if first.nav > Decimal::ZERO {
                (last.nav - first.nav) / first.nav
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };

        // Calculate win/loss by matching buy/sell pairs (FIFO)
        let mut entry_prices: HashMap<String, Vec<Decimal>> = HashMap::new();
        let mut winning_count = 0usize;
        let mut total_trade_pnl = Decimal::ZERO;
        let mut round_trips = 0usize;

        for trade in &self.trades {
            match trade.side {
                OrderSide::Buy => {
                    entry_prices
                        .entry(trade.ticker.clone())
                        .or_default()
                        .push(trade.price);
                }
                OrderSide::Sell => {
                    if let Some(entries) = entry_prices.get_mut(&trade.ticker) {
                        if let Some(entry_price) = entries.first().copied() {
                            let pnl =
                                (trade.price - entry_price) * trade.quantity - trade.commission;
                            total_trade_pnl += pnl;
                            if pnl > Decimal::ZERO {
                                winning_count += 1;
                            }
                            round_trips += 1;
                            entries.remove(0);
                        }
                    }
                }
            }
        }

        let winning_trades = winning_count;
        let losing_trades = round_trips.saturating_sub(winning_count);
        let win_rate = if round_trips > 0 {
            Decimal::from(winning_count as i32) / Decimal::from(round_trips as i32)
        } else {
            Decimal::ZERO
        };
        let avg_trade_return = if round_trips > 0 {
            total_trade_pnl / Decimal::from(round_trips as i32) / self.config.initial_capital
        } else {
            Decimal::ZERO
        };

        // Calculate max drawdown
        let max_drawdown = self.calculate_max_drawdown(&equity_curve);

        // Calculate Sharpe ratio (simplified - would need risk-free rate)
        let sharpe_ratio = self.calculate_sharpe_ratio(&daily_returns);

        BacktestResult {
            config: self.config.clone(),
            total_return,
            annualized_return: total_return, // Simplified
            total_trades: self.trades.len(),
            winning_trades,
            losing_trades,
            win_rate,
            avg_trade_return,
            max_drawdown,
            sharpe_ratio,
            sortino_ratio: sharpe_ratio, // Simplified
            daily_returns,
            equity_curve,
            trades: self.trades.clone(),
        }
    }

    fn calculate_max_drawdown(&self, equity_curve: &[EquityPoint]) -> Decimal {
        let mut max_dd = Decimal::ZERO;
        let mut peak = Decimal::ZERO;

        for point in equity_curve {
            if point.equity > peak {
                peak = point.equity;
            }

            if peak > Decimal::ZERO {
                let dd = (peak - point.equity) / peak;
                if dd > max_dd {
                    max_dd = dd;
                }
            }
        }

        -max_dd // Return as negative number
    }

    fn calculate_sharpe_ratio(&self, daily_returns: &[DailyReturn]) -> Decimal {
        if daily_returns.len() < 2 {
            return Decimal::ZERO;
        }

        let returns: Vec<Decimal> = daily_returns.iter().map(|r| r.return_pct).collect();

        let mean = returns.iter().sum::<Decimal>() / Decimal::from(returns.len() as i32);

        // Calculate standard deviation
        let variance = returns
            .iter()
            .map(|r| (*r - mean) * (*r - mean))
            .sum::<Decimal>()
            / Decimal::from(returns.len() as i32);

        let std_dev = variance.sqrt().unwrap_or(Decimal::ONE);

        if std_dev > Decimal::ZERO {
            // Annualized Sharpe (assuming 252 trading days)
            let sqrt_252 = Decimal::from(252u32).sqrt().unwrap_or(Decimal::ONE);
            mean / std_dev * sqrt_252
        } else {
            Decimal::ZERO
        }
    }
}
