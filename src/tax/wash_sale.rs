//! Wash Sale Rule Monitor
//!
//! Tracks and enforces wash sale rules per IRS regulations

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use tracing::{info, warn};
use uuid::Uuid;

use super::{TaxJurisdiction, TaxLot};

/// Wash sale rule tracker
#[derive(Debug)]
pub struct WashSaleMonitor {
    jurisdiction: TaxJurisdiction,
    /// Days before sale to check (default: 30)
    lookback_days: i64,
    /// Days after sale to block (default: 30)
    lookforward_days: i64,
    /// Recent sales at a loss
    recent_loss_sales: Vec<LossSale>,
    /// Recent purchases
    recent_purchases: Vec<PurchaseRecord>,
    /// Blocked repurchases
    blocked_repurchases: HashMap<String, DateTime<Utc>>,
    /// Recorded violations
    violations: Vec<WashSaleViolation>,
}

/// Record of a sale at a loss
#[derive(Debug, Clone)]
struct LossSale {
    id: Uuid,
    symbol: String,
    sale_date: DateTime<Utc>,
    loss_amount: Decimal,
    quantity: Decimal,
}

/// Record of a purchase
#[derive(Debug, Clone)]
struct PurchaseRecord {
    id: Uuid,
    symbol: String,
    purchase_date: DateTime<Utc>,
    quantity: Decimal,
    price: Decimal,
}

/// Wash sale rule definition
#[derive(Debug, Clone)]
pub struct WashSaleRule {
    pub lookback_days: i64,
    pub lookforward_days: i64,
    pub applies_to_substantially_identical: bool,
    pub applies_to_options: bool,
}

impl Default for WashSaleRule {
    fn default() -> Self {
        Self {
            lookback_days: 30,
            lookforward_days: 30,
            applies_to_substantially_identical: true,
            applies_to_options: true,
        }
    }
}

/// Wash sale violation
#[derive(Debug, Clone)]
pub struct WashSaleViolation {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub purchase_id: Uuid,
    pub symbol: String,
    pub description: String,
    pub disallowed_loss: Decimal,
    pub adjustment_basis_increase: Decimal,
    pub detected_at: DateTime<Utc>,
}

impl WashSaleMonitor {
    /// Create new monitor
    pub fn new(jurisdiction: TaxJurisdiction) -> Self {
        Self {
            jurisdiction,
            lookback_days: 30,
            lookforward_days: 30,
            recent_loss_sales: Vec::new(),
            recent_purchases: Vec::new(),
            blocked_repurchases: HashMap::new(),
            violations: Vec::new(),
        }
    }

    /// Record a sale at a loss
    pub fn record_loss_sale(&mut self, symbol: &str, quantity: Decimal, loss: Decimal) -> Uuid {
        let id = Uuid::new_v4();
        let sale = LossSale {
            id,
            symbol: symbol.to_string(),
            sale_date: Utc::now(),
            loss_amount: loss,
            quantity,
        };

        self.recent_loss_sales.push(sale);

        // Block repurchase for lookforward period
        let unblock_date = Utc::now() + Duration::days(self.lookforward_days);
        self.blocked_repurchases
            .insert(symbol.to_string(), unblock_date);

        info!(
            "Loss sale recorded for {}: {}, repurchase blocked until {}",
            symbol,
            loss,
            unblock_date.format("%Y-%m-%d")
        );

        id
    }

    /// Record a purchase
    pub fn record_purchase(&mut self, symbol: &str, quantity: Decimal, price: Decimal) -> Uuid {
        let id = Uuid::new_v4();
        let purchase = PurchaseRecord {
            id,
            symbol: symbol.to_string(),
            purchase_date: Utc::now(),
            quantity,
            price,
        };

        self.recent_purchases.push(purchase.clone());

        // Check if this creates a wash sale with recent loss sales
        self.check_wash_sale_for_purchase(&purchase);

        id
    }

    /// Check if lot can be harvested (no recent purchases)
    pub fn check_wash_sale(&self, lot: &TaxLot) -> Option<WashSaleViolation> {
        let cutoff = Utc::now() - Duration::days(self.lookback_days);

        // Check for purchases within lookback window
        for purchase in &self.recent_purchases {
            if purchase.symbol == lot.symbol && purchase.purchase_date > cutoff {
                return Some(WashSaleViolation {
                    id: Uuid::new_v4(),
                    sale_id: lot.id,
                    purchase_id: purchase.id,
                    symbol: lot.symbol.clone(),
                    description: format!(
                        "Purchase of {} on {} within 30 days of intended sale",
                        lot.symbol,
                        purchase.purchase_date.format("%Y-%m-%d")
                    ),
                    disallowed_loss: Decimal::ZERO, // Would be calculated
                    adjustment_basis_increase: Decimal::ZERO,
                    detected_at: Utc::now(),
                });
            }
        }

        // Check substantially identical securities
        if let Some(substantially_identical) = self.get_substantially_identical(&lot.symbol) {
            for purchase in &self.recent_purchases {
                if substantially_identical.contains(&purchase.symbol)
                    && purchase.purchase_date > cutoff
                {
                    return Some(WashSaleViolation {
                        id: Uuid::new_v4(),
                        sale_id: lot.id,
                        purchase_id: purchase.id,
                        symbol: lot.symbol.clone(),
                        description: format!(
                            "Purchase of substantially identical security {} on {}",
                            purchase.symbol,
                            purchase.purchase_date.format("%Y-%m-%d")
                        ),
                        disallowed_loss: Decimal::ZERO,
                        adjustment_basis_increase: Decimal::ZERO,
                        detected_at: Utc::now(),
                    });
                }
            }
        }

        None
    }

    /// Check wash sale for a purchase (creates violation)
    fn check_wash_sale_for_purchase(&mut self, purchase: &PurchaseRecord) {
        let cutoff = Utc::now() - Duration::days(self.lookforward_days);

        for sale in &self.recent_loss_sales {
            if sale.symbol == purchase.symbol && sale.sale_date > cutoff {
                let violation = WashSaleViolation {
                    id: Uuid::new_v4(),
                    sale_id: sale.id,
                    purchase_id: purchase.id,
                    symbol: purchase.symbol.clone(),
                    description: format!(
                        "Wash sale: Purchase of {} within 30 days of loss sale",
                        purchase.symbol
                    ),
                    disallowed_loss: sale.loss_amount.min(purchase.quantity * purchase.price),
                    adjustment_basis_increase: sale.loss_amount,
                    detected_at: Utc::now(),
                };

                warn!(
                    "Wash sale violation detected: {} - disallowed loss: {}",
                    violation.description, violation.disallowed_loss
                );

                self.violations.push(violation);
            }
        }
    }

    /// Check if can repurchase symbol
    pub fn can_repurchase(&self, symbol: &str) -> bool {
        if let Some(unblock_date) = self.blocked_repurchases.get(symbol) {
            return Utc::now() >= *unblock_date;
        }
        true
    }

    /// Get when repurchase will be allowed
    pub fn repurchase_available_date(&self, symbol: &str) -> Option<DateTime<Utc>> {
        self.blocked_repurchases.get(symbol).copied()
    }

    /// Get all violations
    pub fn get_violations(&self) -> Vec<WashSaleViolation> {
        self.violations.clone()
    }

    /// Get total disallowed losses
    pub fn total_disallowed_losses(&self) -> Decimal {
        self.violations.iter().map(|v| v.disallowed_loss).sum()
    }

    /// Clean old records
    pub fn clean_old_records(&mut self) {
        let cutoff = Utc::now() - Duration::days(90); // Keep 90 days

        self.recent_loss_sales.retain(|s| s.sale_date > cutoff);
        self.recent_purchases.retain(|p| p.purchase_date > cutoff);

        // Clean expired blocks
        let now = Utc::now();
        self.blocked_repurchases.retain(|_, date| *date > now);
    }

    /// Get substantially identical securities
    fn get_substantially_identical(&self, symbol: &str) -> Option<HashSet<String>> {
        // Simplified mapping
        let groups: HashMap<&str, HashSet<&str>> = [
            ("SPY", vec!["VOO", "IVV"].into_iter().collect()),
            ("VOO", vec!["SPY", "IVV"].into_iter().collect()),
            ("IVV", vec!["SPY", "VOO"].into_iter().collect()),
            ("QQQ", vec!["QQQM"].into_iter().collect()),
            ("QQQM", vec!["QQQ"].into_iter().collect()),
        ]
        .iter()
        .cloned()
        .collect();

        groups
            .get(symbol)
            .map(|s| s.iter().map(|&s| s.to_string()).collect())
    }

    /// Set lookback/lookforward days
    pub fn set_lookback_days(&mut self, days: i64) {
        self.lookback_days = days;
    }

    pub fn set_lookforward_days(&mut self, days: i64) {
        self.lookforward_days = days;
    }
}

impl Default for WashSaleMonitor {
    fn default() -> Self {
        Self::new(TaxJurisdiction::USA)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let monitor = WashSaleMonitor::new(TaxJurisdiction::USA);
        assert!(monitor.get_violations().is_empty());
    }

    #[test]
    fn test_record_loss_sale() {
        let mut monitor = WashSaleMonitor::new(TaxJurisdiction::USA);
        let id = monitor.record_loss_sale("AAPL", Decimal::from(100), Decimal::from(1000));

        assert!(!monitor.can_repurchase("AAPL"));
        assert!(monitor.repurchase_available_date("AAPL").is_some());
    }

    #[test]
    fn test_wash_sale_detection() {
        let mut monitor = WashSaleMonitor::new(TaxJurisdiction::USA);

        // Record purchase first
        monitor.record_purchase("AAPL", Decimal::from(100), Decimal::from(150));

        // Create lot for sale
        let lot = TaxLot {
            id: Uuid::new_v4(),
            symbol: "AAPL".to_string(),
            quantity: Decimal::from(100),
            purchase_date: Utc::now() - Duration::days(60),
            purchase_price: Decimal::from(160),
            current_price: Some(Decimal::from(140)),
            fees: Decimal::ZERO,
        };

        // Check for wash sale
        let violation = monitor.check_wash_sale(&lot);
        assert!(violation.is_some());
        assert_eq!(violation.unwrap().symbol, "AAPL");
    }

    #[test]
    fn test_clean_old_records() {
        let mut monitor = WashSaleMonitor::new(TaxJurisdiction::USA);

        monitor.record_loss_sale("AAPL", Decimal::from(100), Decimal::from(1000));
        assert!(!monitor.can_repurchase("AAPL"));

        // Clean shouldn't affect recent records
        monitor.clean_old_records();
        // Note: in test, we can't time travel, so records are still there
    }

    #[test]
    fn test_substantially_identical() {
        let monitor = WashSaleMonitor::new(TaxJurisdiction::USA);

        let spy_group = monitor.get_substantially_identical("SPY");
        assert!(spy_group.is_some());
        assert!(spy_group.unwrap().contains("VOO"));
    }
}
