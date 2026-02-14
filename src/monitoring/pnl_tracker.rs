//! Real-time P&L Tracker
//!
//! Tracks profit and loss in real-time

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{info, trace};
use uuid::Uuid;

/// Real-time P&L tracker
#[derive(Debug)]
pub struct PnlTracker {
    positions: HashMap<String, PositionPnl>,
    daily_snapshots: HashMap<NaiveDate, DailyPnl>,
    trades: Vec<TradePnl>,
    total_realized: Decimal,
    total_unrealized: Decimal,
}

/// Position P&L
#[derive(Debug, Clone)]
pub struct PositionPnl {
    pub symbol: String,
    pub quantity: Decimal,
    pub avg_entry_price: Decimal,
    pub current_price: Decimal,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub total_pnl: Decimal,
    pub last_update: DateTime<Utc>,
}

/// Trade P&L
#[derive(Debug, Clone)]
pub struct TradePnl {
    pub id: Uuid,
    pub symbol: String,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    pub pnl: Decimal,
    pub fees: Decimal,
    pub net_pnl: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// Daily P&L summary
#[derive(Debug, Clone)]
pub struct DailyPnl {
    pub date: NaiveDate,
    pub realized_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub total_pnl: Decimal,
    pub trade_count: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
}

/// Real-time P&L summary
#[derive(Debug, Clone)]
pub struct RealTimePnl {
    pub total_realized: Decimal,
    pub total_unrealized: Decimal,
    pub total_pnl: Decimal,
    pub daily_pnl: Decimal,
    pub mtd_pnl: Decimal, // Month-to-date
    pub ytd_pnl: Decimal, // Year-to-date
    pub win_rate: f32,
    pub profit_factor: f32,
    pub sharpe_ratio: f32,
    pub max_drawdown: Decimal,
    pub position_count: usize,
    pub timestamp: DateTime<Utc>,
}

/// P&L snapshot
#[derive(Debug, Clone)]
pub struct PnlSnapshot {
    pub timestamp: DateTime<Utc>,
    pub positions: Vec<PositionPnl>,
    pub total_pnl: Decimal,
    pub daily_pnl: Decimal,
}

impl PnlTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
            daily_snapshots: HashMap::new(),
            trades: Vec::new(),
            total_realized: Decimal::ZERO,
            total_unrealized: Decimal::ZERO,
        }
    }
    
    /// Update position P&L
    pub fn update(&mut self, symbol: &str, realized: Decimal, unrealized: Decimal) {
        let position = self.positions.entry(symbol.to_string()).or_insert_with(|| {
            PositionPnl {
                symbol: symbol.to_string(),
                quantity: Decimal::ZERO,
                avg_entry_price: Decimal::ZERO,
                current_price: Decimal::ZERO,
                realized_pnl: Decimal::ZERO,
                unrealized_pnl: Decimal::ZERO,
                total_pnl: Decimal::ZERO,
                last_update: Utc::now(),
            }
        });
        
        position.realized_pnl = realized;
        position.unrealized_pnl = unrealized;
        position.total_pnl = realized + unrealized;
        position.last_update = Utc::now();
        
        self.recalculate_totals();
    }
    
    /// Update with position details
    pub fn update_position(
        &mut self,
        symbol: &str,
        quantity: Decimal,
        avg_entry: Decimal,
        current_price: Decimal,
    ) {
        let position = self.positions.entry(symbol.to_string()).or_insert_with(|| {
            PositionPnl {
                symbol: symbol.to_string(),
                quantity: Decimal::ZERO,
                avg_entry_price: Decimal::ZERO,
                current_price: Decimal::ZERO,
                realized_pnl: Decimal::ZERO,
                unrealized_pnl: Decimal::ZERO,
                total_pnl: Decimal::ZERO,
                last_update: Utc::now(),
            }
        });
        
        position.quantity = quantity;
        position.avg_entry_price = avg_entry;
        position.current_price = current_price;
        
        // Calculate unrealized P&L
        if quantity > Decimal::ZERO {
            position.unrealized_pnl = (current_price - avg_entry) * quantity;
        }
        
        position.total_pnl = position.realized_pnl + position.unrealized_pnl;
        position.last_update = Utc::now();
        
        self.recalculate_totals();
    }
    
    /// Record a closed trade
    pub fn record_trade(
        &mut self,
        symbol: &str,
        quantity: Decimal,
        entry_price: Decimal,
        exit_price: Decimal,
        fees: Decimal,
    ) -> TradePnl {
        let gross_pnl = (exit_price - entry_price) * quantity;
        let net_pnl = gross_pnl - fees;
        
        let trade = TradePnl {
            id: Uuid::new_v4(),
            symbol: symbol.to_string(),
            quantity,
            entry_price,
            exit_price,
            pnl: gross_pnl,
            fees,
            net_pnl,
            timestamp: Utc::now(),
        };
        
        self.trades.push(trade.clone());
        
        // Update daily snapshot
        self.update_daily_snapshot(net_pnl);
        
        info!(
            "Trade recorded: {} {} shares, P&L: {} (net: {})",
            symbol, quantity, gross_pnl, net_pnl
        );
        
        trade
    }
    
    /// Get total P&L
    pub fn total_pnl(&self) -> Decimal {
        self.total_realized + self.total_unrealized
    }
    
    /// Get daily P&L
    pub fn daily_pnl(&self) -> Decimal {
        let today = Utc::now().date_naive();
        self.daily_snapshots
            .get(&today)
            .map(|s| s.total_pnl)
            .unwrap_or(Decimal::ZERO)
    }
    
    /// Get month-to-date P&L
    pub fn mtd_pnl(&self) -> Decimal {
        let today = Utc::now().date_naive();
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        
        self.daily_snapshots
            .iter()
            .filter(|(date, _)| **date >= month_start && **date <= today)
            .map(|(_, snapshot)| snapshot.total_pnl)
            .sum()
    }
    
    /// Get year-to-date P&L
    pub fn ytd_pnl(&self) -> Decimal {
        let today = Utc::now().date_naive();
        let year_start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
        
        self.daily_snapshots
            .iter()
            .filter(|(date, _)| **date >= year_start && **date <= today)
            .map(|(_, snapshot)| snapshot.total_pnl)
            .sum()
    }
    
    /// Get position P&L
    pub fn get_position(&self, symbol: &str) -> Option<&PositionPnl> {
        self.positions.get(symbol)
    }
    
    /// Get all positions
    pub fn get_all_positions(&self) -> Vec<&PositionPnl> {
        self.positions.values().collect()
    }
    
    /// Get summary
    pub fn get_summary(&self) -> RealTimePnl {
        let today_pnl = self.daily_pnl();
        let mtd = self.mtd_pnl();
        let ytd = self.ytd_pnl();
        
        let today = Utc::now().date_naive();
        let today_snapshot = self.daily_snapshots.get(&today);
        
        let win_rate = if !self.trades.is_empty() {
            let wins = self.trades.iter().filter(|t| t.net_pnl > Decimal::ZERO).count();
            wins as f32 / self.trades.len() as f32
        } else {
            0.0
        };
        
        let profit_factor = self.calculate_profit_factor();
        
        RealTimePnl {
            total_realized: self.total_realized,
            total_unrealized: self.total_unrealized,
            total_pnl: self.total_pnl(),
            daily_pnl: today_pnl,
            mtd_pnl: mtd,
            ytd_pnl: ytd,
            win_rate,
            profit_factor,
            sharpe_ratio: 0.0, // Would calculate from returns
            max_drawdown: Decimal::ZERO, // Would track
            position_count: self.positions.len(),
            timestamp: Utc::now(),
        }
    }
    
    /// Get snapshot
    pub fn get_snapshot(&self) -> PnlSnapshot {
        let positions: Vec<PositionPnl> = self.positions.values().cloned().collect();
        
        PnlSnapshot {
            timestamp: Utc::now(),
            positions,
            total_pnl: self.total_pnl(),
            daily_pnl: self.daily_pnl(),
        }
    }
    
    /// Get trades for date range
    pub fn get_trades_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&TradePnl> {
        self.trades.iter()
            .filter(|t| t.timestamp >= start && t.timestamp <= end)
            .collect()
    }
    
    /// Get daily history
    pub fn get_daily_history(&self, days: i64) -> Vec<&DailyPnl> {
        let cutoff = Utc::now().date_naive() - Duration::days(days);
        
        let mut history: Vec<&DailyPnl> = self.daily_snapshots
            .values()
            .filter(|s| s.date >= cutoff)
            .collect();
        
        history.sort_by(|a, b| a.date.cmp(&b.date));
        history
    }
    
    /// Recalculate totals
    fn recalculate_totals(&mut self) {
        self.total_realized = self.positions.values()
            .map(|p| p.realized_pnl)
            .sum();
        
        self.total_unrealized = self.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum();
    }
    
    /// Update daily snapshot
    fn update_daily_snapshot(&mut self, trade_pnl: Decimal) {
        let today = Utc::now().date_naive();
        
        let snapshot = self.daily_snapshots.entry(today).or_insert_with(|| {
            DailyPnl {
                date: today,
                realized_pnl: Decimal::ZERO,
                unrealized_pnl: Decimal::ZERO,
                total_pnl: Decimal::ZERO,
                trade_count: 0,
                winning_trades: 0,
                losing_trades: 0,
            }
        });
        
        snapshot.realized_pnl += trade_pnl;
        snapshot.total_pnl += trade_pnl;
        snapshot.trade_count += 1;
        
        if trade_pnl > Decimal::ZERO {
            snapshot.winning_trades += 1;
        } else {
            snapshot.losing_trades += 1;
        }
    }
    
    /// Calculate profit factor
    fn calculate_profit_factor(&self) -> f32 {
        let gross_profit: Decimal = self.trades.iter()
            .filter(|t| t.net_pnl > Decimal::ZERO)
            .map(|t| t.net_pnl)
            .sum();
        
        let gross_loss: Decimal = self.trades.iter()
            .filter(|t| t.net_pnl < Decimal::ZERO)
            .map(|t| t.net_pnl.abs())
            .sum();
        
        if gross_loss > Decimal::ZERO {
            let pf: f32 = (gross_profit / gross_loss).try_into().unwrap_or(0.0);
            pf
        } else if gross_profit > Decimal::ZERO {
            f32::INFINITY
        } else {
            0.0
        }
    }
    
    /// Get best performing position
    pub fn best_position(&self) -> Option<&PositionPnl> {
        self.positions.values()
            .max_by(|a, b| a.total_pnl.cmp(&b.total_pnl))
    }
    
    /// Get worst performing position
    pub fn worst_position(&self) -> Option<&PositionPnl> {
        self.positions.values()
            .min_by(|a, b| a.total_pnl.cmp(&b.total_pnl))
    }
    
    /// Clear old data
    pub fn cleanup(&mut self, days: i64) {
        let cutoff = Utc::now().date_naive() - Duration::days(days);
        
        self.daily_snapshots.retain(|date, _| *date > cutoff);
        
        let cutoff_datetime = Utc::now() - Duration::days(days);
        self.trades.retain(|t| t.timestamp > cutoff_datetime);
    }
    
    /// Get trade count
    pub fn trade_count(&self) -> usize {
        self.trades.len()
    }
}

impl Default for PnlTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let tracker = PnlTracker::new();
        assert_eq!(tracker.total_pnl(), Decimal::ZERO);
        assert!(tracker.get_all_positions().is_empty());
    }

    #[test]
    fn test_update_pnl() {
        let mut tracker = PnlTracker::new();
        
        tracker.update("AAPL", Decimal::from(100), Decimal::from(50));
        
        assert_eq!(tracker.total_realized, Decimal::from(100));
        assert_eq!(tracker.total_unrealized, Decimal::from(50));
        assert_eq!(tracker.total_pnl(), Decimal::from(150));
    }

    #[test]
    fn test_update_position() {
        let mut tracker = PnlTracker::new();
        
        tracker.update_position("AAPL", Decimal::from(100), Decimal::from(150), Decimal::from(160));
        
        let position = tracker.get_position("AAPL").unwrap();
        assert_eq!(position.quantity, Decimal::from(100));
        assert_eq!(position.unrealized_pnl, Decimal::from(1000)); // (160-150)*100
    }

    #[test]
    fn test_record_trade() {
        let mut tracker = PnlTracker::new();
        
        let trade = tracker.record_trade(
            "AAPL",
            Decimal::from(100),
            Decimal::from(150),
            Decimal::from(160),
            Decimal::from(10),
        );
        
        assert_eq!(trade.net_pnl, Decimal::from(990)); // (160-150)*100 - 10
        assert_eq!(tracker.trade_count(), 1);
        assert!(tracker.daily_pnl() != Decimal::ZERO);
    }

    #[test]
    fn test_daily_pnl() {
        let mut tracker = PnlTracker::new();
        
        tracker.record_trade("AAPL", Decimal::from(10), Decimal::from(100), Decimal::from(110), Decimal::from(1));
        tracker.record_trade("MSFT", Decimal::from(5), Decimal::from(200), Decimal::from(210), Decimal::from(1));
        
        let daily = tracker.daily_pnl();
        assert!(daily > Decimal::ZERO);
    }

    #[test]
    fn test_get_summary() {
        let mut tracker = PnlTracker::new();
        
        tracker.update("AAPL", Decimal::from(500), Decimal::from(200));
        tracker.record_trade("MSFT", Decimal::from(10), Decimal::from(100), Decimal::from(110), Decimal::from(1));
        
        let summary = tracker.get_summary();
        
        assert_eq!(summary.total_realized, Decimal::from(500));
        assert_eq!(summary.position_count, 1);
        assert!(summary.total_pnl > Decimal::ZERO);
    }

    #[test]
    fn test_best_worst_positions() {
        let mut tracker = PnlTracker::new();
        
        tracker.update("WINNER", Decimal::from(1000), Decimal::ZERO);
        tracker.update("LOSER", Decimal::from(-500), Decimal::ZERO);
        tracker.update("NEUTRAL", Decimal::ZERO, Decimal::ZERO);
        
        let best = tracker.best_position().unwrap();
        assert_eq!(best.symbol, "WINNER");
        
        let worst = tracker.worst_position().unwrap();
        assert_eq!(worst.symbol, "LOSER");
    }

    #[test]
    fn test_profit_factor() {
        let mut tracker = PnlTracker::new();
        
        // Winning trades
        tracker.record_trade("A", Decimal::from(10), Decimal::from(100), Decimal::from(110), Decimal::ZERO);
        tracker.record_trade("B", Decimal::from(10), Decimal::from(100), Decimal::from(110), Decimal::ZERO);
        
        // Losing trade
        tracker.record_trade("C", Decimal::from(10), Decimal::from(100), Decimal::from(95), Decimal::ZERO);
        
        let summary = tracker.get_summary();
        assert!(summary.profit_factor > 1.0);
    }

    #[test]
    fn test_win_rate() {
        let mut tracker = PnlTracker::new();
        
        // 2 wins, 1 loss
        tracker.record_trade("A", Decimal::from(10), Decimal::from(100), Decimal::from(110), Decimal::ZERO);
        tracker.record_trade("B", Decimal::from(10), Decimal::from(100), Decimal::from(110), Decimal::ZERO);
        tracker.record_trade("C", Decimal::from(10), Decimal::from(100), Decimal::from(90), Decimal::ZERO);
        
        let summary = tracker.get_summary();
        assert!((summary.win_rate - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_snapshot() {
        let mut tracker = PnlTracker::new();
        
        tracker.update("AAPL", Decimal::from(100), Decimal::from(50));
        tracker.update("MSFT", Decimal::from(200), Decimal::from(100));
        
        let snapshot = tracker.get_snapshot();
        
        assert_eq!(snapshot.positions.len(), 2);
        assert_eq!(snapshot.total_pnl, Decimal::from(450));
    }
}
