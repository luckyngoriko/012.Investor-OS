//! Tax & Compliance Engine
//!
//! Sprint 30: Tax & Compliance Engine
//! - Tax loss harvesting optimization
//! - Wash sale rule monitoring
//! - Tax reporting and compliance

use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

pub mod harvesting;
pub mod reporting;
pub mod wash_sale;

pub use harvesting::{TaxLossHarvester, HarvestOpportunity, HarvestResult};
pub use reporting::{TaxReport, TaxReportingEngine, TaxForm, TaxYear};
pub use wash_sale::{WashSaleMonitor, WashSaleRule, WashSaleViolation};

/// Tax errors
#[derive(Error, Debug, Clone)]
pub enum TaxError {
    #[error("Invalid tax year: {0}")]
    InvalidTaxYear(String),
    
    #[error("Wash sale violation: {0}")]
    WashSaleViolation(String),
    
    #[error("Reporting error: {0}")]
    Reporting(String),
    
    #[error("Harvest failed: {0}")]
    HarvestFailed(String),
}

pub type Result<T> = std::result::Result<T, TaxError>;

/// Tax jurisdiction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxJurisdiction {
    USA,
    UK,
    EU,
    Singapore,
    HongKong,
    Other,
}

impl TaxJurisdiction {
    pub fn supports_loss_harvesting(&self) -> bool {
        matches!(self, TaxJurisdiction::USA)
    }
    
    pub fn tax_year_end(&self) -> (u32, u32) {
        // (month, day)
        match self {
            TaxJurisdiction::USA => (12, 31),
            TaxJurisdiction::UK => (4, 5),
            TaxJurisdiction::Singapore => (12, 31),
            TaxJurisdiction::HongKong => (3, 31),
            _ => (12, 31),
        }
    }
    
    pub fn short_term_period_days(&self) -> i64 {
        match self {
            TaxJurisdiction::USA => 365, // 1 year
            _ => 365,
        }
    }
}

/// Tax lot (individual purchase)
#[derive(Debug, Clone)]
pub struct TaxLot {
    pub id: Uuid,
    pub symbol: String,
    pub quantity: Decimal,
    pub purchase_date: DateTime<Utc>,
    pub purchase_price: Decimal,
    pub current_price: Option<Decimal>,
    pub fees: Decimal,
}

impl TaxLot {
    /// Calculate unrealized gain/loss
    pub fn unrealized_pnl(&self) -> Option<Decimal> {
        self.current_price.map(|price| {
            (price - self.purchase_price) * self.quantity - self.fees
        })
    }
    
    /// Check if short term
    pub fn is_short_term(&self, jurisdiction: TaxJurisdiction) -> bool {
        let holding_period = Utc::now() - self.purchase_date;
        holding_period.num_days() <= jurisdiction.short_term_period_days()
    }
    
    /// Calculate days held
    pub fn days_held(&self) -> i64 {
        (Utc::now() - self.purchase_date).num_days()
    }
}

/// Realized gain/loss from sale
#[derive(Debug, Clone)]
pub struct RealizedGain {
    pub lot_id: Uuid,
    pub symbol: String,
    pub quantity: Decimal,
    pub purchase_date: DateTime<Utc>,
    pub sale_date: DateTime<Utc>,
    pub cost_basis: Decimal,
    pub proceeds: Decimal,
    pub gain_loss: Decimal,
    pub is_short_term: bool,
    pub wash_sale_adjusted: bool,
    pub adjustment_amount: Decimal,
}

/// Tax engine coordinator
#[derive(Debug)]
pub struct TaxEngine {
    jurisdiction: TaxJurisdiction,
    lots: HashMap<Uuid, TaxLot>,
    realized_gains: Vec<RealizedGain>,
    harvester: TaxLossHarvester,
    wash_sale_monitor: WashSaleMonitor,
    reporting_engine: TaxReportingEngine,
}

impl TaxEngine {
    /// Create new tax engine
    pub fn new(jurisdiction: TaxJurisdiction) -> Self {
        Self {
            jurisdiction,
            lots: HashMap::new(),
            realized_gains: Vec::new(),
            harvester: TaxLossHarvester::new(jurisdiction),
            wash_sale_monitor: WashSaleMonitor::new(jurisdiction),
            reporting_engine: TaxReportingEngine::new(jurisdiction),
        }
    }
    
    /// Add tax lot
    pub fn add_lot(&mut self, lot: TaxLot) {
        self.lots.insert(lot.id, lot);
    }
    
    /// Update current prices
    pub fn update_prices(&mut self, prices: &HashMap<String, Decimal>) {
        for lot in self.lots.values_mut() {
            if let Some(price) = prices.get(&lot.symbol) {
                lot.current_price = Some(*price);
            }
        }
    }
    
    /// Find tax loss harvesting opportunities
    pub fn find_harvest_opportunities(&self) -> Vec<HarvestOpportunity> {
        if !self.jurisdiction.supports_loss_harvesting() {
            return Vec::new();
        }
        
        self.harvester.find_opportunities(self.lots.values().collect())
    }
    
    /// Execute harvest (sell losing position)
    pub fn execute_harvest(&mut self, lot_id: Uuid) -> Result<HarvestResult> {
        let lot = self.lots.get(&lot_id)
            .ok_or_else(|| TaxError::HarvestFailed("Lot not found".to_string()))?
            .clone();
        
        // Check for wash sale violation
        if let Some(violation) = self.wash_sale_monitor.check_wash_sale(&lot) {
            return Err(TaxError::WashSaleViolation(
                format!("Wash sale rule violation: {}", violation.description)
            ));
        }
        
        let result = self.harvester.execute_harvest(&lot)?;
        
        // Record realized gain
        let gain = RealizedGain {
            lot_id,
            symbol: lot.symbol.clone(),
            quantity: lot.quantity,
            purchase_date: lot.purchase_date,
            sale_date: Utc::now(),
            cost_basis: lot.purchase_price * lot.quantity + lot.fees,
            proceeds: result.sale_proceeds,
            gain_loss: result.realized_loss,
            is_short_term: lot.is_short_term(self.jurisdiction),
            wash_sale_adjusted: false,
            adjustment_amount: Decimal::ZERO,
        };
        
        self.realized_gains.push(gain);
        self.lots.remove(&lot_id);
        
        info!(
            "Tax loss harvest executed for {}: realized loss {}",
            lot.symbol, result.realized_loss
        );
        
        Ok(result)
    }
    
    /// Check if can repurchase symbol (wash sale rule)
    pub fn can_repurchase(&self, symbol: &str) -> bool {
        self.wash_sale_monitor.can_repurchase(symbol)
    }
    
    /// Get wash sale violations
    pub fn get_wash_sale_violations(&self) -> Vec<WashSaleViolation> {
        self.wash_sale_monitor.get_violations()
    }
    
    /// Generate tax report for year
    pub fn generate_tax_report(&self, year: i32) -> TaxReport {
        self.reporting_engine.generate_report(
            year,
            &self.realized_gains,
            self.lots.values().collect(),
        )
    }
    
    /// Get unrealized gains/losses summary
    pub fn get_unrealized_summary(&self) -> UnrealizedSummary {
        let mut short_term_losses = Decimal::ZERO;
        let mut long_term_losses = Decimal::ZERO;
        let mut short_term_gains = Decimal::ZERO;
        let mut long_term_gains = Decimal::ZERO;
        
        for lot in self.lots.values() {
            if let Some(pnl) = lot.unrealized_pnl() {
                if lot.is_short_term(self.jurisdiction) {
                    if pnl < Decimal::ZERO {
                        short_term_losses += pnl.abs();
                    } else {
                        short_term_gains += pnl;
                    }
                } else if pnl < Decimal::ZERO {
                    long_term_losses += pnl.abs();
                } else {
                    long_term_gains += pnl;
                }
            }
        }
        
        UnrealizedSummary {
            short_term_unrealized: short_term_gains - short_term_losses,
            long_term_unrealized: long_term_gains - long_term_losses,
            total_unrealized: (short_term_gains + long_term_gains) 
                - (short_term_losses + long_term_losses),
            harvestable_losses: short_term_losses + long_term_losses,
        }
    }
    
    /// Get realized gains/losses for year
    pub fn get_realized_for_year(&self, year: i32) -> RealizedSummary {
        let year_gains: Vec<_> = self.realized_gains.iter()
            .filter(|g| g.sale_date.year() == year)
            .collect();
        
        let short_term: Decimal = year_gains.iter()
            .filter(|g| g.is_short_term)
            .map(|g| g.gain_loss)
            .sum();
        
        let long_term: Decimal = year_gains.iter()
            .filter(|g| !g.is_short_term)
            .map(|g| g.gain_loss)
            .sum();
        
        RealizedSummary {
            short_term_realized: short_term,
            long_term_realized: long_term,
            total_realized: short_term + long_term,
            wash_sale_adjustments: year_gains.iter()
                .map(|g| g.adjustment_amount)
                .sum(),
        }
    }
    
    /// Get lot count
    pub fn lot_count(&self) -> usize {
        self.lots.len()
    }
    
    /// Get realized gain count
    pub fn realized_count(&self) -> usize {
        self.realized_gains.len()
    }
}

impl Default for TaxEngine {
    fn default() -> Self {
        Self::new(TaxJurisdiction::USA)
    }
}

/// Unrealized gains/losses summary
#[derive(Debug, Clone)]
pub struct UnrealizedSummary {
    pub short_term_unrealized: Decimal,
    pub long_term_unrealized: Decimal,
    pub total_unrealized: Decimal,
    pub harvestable_losses: Decimal,
}

/// Realized gains/losses summary
#[derive(Debug, Clone)]
pub struct RealizedSummary {
    pub short_term_realized: Decimal,
    pub long_term_realized: Decimal,
    pub total_realized: Decimal,
    pub wash_sale_adjustments: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_tax_lot_creation() {
        let lot = TaxLot {
            id: Uuid::new_v4(),
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            purchase_date: Utc::now() - Duration::days(30),
            purchase_price: Decimal::from(150),
            current_price: Some(Decimal::from(140)),
            fees: Decimal::from(10),
        };

        let pnl = lot.unrealized_pnl().unwrap();
        assert!(pnl < Decimal::ZERO); // Loss
        assert!(lot.is_short_term(TaxJurisdiction::USA));
    }

    #[test]
    fn test_tax_engine_creation() {
        let engine = TaxEngine::new(TaxJurisdiction::USA);
        assert_eq!(engine.lot_count(), 0);
    }

    #[test]
    fn test_jurisdiction_supports_harvesting() {
        assert!(TaxJurisdiction::USA.supports_loss_harvesting());
        assert!(!TaxJurisdiction::UK.supports_loss_harvesting());
    }

    #[test]
    fn test_tax_year_end() {
        assert_eq!(TaxJurisdiction::USA.tax_year_end(), (12, 31));
        assert_eq!(TaxJurisdiction::UK.tax_year_end(), (4, 5));
    }
}
