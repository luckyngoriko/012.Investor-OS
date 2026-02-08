//! Paper Trading Simulator
//!
//! Virtual trading environment using real market data
//! Uses IB API pattern from Sprint 6

use chrono::{DateTime, Utc, Duration};
use rust_decimal::Decimal;
use std::collections::HashMap;

use super::{
    Action, DailyResult, EpochMetrics, EpochResult, FeatureVector, TradingDecision,
    memory::{MarketCondition, MarketRegime, TradeOutcome, TradingExperience, TrendDirection, VolatilityLevel, ExitReason},
    strategist::Sentiment,
    DecimalExt,
};

/// Paper trading simulator - virtual trading environment
pub struct PaperTradingSimulator {
    portfolio: VirtualPortfolio,
    market_data: Vec<MarketDataPoint>,
    current_index: usize,
    commission_per_trade: Decimal,
    slippage_pct: Decimal,
}

/// Virtual portfolio state
#[derive(Debug, Clone)]
pub struct VirtualPortfolio {
    pub cash: Decimal,
    pub positions: HashMap<String, Position>,
    pub trade_history: Vec<ExecutedTrade>,
    pub equity_curve: Vec<EquityPoint>,
    pub start_value: Decimal,
}

/// Position in a security
#[derive(Debug, Clone)]
pub struct Position {
    pub ticker: String,
    pub quantity: u32,
    pub avg_entry_price: Decimal,
    pub entry_date: DateTime<Utc>,
    pub unrealized_pnl: Decimal,
    pub stop_loss: Option<Decimal>,
    pub take_profit: Option<Decimal>,
}

/// Executed trade record
#[derive(Debug, Clone)]
pub struct ExecutedTrade {
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub action: Action,
    pub quantity: u32,
    pub price: Decimal,
    pub commission: Decimal,
    pub realized_pnl: Option<Decimal>,
}

/// Point in equity curve
#[derive(Debug, Clone)]
pub struct EquityPoint {
    pub timestamp: DateTime<Utc>,
    pub total_value: Decimal,
    pub cash: Decimal,
    pub positions_value: Decimal,
}

/// Market data point for simulation
#[derive(Debug, Clone)]
pub struct MarketDataPoint {
    pub timestamp: DateTime<Utc>,
    pub ticker: String,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: u64,
    pub indicators: TechnicalIndicators,
}

/// Technical indicators
#[derive(Debug, Clone, Default)]
pub struct TechnicalIndicators {
    pub rsi: Option<f64>,
    pub macd: Option<f64>,
    pub macd_signal: Option<f64>,
    pub vwap: Option<Decimal>,
    pub atr: Option<Decimal>,
    pub sma_20: Option<Decimal>,
    pub sma_50: Option<Decimal>,
    pub bb_upper: Option<Decimal>,
    pub bb_lower: Option<Decimal>,
}

/// Simulation configuration
#[derive(Debug, Clone)]
pub struct SimulatorConfig {
    pub initial_capital: Decimal,
    pub commission_per_trade: Decimal,
    pub slippage_pct: Decimal,
    pub max_position_pct: Decimal,
    pub max_positions: u32,
    pub default_stop_loss_pct: Decimal,
    pub default_take_profit_pct: Decimal,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            initial_capital: Decimal::from(1000),
            commission_per_trade: Decimal::from_str_exact("1.00").unwrap_or(Decimal::ONE),
            slippage_pct: Decimal::from_str_exact("0.001").unwrap_or(Decimal::from(1) / Decimal::from(1000)),
            max_position_pct: Decimal::from_str_exact("0.20").unwrap_or(Decimal::from(20) / Decimal::from(100)),
            max_positions: 5,
            default_stop_loss_pct: Decimal::from_str_exact("0.05").unwrap_or(Decimal::from(5) / Decimal::from(100)),
            default_take_profit_pct: Decimal::from_str_exact("0.10").unwrap_or(Decimal::from(10) / Decimal::from(100)),
        }
    }
}

impl PaperTradingSimulator {
    pub fn new(config: SimulatorConfig) -> Self {
        let initial_capital = config.initial_capital;
        
        Self {
            portfolio: VirtualPortfolio {
                cash: initial_capital,
                positions: HashMap::new(),
                trade_history: Vec::new(),
                equity_curve: vec![EquityPoint {
                    timestamp: Utc::now(),
                    total_value: initial_capital,
                    cash: initial_capital,
                    positions_value: Decimal::ZERO,
                }],
                start_value: initial_capital,
            },
            market_data: Vec::new(),
            current_index: 0,
            commission_per_trade: config.commission_per_trade,
            slippage_pct: config.slippage_pct,
        }
    }
    
    /// Load historical market data for simulation
    pub fn load_market_data(&mut self, data: Vec<MarketDataPoint>) {
        self.market_data = data;
        self.current_index = 0;
    }
    
    /// Get current market data point
    pub fn current_data(&self) -> Option<&MarketDataPoint> {
        self.market_data.get(self.current_index)
    }
    
    /// Execute a trading decision
    pub fn execute(&mut self, decision: &TradingDecision) -> TradeOutcome {
        let current_data = match self.current_data() {
            Some(d) => d.clone(),
            None => return self.empty_outcome(),
        };
        
        let timestamp = current_data.timestamp;
        let price = self.apply_slippage(current_data.close, &decision.action);
        
        match decision.action {
            Action::Buy => self.execute_buy(decision, price, timestamp, &current_data),
            Action::Sell => self.execute_sell(decision, price, timestamp, &current_data),
            Action::Hold => self.hold_outcome(&current_data),
        }
    }
    
    /// Execute buy order
    fn execute_buy(
        &mut self,
        decision: &TradingDecision,
        price: Decimal,
        timestamp: DateTime<Utc>,
        data: &MarketDataPoint,
    ) -> TradeOutcome {
        let quantity = decision.quantity.unwrap_or(0);
        if quantity == 0 {
            return self.empty_outcome();
        }
        
        let total_cost = price * Decimal::from(quantity) + self.commission_per_trade;
        
        // Check if we have enough cash
        if total_cost > self.portfolio.cash {
            return TradeOutcome {
                profit_loss: Decimal::ZERO,
                profit_loss_pct: 0.0,
                holding_period_days: 0,
                max_favorable_excursion: Decimal::ZERO,
                max_adverse_excursion: Decimal::ZERO,
                exit_reason: ExitReason::Manual,
                success: false,
            };
        }
        
        // Create or update position
        let position = self.portfolio.positions
            .entry(decision.ticker.clone())
            .or_insert(Position {
                ticker: decision.ticker.clone(),
                quantity: 0,
                avg_entry_price: Decimal::ZERO,
                entry_date: timestamp,
                unrealized_pnl: Decimal::ZERO,
                stop_loss: decision.quantity.map(|_| price * (Decimal::ONE - Decimal::from_str_exact("0.05").unwrap())),
                take_profit: decision.quantity.map(|_| price * (Decimal::ONE + Decimal::from_str_exact("0.10").unwrap())),
            });
        
        // Update position
        let total_quantity = position.quantity + quantity;
        position.avg_entry_price = (position.avg_entry_price * Decimal::from(position.quantity) + price * Decimal::from(quantity)) 
            / Decimal::from(total_quantity);
        position.quantity = total_quantity;
        
        // Deduct cash
        self.portfolio.cash -= total_cost;
        
        // Record trade
        self.portfolio.trade_history.push(ExecutedTrade {
            timestamp,
            ticker: decision.ticker.clone(),
            action: Action::Buy,
            quantity,
            price,
            commission: self.commission_per_trade,
            realized_pnl: None,
        });
        
        TradeOutcome {
            profit_loss: -self.commission_per_trade,
            profit_loss_pct: -0.1, // Approximate
            holding_period_days: 0,
            max_favorable_excursion: Decimal::ZERO,
            max_adverse_excursion: Decimal::ZERO,
            exit_reason: ExitReason::Manual,
            success: false, // Not realized yet
        }
    }
    
    /// Execute sell order
    fn execute_sell(
        &mut self,
        decision: &TradingDecision,
        price: Decimal,
        timestamp: DateTime<Utc>,
        data: &MarketDataPoint,
    ) -> TradeOutcome {
        let position = match self.portfolio.positions.get(&decision.ticker) {
            Some(p) if p.quantity > 0 => p.clone(),
            _ => return self.empty_outcome(),
        };
        
        let quantity = decision.quantity.unwrap_or(position.quantity).min(position.quantity);
        let sell_value = price * Decimal::from(quantity);
        let buy_value = position.avg_entry_price * Decimal::from(quantity);
        let gross_pnl = sell_value - buy_value;
        let net_pnl = gross_pnl - self.commission_per_trade;
        let pnl_pct = (net_pnl / buy_value).to_f64().unwrap_or(0.0) * 100.0;
        
        // Update or remove position
        if quantity >= position.quantity {
            self.portfolio.positions.remove(&decision.ticker);
        } else {
            if let Some(p) = self.portfolio.positions.get_mut(&decision.ticker) {
                p.quantity -= quantity;
            }
        }
        
        // Add cash
        self.portfolio.cash += sell_value - self.commission_per_trade;
        
        // Record trade
        self.portfolio.trade_history.push(ExecutedTrade {
            timestamp,
            ticker: decision.ticker.clone(),
            action: Action::Sell,
            quantity,
            price,
            commission: self.commission_per_trade,
            realized_pnl: Some(net_pnl),
        });
        
        let holding_days = (timestamp - position.entry_date).num_days() as u32;
        
        TradeOutcome {
            profit_loss: net_pnl,
            profit_loss_pct: pnl_pct,
            holding_period_days: holding_days,
            max_favorable_excursion: gross_pnl.max(Decimal::ZERO),
            max_adverse_excursion: (-gross_pnl).max(Decimal::ZERO),
            exit_reason: ExitReason::Manual,
            success: net_pnl > Decimal::ZERO,
        }
    }
    
    /// Apply slippage to price
    fn apply_slippage(&self, price: Decimal, action: &Action) -> Decimal {
        match action {
            Action::Buy => price * (Decimal::ONE + self.slippage_pct),
            Action::Sell => price * (Decimal::ONE - self.slippage_pct),
            Action::Hold => price,
        }
    }
    
    /// Empty outcome for hold/no action
    fn empty_outcome(&self) -> TradeOutcome {
        TradeOutcome {
            profit_loss: Decimal::ZERO,
            profit_loss_pct: 0.0,
            holding_period_days: 0,
            max_favorable_excursion: Decimal::ZERO,
            max_adverse_excursion: Decimal::ZERO,
            exit_reason: ExitReason::Manual,
            success: false,
        }
    }
    
    /// Hold action outcome
    fn hold_outcome(&self, data: &MarketDataPoint) -> TradeOutcome {
        TradeOutcome {
            profit_loss: Decimal::ZERO,
            profit_loss_pct: 0.0,
            holding_period_days: 0,
            max_favorable_excursion: Decimal::ZERO,
            max_adverse_excursion: Decimal::ZERO,
            exit_reason: ExitReason::Manual,
            success: false,
        }
    }
    
    /// Step to next day
    pub fn step(&mut self) -> bool {
        if self.current_index + 1 < self.market_data.len() {
            self.current_index += 1;
            self.update_positions();
            self.record_equity();
            true
        } else {
            false
        }
    }
    
    /// Update unrealized P&L for all positions
    fn update_positions(&mut self) {
        let current_data = match self.current_data() {
            Some(d) => d.clone(),
            None => return,
        };
        
        for (_, position) in self.portfolio.positions.iter_mut() {
            let current_value = current_data.close * Decimal::from(position.quantity);
            let cost_basis = position.avg_entry_price * Decimal::from(position.quantity);
            position.unrealized_pnl = current_value - cost_basis;
        }
    }
    
    /// Record equity point
    fn record_equity(&mut self) {
        let current_data = match self.current_data() {
            Some(d) => d,
            None => return,
        };
        
        let positions_value: Decimal = self.portfolio.positions
            .values()
            .map(|p| current_data.close * Decimal::from(p.quantity))
            .sum();
        
        let total_value = self.portfolio.cash + positions_value;
        
        self.portfolio.equity_curve.push(EquityPoint {
            timestamp: current_data.timestamp,
            total_value,
            cash: self.portfolio.cash,
            positions_value,
        });
    }
    
    /// Get current portfolio value
    pub fn portfolio_value(&self) -> Decimal {
        let current_data = match self.current_data() {
            Some(d) => d,
            None => return self.portfolio.cash,
        };
        
        let positions_value: Decimal = self.portfolio.positions
            .values()
            .map(|p| current_data.close * Decimal::from(p.quantity))
            .sum();
        
        self.portfolio.cash + positions_value
    }
    
    /// Calculate current epoch metrics
    pub fn calculate_metrics(&self) -> EpochMetrics {
        let final_value = self.portfolio_value();
        let start_value = self.portfolio.start_value;
        let total_return = final_value - start_value;
        let return_pct = (total_return / start_value).to_f64().unwrap_or(0.0);
        
        // Calculate CAGR (assuming 252 trading days per year)
        let trading_days = self.portfolio.equity_curve.len() as f64;
        let years = trading_days / 252.0;
        let cagr = if years > 0.0 {
            ((final_value / start_value).to_f64().unwrap_or(1.0)).powf(1.0 / years) - 1.0
        } else {
            0.0
        };
        
        // Calculate max drawdown
        let max_drawdown = self.calculate_max_drawdown();
        
        // Calculate win rate from realized trades
        let realized_trades: Vec<_> = self.portfolio.trade_history
            .iter()
            .filter(|t| t.realized_pnl.is_some())
            .collect();
        
        let winning_trades = realized_trades
            .iter()
            .filter(|t| t.realized_pnl.unwrap_or(Decimal::ZERO) > Decimal::ZERO)
            .count();
        
        let win_rate = if !realized_trades.is_empty() {
            winning_trades as f64 / realized_trades.len() as f64
        } else {
            0.0
        };
        
        // Calculate Sharpe ratio (simplified, assuming risk-free rate = 0)
        let sharpe = if self.portfolio.equity_curve.len() > 1 {
            let returns: Vec<f64> = self.portfolio.equity_curve
                .windows(2)
                .map(|w| {
                    let prev = w[0].total_value;
                    let curr = w[1].total_value;
                    ((curr - prev) / prev).to_f64().unwrap_or(0.0)
                })
                .collect();
            
            let avg_return = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance = returns.iter().map(|r| (r - avg_return).powi(2)).sum::<f64>() / returns.len() as f64;
            let std_dev = variance.sqrt();
            
            if std_dev > 0.0 {
                avg_return / std_dev * (252.0_f64).sqrt() // Annualized
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        EpochMetrics {
            final_portfolio_value: final_value,
            total_return,
            cagr,
            sharpe_ratio: sharpe,
            max_drawdown,
            win_rate,
            profit_factor: 0.0, // Would need gross profit / gross loss
        }
    }
    
    /// Calculate maximum drawdown
    fn calculate_max_drawdown(&self) -> Decimal {
        let mut max_dd = Decimal::ZERO;
        let mut peak = Decimal::ZERO;
        
        for point in &self.portfolio.equity_curve {
            if point.total_value > peak {
                peak = point.total_value;
            }
            
            let dd = (peak - point.total_value) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
        
        -max_dd // Return as negative number
    }
    
    /// Get portfolio reference
    pub fn portfolio(&self) -> &VirtualPortfolio {
        &self.portfolio
    }
    
    /// Get mutable portfolio reference
    pub fn portfolio_mut(&mut self) -> &mut VirtualPortfolio {
        &mut self.portfolio
    }
}
