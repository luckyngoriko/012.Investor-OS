//! Tax Loss Harvesting
//!
//! Identifies and executes tax loss harvesting opportunities

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use tracing::info;

use super::{TaxError, TaxJurisdiction, TaxLot};

/// Tax loss harvester
#[derive(Debug)]
pub struct TaxLossHarvester {
    jurisdiction: TaxJurisdiction,
    min_harvest_amount: Decimal,
    max_harvests_per_month: u32,
    recent_harvests: Vec<DateTime<Utc>>,
}

/// Harvest opportunity
#[derive(Debug, Clone)]
pub struct HarvestOpportunity {
    pub lot_id: uuid::Uuid,
    pub symbol: String,
    pub quantity: Decimal,
    pub unrealized_loss: Decimal,
    pub days_held: i64,
    pub is_short_term: bool,
    pub harvest_value: Decimal, // Tax savings potential
}

/// Harvest execution result
#[derive(Debug, Clone)]
pub struct HarvestResult {
    pub lot_id: uuid::Uuid,
    pub symbol: String,
    pub realized_loss: Decimal,
    pub sale_proceeds: Decimal,
    pub tax_savings_estimate: Decimal,
    pub executed_at: DateTime<Utc>,
}

impl TaxLossHarvester {
    /// Create new harvester
    pub fn new(jurisdiction: TaxJurisdiction) -> Self {
        Self {
            jurisdiction,
            min_harvest_amount: Decimal::from(1000), // $1,000 minimum
            max_harvests_per_month: 10,
            recent_harvests: Vec::new(),
        }
    }

    /// Find harvesting opportunities
    pub fn find_opportunities(&self, lots: Vec<&TaxLot>) -> Vec<HarvestOpportunity> {
        let mut opportunities = Vec::new();

        for lot in lots {
            // Check if has unrealized loss
            if let Some(pnl) = lot.unrealized_pnl() {
                if pnl >= Decimal::ZERO {
                    continue; // Skip gains
                }

                let loss = pnl.abs();

                // Check minimum threshold
                if loss < self.min_harvest_amount {
                    continue;
                }

                let is_short_term = lot.is_short_term(self.jurisdiction);

                // Calculate tax savings (simplified)
                let tax_rate = if is_short_term {
                    Decimal::try_from(0.37).unwrap() // Short-term rate
                } else {
                    Decimal::try_from(0.20).unwrap() // Long-term rate
                };

                let harvest_value = loss * tax_rate;

                opportunities.push(HarvestOpportunity {
                    lot_id: lot.id,
                    symbol: lot.symbol.clone(),
                    quantity: lot.quantity,
                    unrealized_loss: loss,
                    days_held: lot.days_held(),
                    is_short_term,
                    harvest_value,
                });
            }
        }

        // Sort by harvest value (descending)
        opportunities.sort_by(|a, b| b.harvest_value.cmp(&a.harvest_value));

        opportunities
    }

    /// Execute harvest
    pub fn execute_harvest(&mut self, lot: &TaxLot) -> Result<HarvestResult, TaxError> {
        // Check harvest limit
        self.clean_old_harvests();
        if self.recent_harvests.len() >= self.max_harvests_per_month as usize {
            return Err(TaxError::HarvestFailed(
                "Maximum harvests per month reached".to_string(),
            ));
        }

        let pnl = lot
            .unrealized_pnl()
            .ok_or_else(|| TaxError::HarvestFailed("No price available".to_string()))?;

        if pnl >= Decimal::ZERO {
            return Err(TaxError::HarvestFailed(
                "Position is not at a loss".to_string(),
            ));
        }

        let realized_loss = pnl.abs();
        let sale_proceeds = lot.current_price.unwrap() * lot.quantity - lot.fees;

        // Calculate tax savings
        let tax_rate = if lot.is_short_term(self.jurisdiction) {
            Decimal::try_from(0.37).unwrap()
        } else {
            Decimal::try_from(0.20).unwrap()
        };

        let tax_savings = realized_loss * tax_rate;

        self.recent_harvests.push(Utc::now());

        info!(
            "Tax loss harvest: {} {} shares, loss: {}, estimated savings: {}",
            lot.symbol, lot.quantity, realized_loss, tax_savings
        );

        Ok(HarvestResult {
            lot_id: lot.id,
            symbol: lot.symbol.clone(),
            realized_loss,
            sale_proceeds,
            tax_savings_estimate: tax_savings,
            executed_at: Utc::now(),
        })
    }

    /// Set minimum harvest amount
    pub fn set_min_amount(&mut self, amount: Decimal) {
        self.min_harvest_amount = amount;
    }

    /// Get recent harvest count
    pub fn recent_harvest_count(&self) -> usize {
        self.recent_harvests.len()
    }

    /// Clean harvests older than 30 days
    fn clean_old_harvests(&mut self) {
        let cutoff = Utc::now() - Duration::days(30);
        self.recent_harvests.retain(|h| *h > cutoff);
    }

    /// Calculate total harvested losses this year
    pub fn total_harvested_this_year(&self) -> Decimal {
        // Simplified - would track actual harvests
        Decimal::ZERO
    }

    /// Get replacement suggestions (similar but not identical securities)
    pub fn get_replacement_suggestions(&self, symbol: &str) -> Vec<String> {
        // Simplified mapping - real implementation would use correlation analysis
        let replacements: HashMap<&str, Vec<&str>> = [
            ("SPY", vec!["VOO", "IVV"]),
            ("QQQ", vec!["QQQM"]),
            ("VTI", vec!["ITOT", "SCHB"]),
            ("AAPL", vec!["QQQ", "VGT"]), // Tech exposure
            ("MSFT", vec!["QQQ", "VGT"]),
            ("GOOGL", vec!["QQQ", "FTEC"]),
        ]
        .iter()
        .cloned()
        .collect();

        replacements
            .get(symbol)
            .map(|v| v.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }
}

use std::collections::HashMap;

impl Default for TaxLossHarvester {
    fn default() -> Self {
        Self::new(TaxJurisdiction::USA)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_lot(price: Decimal, current: Decimal, days_ago: i64) -> TaxLot {
        TaxLot {
            id: Uuid::new_v4(),
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            purchase_date: Utc::now() - Duration::days(days_ago),
            purchase_price: price,
            current_price: Some(current),
            fees: Decimal::from(10),
        }
    }

    #[test]
    fn test_harvester_creation() {
        let harvester = TaxLossHarvester::new(TaxJurisdiction::USA);
        assert_eq!(harvester.recent_harvest_count(), 0);
    }

    #[test]
    fn test_find_opportunities() {
        let harvester = TaxLossHarvester::new(TaxJurisdiction::USA);

        let lot1 = create_test_lot(Decimal::from(150), Decimal::from(140), 30); // Loss
        let lot2 = create_test_lot(Decimal::from(150), Decimal::from(160), 30); // Gain

        let opportunities = harvester.find_opportunities(vec![&lot1, &lot2]);

        assert_eq!(opportunities.len(), 1);
        assert_eq!(opportunities[0].symbol, "AAPL");
    }

    #[test]
    fn test_harvest_threshold() {
        let mut harvester = TaxLossHarvester::new(TaxJurisdiction::USA);
        harvester.set_min_amount(Decimal::from(5000));

        let lot = create_test_lot(Decimal::from(100), Decimal::from(99), 30); // Small loss
        let opportunities = harvester.find_opportunities(vec![&lot]);

        // Loss is only $100, below threshold
        assert!(opportunities.is_empty());
    }

    #[test]
    fn test_execute_harvest() {
        let mut harvester = TaxLossHarvester::new(TaxJurisdiction::USA);
        let lot = create_test_lot(Decimal::from(150), Decimal::from(130), 30);

        let result = harvester.execute_harvest(&lot).unwrap();

        assert!(result.realized_loss > Decimal::ZERO);
        assert!(result.tax_savings_estimate > Decimal::ZERO);
        assert_eq!(harvester.recent_harvest_count(), 1);
    }

    #[test]
    fn test_replacement_suggestions() {
        let harvester = TaxLossHarvester::new(TaxJurisdiction::USA);

        let suggestions = harvester.get_replacement_suggestions("SPY");
        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"VOO".to_string()));
    }
}
