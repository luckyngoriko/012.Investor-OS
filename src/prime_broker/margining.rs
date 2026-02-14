//! Cross-Margining Engine
//!
//! Manages margin across multiple prime brokers for capital efficiency

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::{PrimeBroker, PrimeBrokerId, PrimeBrokerError, Result};

/// Cross-margining engine
#[derive(Debug)]
pub struct CrossMarginEngine {
    accounts: HashMap<PrimeBrokerId, MarginAccount>,
    cross_margin_enabled: bool,
    net_margin_requirement: Decimal,
}

/// Margin account at a prime broker
#[derive(Debug, Clone)]
pub struct MarginAccount {
    pub broker_id: PrimeBrokerId,
    pub cash_balance: Decimal,
    pub long_market_value: Decimal,
    pub short_market_value: Decimal,
    pub equity: Decimal,
    pub maintenance_margin: Decimal,
    pub initial_margin: Decimal,
    pub reg_t_margin: Decimal,
    pub positions: Vec<Position>,
    pub updated_at: DateTime<Utc>,
}

/// Position in an account
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub avg_price: Decimal,
    pub market_price: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub broker_id: PrimeBrokerId,
}

/// Margin requirement calculation
#[derive(Debug, Clone)]
pub struct MarginRequirement {
    pub initial_margin: Decimal,
    pub maintenance_margin: Decimal,
    pub reg_t_margin: Decimal,
}

impl CrossMarginEngine {
    /// Create new engine
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            cross_margin_enabled: true,
            net_margin_requirement: Decimal::ZERO,
        }
    }

    /// Add margin account
    pub fn add_account(&mut self, account: MarginAccount) {
        info!("Adding margin account for {:?}", account.broker_id);
        self.accounts.insert(account.broker_id, account);
        self.recalculate_net_margin();
    }

    /// Get account by broker
    pub fn get_account(&self, broker_id: PrimeBrokerId) -> Option<&MarginAccount> {
        self.accounts.get(&broker_id)
    }

    /// Update account
    pub fn update_account(&mut self, broker_id: PrimeBrokerId, f: impl FnOnce(&mut MarginAccount)) {
        if let Some(account) = self.accounts.get_mut(&broker_id) {
            f(account);
            account.updated_at = Utc::now();
            self.recalculate_net_margin();
        }
    }

    /// Transfer margin between brokers
    pub fn transfer_margin(
        &mut self,
        from: PrimeBrokerId,
        to: PrimeBrokerId,
        amount: Decimal,
    ) -> Result<()> {
        let from_account = self.accounts.get_mut(&from)
            .ok_or_else(|| PrimeBrokerError::BrokerNotFound(from.as_str().to_string()))?;
        
        if from_account.cash_balance < amount {
            return Err(PrimeBrokerError::InsufficientMargin(
                format!("Insufficient cash in {:?} account", from)
            ));
        }

        from_account.cash_balance -= amount;
        from_account.equity -= amount;

        let to_account = self.accounts.get_mut(&to)
            .ok_or_else(|| PrimeBrokerError::BrokerNotFound(to.as_str().to_string()))?;
        
        to_account.cash_balance += amount;
        to_account.equity += amount;

        info!("Transferred {} margin from {:?} to {:?}", amount, from, to);
        
        self.recalculate_net_margin();
        Ok(())
    }

    /// Calculate cross-margin benefit
    pub fn calculate_benefit(
        &self,
        _brokers: &HashMap<PrimeBrokerId, Box<dyn PrimeBroker>>,
    ) -> super::CrossMarginBenefit {
        // Calculate standalone margin (sum of individual requirements)
        let standalone_margin: Decimal = self.accounts.values()
            .map(|a| a.maintenance_margin)
            .sum();

        // Calculate cross-margined requirement (net positions)
        let net_long: Decimal = self.accounts.values()
            .map(|a| a.long_market_value)
            .sum();
        
        let net_short: Decimal = self.accounts.values()
            .map(|a| a.short_market_value)
            .sum();

        // Cross-margining allows offsetting longs and shorts
        let gross_exposure = net_long + net_short;
        let net_exposure = (net_long - net_short).abs();
        
        // Benefit from hedging
        let hedge_benefit = if gross_exposure > Decimal::ZERO {
            (gross_exposure - net_exposure) / gross_exposure * Decimal::from(50) // 50% offset
        } else {
            Decimal::ZERO
        };

        let cross_margined = standalone_margin * (Decimal::from(100) - hedge_benefit) / Decimal::from(100);

        let benefit = standalone_margin - cross_margined;
        let benefit_pct = if standalone_margin > Decimal::ZERO {
            benefit / standalone_margin * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        super::CrossMarginBenefit {
            standalone_margin_required: standalone_margin,
            cross_margined_requirement: cross_margined,
            benefit_amount: benefit,
            benefit_percentage: benefit_pct,
        }
    }

    /// Generate consolidated margin report
    pub fn generate_report(
        &self,
        _brokers: &HashMap<PrimeBrokerId, Box<dyn PrimeBroker>>,
    ) -> super::ConsolidatedMarginReport {
        let total_equity: Decimal = self.accounts.values().map(|a| a.equity).sum();
        let total_margin_used: Decimal = self.accounts.values().map(|a| a.initial_margin).sum();
        let total_margin_avail: Decimal = self.accounts.values()
            .map(|a| a.equity - a.maintenance_margin)
            .sum();
        let total_maintenance: Decimal = self.accounts.values().map(|a| a.maintenance_margin).sum();

        let utilization = if total_equity > Decimal::ZERO {
            total_margin_used / total_equity * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let broker_breakdown: HashMap<PrimeBrokerId, super::BrokerMarginSummary> = self.accounts
            .iter()
            .map(|(id, account)| (*id, super::BrokerMarginSummary {
                broker_id: *id,
                equity: account.equity,
                margin_used: account.initial_margin,
                margin_available: account.equity - account.maintenance_margin,
                financing_rate: Decimal::try_from(0.015).unwrap(), // Would be actual rate
                positions_count: account.positions.len(),
            }))
            .collect();

        super::ConsolidatedMarginReport {
            total_equity,
            total_margin_used: total_margin_used,
            total_margin_available: total_margin_avail,
            margin_utilization_pct: utilization,
            maintenance_margin: total_maintenance,
            excess_liquidity: total_equity - total_maintenance,
            broker_breakdown,
            timestamp: Utc::now(),
        }
    }

    /// Check if portfolio is in margin call
    pub fn is_margin_call(&self) -> bool {
        for account in self.accounts.values() {
            if account.equity < account.maintenance_margin {
                return true;
            }
        }
        false
    }

    /// Get total buying power (with cross-margin)
    pub fn total_buying_power(&self) -> Decimal {
        if self.cross_margin_enabled {
            // Net across all accounts
            let total_equity: Decimal = self.accounts.values().map(|a| a.equity).sum();
            let total_margin: Decimal = self.accounts.values().map(|a| a.initial_margin).sum();
            (total_equity - total_margin) * Decimal::from(2) // 2:1 leverage
        } else {
            // Sum individual buying powers
            self.accounts.values()
                .map(|a| (a.equity - a.initial_margin) * Decimal::from(2))
                .sum()
        }
    }

    /// Get all positions across all accounts
    pub fn get_all_positions(&self) -> Vec<Position> {
        self.accounts.values()
            .flat_map(|a| a.positions.clone())
            .collect()
    }

    /// Enable/disable cross-margining
    pub fn set_cross_margin(&mut self, enabled: bool) {
        self.cross_margin_enabled = enabled;
        info!("Cross-margining {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Recalculate net margin requirements
    fn recalculate_net_margin(&mut self) {
        // Calculate net position exposure
        let mut net_exposure = Decimal::ZERO;
        
        for account in self.accounts.values() {
            net_exposure += account.long_market_value;
            net_exposure -= account.short_market_value;
        }

        // Simplified: 25% margin requirement on net exposure
        self.net_margin_requirement = net_exposure.abs() * Decimal::try_from(0.25).unwrap();
    }

    /// Get account count
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Check if has accounts
    pub fn has_accounts(&self) -> bool {
        !self.accounts.is_empty()
    }
}

impl Default for CrossMarginEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MarginAccount {
    /// Create new margin account
    pub fn new(broker_id: PrimeBrokerId, initial_cash: Decimal) -> Self {
        Self {
            broker_id,
            cash_balance: initial_cash,
            long_market_value: Decimal::ZERO,
            short_market_value: Decimal::ZERO,
            equity: initial_cash,
            maintenance_margin: Decimal::ZERO,
            initial_margin: Decimal::ZERO,
            reg_t_margin: Decimal::ZERO,
            positions: Vec::new(),
            updated_at: Utc::now(),
        }
    }

    /// Add position
    pub fn add_position(&mut self, position: Position) {
        self.positions.push(position);
        self.recalculate();
    }

    /// Recalculate account values
    pub fn recalculate(&mut self) {
        self.long_market_value = self.positions.iter()
            .filter(|p| p.quantity > Decimal::ZERO)
            .map(|p| p.market_value)
            .sum();
        
        self.short_market_value = self.positions.iter()
            .filter(|p| p.quantity < Decimal::ZERO)
            .map(|p| p.market_value.abs())
            .sum();

        self.equity = self.cash_balance + self.long_market_value - self.short_market_value;

        // Simplified margin calculations
        self.maintenance_margin = (self.long_market_value + self.short_market_value) 
            * Decimal::try_from(0.25).unwrap();
        self.initial_margin = self.maintenance_margin * Decimal::try_from(1.2).unwrap();
        self.reg_t_margin = (self.long_market_value + self.short_market_value) 
            * Decimal::from(2); // 50% requirement
    }

    /// Get excess margin
    pub fn excess_margin(&self) -> Decimal {
        self.equity - self.maintenance_margin
    }

    /// Check if in margin call
    pub fn in_margin_call(&self) -> bool {
        self.equity < self.maintenance_margin
    }
}

/// Margin utilization by symbol
#[derive(Debug, Clone)]
pub struct SymbolMarginUtilization {
    pub symbol: String,
    pub total_long_value: Decimal,
    pub total_short_value: Decimal,
    pub net_margin_required: Decimal,
    pub cross_margin_benefit: Decimal,
}

impl CrossMarginEngine {
    /// Get margin utilization by symbol (for reporting)
    pub fn get_symbol_utilization(&self) -> Vec<SymbolMarginUtilization> {
        let mut symbol_map: HashMap<String, (Decimal, Decimal)> = HashMap::new();

        // Aggregate positions across accounts
        for account in self.accounts.values() {
            for position in &account.positions {
                let entry = symbol_map.entry(position.symbol.clone()).or_insert((Decimal::ZERO, Decimal::ZERO));
                if position.quantity > Decimal::ZERO {
                    entry.0 += position.market_value;
                } else {
                    entry.1 += position.market_value.abs();
                }
            }
        }

        symbol_map.iter()
            .map(|(symbol, (long, short))| {
                let gross = long + short;
                let net = (long - short).abs();
                let benefit = gross - net;
                
                SymbolMarginUtilization {
                    symbol: symbol.clone(),
                    total_long_value: *long,
                    total_short_value: *short,
                    net_margin_required: net * Decimal::try_from(0.25).unwrap(),
                    cross_margin_benefit: benefit * Decimal::try_from(0.25).unwrap(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_margin_engine_creation() {
        let engine = CrossMarginEngine::new();
        assert!(!engine.has_accounts());
    }

    #[test]
    fn test_margin_account_creation() {
        let account = MarginAccount::new(PrimeBrokerId::GoldmanSachs, Decimal::from(1_000_000));
        
        assert_eq!(account.cash_balance, Decimal::from(1_000_000));
        assert_eq!(account.equity, Decimal::from(1_000_000));
        assert!(!account.in_margin_call());
    }

    #[test]
    fn test_add_position() {
        let mut account = MarginAccount::new(PrimeBrokerId::GoldmanSachs, Decimal::from(1_000_000));
        
        let position = Position {
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            avg_price: Decimal::from(150),
            market_price: Decimal::from(160),
            market_value: Decimal::from(16_000),
            unrealized_pnl: Decimal::from(1_000),
            broker_id: PrimeBrokerId::GoldmanSachs,
        };
        
        account.add_position(position);
        
        assert_eq!(account.long_market_value, Decimal::from(16_000));
        assert!(account.maintenance_margin > Decimal::ZERO);
    }

    #[test]
    fn test_margin_transfer() {
        let mut engine = CrossMarginEngine::new();
        
        engine.add_account(MarginAccount::new(PrimeBrokerId::GoldmanSachs, Decimal::from(1_000_000)));
        engine.add_account(MarginAccount::new(PrimeBrokerId::MorganStanley, Decimal::from(500_000)));
        
        engine.transfer_margin(
            PrimeBrokerId::GoldmanSachs,
            PrimeBrokerId::MorganStanley,
            Decimal::from(100_000),
        ).unwrap();
        
        let gs = engine.get_account(PrimeBrokerId::GoldmanSachs).unwrap();
        let ms = engine.get_account(PrimeBrokerId::MorganStanley).unwrap();
        
        assert_eq!(gs.cash_balance, Decimal::from(900_000));
        assert_eq!(ms.cash_balance, Decimal::from(600_000));
    }

    #[test]
    fn test_insufficient_margin() {
        let mut engine = CrossMarginEngine::new();
        
        engine.add_account(MarginAccount::new(PrimeBrokerId::GoldmanSachs, Decimal::from(100_000)));
        
        let result = engine.transfer_margin(
            PrimeBrokerId::GoldmanSachs,
            PrimeBrokerId::MorganStanley,
            Decimal::from(200_000),
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_buying_power() {
        let mut engine = CrossMarginEngine::new();
        
        engine.add_account(MarginAccount::new(PrimeBrokerId::GoldmanSachs, Decimal::from(1_000_000)));
        engine.add_account(MarginAccount::new(PrimeBrokerId::MorganStanley, Decimal::from(500_000)));
        
        let buying_power = engine.total_buying_power();
        assert!(buying_power > Decimal::ZERO);
    }
}
