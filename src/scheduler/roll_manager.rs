//! Futures and Options Roll Manager
//!
//! Manages contract rolls for continuous trading

use chrono::{DateTime, Datelike, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{info, warn};

/// Roll manager for futures/options contracts
#[derive(Debug)]
pub struct RollManager {
    rolls: HashMap<String, RollConfig>,
    executed_rolls: Vec<ExecutedRoll>,
}

/// Roll configuration
#[derive(Debug, Clone)]
pub struct RollConfig {
    pub id: String,
    pub symbol: String,
    pub from_contract: String,
    pub to_contract: String,
    pub roll_type: RollType,
    pub target_date: DateTime<Utc>,
    pub status: RollStatus,
    pub quantity: Decimal,
}

/// Type of roll
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollType {
    FuturesMonthly,
    FuturesQuarterly,
    OptionsMonthly,
    OptionsWeekly,
    Custom,
}

impl RollType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RollType::FuturesMonthly => "Futures Monthly",
            RollType::FuturesQuarterly => "Futures Quarterly",
            RollType::OptionsMonthly => "Options Monthly",
            RollType::OptionsWeekly => "Options Weekly",
            RollType::Custom => "Custom",
        }
    }
}

/// Roll status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollStatus {
    Scheduled,
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Executed roll record
#[derive(Debug, Clone)]
pub struct ExecutedRoll {
    pub config: RollConfig,
    pub executed_at: DateTime<Utc>,
    pub execution_price_from: Decimal,
    pub execution_price_to: Decimal,
    pub slippage: Decimal,
}

/// Roll calendar for standard contracts
#[derive(Debug, Clone)]
pub struct RollCalendar {
    pub symbol: String,
    pub contract_months: Vec<u32>,
    pub roll_date_offset: i64, // Days before expiry to roll
    pub last_roll: Option<DateTime<Utc>>,
}

impl RollManager {
    /// Create new roll manager
    pub fn new() -> Self {
        Self {
            rolls: HashMap::new(),
            executed_rolls: Vec::new(),
        }
    }

    /// Add a roll configuration
    pub fn add_roll(&mut self, config: RollConfig) {
        self.rolls.insert(config.id.clone(), config);
    }

    /// Get roll by ID
    pub fn get_roll(&self, id: &str) -> Option<&RollConfig> {
        self.rolls.get(id)
    }

    /// Get upcoming rolls within timeframe
    pub fn get_upcoming_rolls(&self, within: Duration) -> Vec<RollConfig> {
        let now = Utc::now();
        self.rolls.values()
            .filter(|r| r.target_date > now && r.target_date <= now + within)
            .filter(|r| r.status == RollStatus::Scheduled)
            .cloned()
            .collect()
    }

    /// Execute pending rolls
    pub async fn execute_pending(&mut self) -> Vec<super::Result<()>> {
        let now = Utc::now();
        let pending: Vec<String> = self.rolls.iter()
            .filter(|(_, r)| r.target_date <= now && r.status == RollStatus::Scheduled)
            .map(|(id, _)| id.clone())
            .collect();

        let mut results = Vec::new();
        
        for id in pending {
            results.push(self.execute_roll(&id).await);
        }

        results
    }

    /// Execute a specific roll
    async fn execute_roll(&mut self, id: &str) -> super::Result<()> {
        let config = self.rolls.get_mut(id)
            .ok_or_else(|| super::SchedulerError::RollFailed(
                format!("Roll {} not found", id)
            ))?;

        info!(
            "Executing {} roll for {}: {} -> {}",
            config.roll_type.as_str(),
            config.symbol,
            config.from_contract,
            config.to_contract
        );

        config.status = RollStatus::InProgress;

        // Simulated execution
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Record execution
        let executed = ExecutedRoll {
            config: config.clone(),
            executed_at: Utc::now(),
            execution_price_from: Decimal::from(100), // Would be actual price
            execution_price_to: Decimal::from(100),
            slippage: Decimal::ZERO,
        };

        self.executed_rolls.push(executed);
        config.status = RollStatus::Completed;

        info!("Roll {} completed successfully", id);
        Ok(())
    }

    /// Cancel a scheduled roll
    pub fn cancel_roll(&mut self, id: &str) -> super::Result<()> {
        let config = self.rolls.get_mut(id)
            .ok_or_else(|| super::SchedulerError::RollFailed(
                format!("Roll {} not found", id)
            ))?;

        if config.status != RollStatus::Scheduled {
            return Err(super::SchedulerError::RollFailed(
                "Can only cancel scheduled rolls".to_string()
            ));
        }

        config.status = RollStatus::Cancelled;
        info!("Roll {} cancelled", id);
        Ok(())
    }

    /// Get time until next roll
    pub fn time_until_next(&self) -> Option<Duration> {
        let now = Utc::now();
        self.rolls.values()
            .filter(|r| r.target_date > now && r.status == RollStatus::Scheduled)
            .map(|r| r.target_date - now)
            .min()
    }

    /// Get next roll time
    pub fn next_roll_time(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        self.rolls.values()
            .filter(|r| r.target_date > now && r.status == RollStatus::Scheduled)
            .map(|r| r.target_date)
            .min()
    }

    /// Count of pending rolls
    pub fn pending_count(&self) -> usize {
        self.rolls.values()
            .filter(|r| r.status == RollStatus::Scheduled)
            .count()
    }

    /// Get executed roll history
    pub fn get_history(&self) -> &[ExecutedRoll] {
        &self.executed_rolls
    }

    /// Create auto-roll calendar for symbol
    pub fn create_auto_calendar(
        &mut self,
        symbol: &str,
        roll_type: RollType,
        months_ahead: i64,
    ) -> Vec<RollConfig> {
        let mut configs = Vec::new();
        let now = Utc::now();

        for month in 1..=months_ahead {
            let roll_date = now + Duration::days(month * 30);
            let _id = format!("{}-{}", symbol, roll_date.format("%Y%m"));
            
            let (from, to) = match roll_type {
                RollType::FuturesMonthly | RollType::FuturesQuarterly => {
                    (format!("{}-M{}", symbol, month), format!("{}-M{}", symbol, month + 1))
                }
                _ => (format!("{}-C{}", symbol, month), format!("{}-C{}", symbol, month + 1))
            };

            let config = RollConfig::new(
                symbol.to_string(),
                from,
                to,
                roll_type,
                roll_date,
            );

            self.add_roll(config.clone());
            configs.push(config);
        }

        configs
    }
}

impl Default for RollManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RollConfig {
    /// Create new roll configuration
    pub fn new(
        symbol: String,
        from_contract: String,
        to_contract: String,
        roll_type: RollType,
        target_date: DateTime<Utc>,
    ) -> Self {
        Self {
            id: format!("{}-{}-to-{}", symbol, from_contract, to_contract),
            symbol,
            from_contract,
            to_contract,
            roll_type,
            target_date,
            status: RollStatus::Scheduled,
            quantity: Decimal::ZERO,
        }
    }

    /// Set quantity
    pub fn with_quantity(mut self, quantity: Decimal) -> Self {
        self.quantity = quantity;
        self
    }
}

impl RollCalendar {
    /// Create new roll calendar
    pub fn new(symbol: &str, contract_months: Vec<u32>, roll_date_offset: i64) -> Self {
        Self {
            symbol: symbol.to_string(),
            contract_months,
            roll_date_offset,
            last_roll: None,
        }
    }

    /// Check if roll needed
    pub fn is_roll_needed(&self, current_date: DateTime<Utc>) -> bool {
        if let Some(last) = self.last_roll {
            // Roll if enough time has passed
            current_date > last + Duration::days(25)
        } else {
            true // Never rolled, need to roll
        }
    }

    /// Get next roll date
    pub fn next_roll_date(&self, from: DateTime<Utc>) -> DateTime<Utc> {
        // Find next contract month
        let current_month = from.date_naive().month();
        let next_month = self.contract_months.iter()
            .find(|&&m| m > current_month)
            .copied()
            .unwrap_or_else(|| self.contract_months[0]);

        // Roll date is offset days before month end
        from + Duration::days(30) - Duration::days(self.roll_date_offset)
    }

    /// Standard monthly futures calendar
    pub fn monthly_futures(symbol: &str) -> Self {
        Self::new(symbol, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 5)
    }

    /// Quarterly futures calendar (Mar, Jun, Sep, Dec)
    pub fn quarterly_futures(symbol: &str) -> Self {
        Self::new(symbol, vec![3, 6, 9, 12], 7)
    }

    /// Monthly options calendar
    pub fn monthly_options(symbol: &str) -> Self {
        Self::new(symbol, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll_manager_creation() {
        let manager = RollManager::new();
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_add_roll() {
        let mut manager = RollManager::new();
        
        let config = RollConfig::new(
            "ES".to_string(),
            "ES-M1".to_string(),
            "ES-M2".to_string(),
            RollType::FuturesMonthly,
            Utc::now() + Duration::days(30),
        );

        manager.add_roll(config);
        assert_eq!(manager.pending_count(), 1);
    }

    #[test]
    fn test_get_upcoming_rolls() {
        let mut manager = RollManager::new();
        
        let config = RollConfig::new(
            "ES".to_string(),
            "ES-M1".to_string(),
            "ES-M2".to_string(),
            RollType::FuturesMonthly,
            Utc::now() + Duration::days(7),
        );

        manager.add_roll(config);
        
        let upcoming = manager.get_upcoming_rolls(Duration::days(14));
        assert_eq!(upcoming.len(), 1);
    }

    #[test]
    fn test_roll_calendar() {
        let calendar = RollCalendar::monthly_futures("CL");
        
        assert_eq!(calendar.symbol, "CL");
        assert_eq!(calendar.contract_months.len(), 12);
    }

    #[test]
    fn test_quarterly_calendar() {
        let calendar = RollCalendar::quarterly_futures("ES");
        
        assert_eq!(calendar.contract_months, vec![3, 6, 9, 12]);
    }

    #[test]
    fn test_cancel_roll() {
        let mut manager = RollManager::new();
        
        let config = RollConfig::new(
            "ES".to_string(),
            "ES-M1".to_string(),
            "ES-M2".to_string(),
            RollType::FuturesMonthly,
            Utc::now() + Duration::days(7),
        );

        manager.add_roll(config);
        manager.cancel_roll("ES-ES-M1-to-ES-M2").unwrap();
        
        let roll = manager.get_roll("ES-ES-M1-to-ES-M2").unwrap();
        assert_eq!(roll.status, RollStatus::Cancelled);
    }
}
