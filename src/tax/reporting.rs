//! Tax Reporting Engine
//!
//! Generates tax reports and forms (Schedule D, Form 8949, etc.)

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::info;

use super::{RealizedGain, TaxJurisdiction, TaxLot};

/// Tax year
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaxYear(pub i32);

impl TaxYear {
    pub fn current() -> Self {
        Self(Utc::now().year())
    }
    
    pub fn previous() -> Self {
        Self(Utc::now().year() - 1)
    }
}

/// Tax reporting engine
#[derive(Debug)]
pub struct TaxReportingEngine {
    jurisdiction: TaxJurisdiction,
}

/// Tax report
#[derive(Debug, Clone)]
pub struct TaxReport {
    pub year: i32,
    pub generated_at: DateTime<Utc>,
    pub short_term_gains: Decimal,
    pub short_term_losses: Decimal,
    pub long_term_gains: Decimal,
    pub long_term_losses: Decimal,
    pub net_short_term: Decimal,
    pub net_long_term: Decimal,
    pub total_net: Decimal,
    pub wash_sale_adjustments: Decimal,
    pub transactions: Vec<ReportTransaction>,
    pub summary_by_symbol: HashMap<String, SymbolSummary>,
}

/// Report transaction
#[derive(Debug, Clone)]
pub struct ReportTransaction {
    pub symbol: String,
    pub quantity: Decimal,
    pub purchase_date: DateTime<Utc>,
    pub sale_date: DateTime<Utc>,
    pub cost_basis: Decimal,
    pub proceeds: Decimal,
    pub gain_loss: Decimal,
    pub term: TermType,
    pub wash_sale_adjustment: Decimal,
}

/// Term type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermType {
    ShortTerm,
    LongTerm,
}

/// Symbol summary
#[derive(Debug, Clone)]
pub struct SymbolSummary {
    pub symbol: String,
    pub total_transactions: u32,
    pub total_gains: Decimal,
    pub total_losses: Decimal,
    pub net_pnl: Decimal,
}

/// Tax form types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxForm {
    ScheduleD,      // Capital gains summary
    Form8949,       // Sales and other dispositions
    Form1099B,      // Broker proceeds
}

impl TaxReportingEngine {
    /// Create new reporting engine
    pub fn new(jurisdiction: TaxJurisdiction) -> Self {
        Self { jurisdiction }
    }
    
    /// Generate tax report for year
    pub fn generate_report(
        &self,
        year: i32,
        realized_gains: &[RealizedGain],
        _open_lots: Vec<&TaxLot>,
    ) -> TaxReport {
        let year_gains: Vec<_> = realized_gains.iter()
            .filter(|g| g.sale_date.year() == year)
            .collect();
        
        let mut short_term_gains = Decimal::ZERO;
        let mut short_term_losses = Decimal::ZERO;
        let mut long_term_gains = Decimal::ZERO;
        let mut long_term_losses = Decimal::ZERO;
        let mut wash_sale_adjustments = Decimal::ZERO;
        
        let mut transactions = Vec::new();
        let mut symbol_summary: HashMap<String, SymbolSummary> = HashMap::new();
        
        for gain in &year_gains {
            let term = if gain.is_short_term {
                TermType::ShortTerm
            } else {
                TermType::LongTerm
            };
            
            let transaction = ReportTransaction {
                symbol: gain.symbol.clone(),
                quantity: gain.quantity,
                purchase_date: gain.purchase_date,
                sale_date: gain.sale_date,
                cost_basis: gain.cost_basis,
                proceeds: gain.proceeds,
                gain_loss: gain.gain_loss,
                term,
                wash_sale_adjustment: gain.adjustment_amount,
            };
            
            // Categorize gains/losses
            if gain.is_short_term {
                if gain.gain_loss >= Decimal::ZERO {
                    short_term_gains += gain.gain_loss;
                } else {
                    short_term_losses += gain.gain_loss.abs();
                }
            } else if gain.gain_loss >= Decimal::ZERO {
                long_term_gains += gain.gain_loss;
            } else {
                long_term_losses += gain.gain_loss.abs();
            }
            
            wash_sale_adjustments += gain.adjustment_amount;
            transactions.push(transaction);
            
            // Update symbol summary
            let summary = symbol_summary.entry(gain.symbol.clone()).or_insert(
                SymbolSummary {
                    symbol: gain.symbol.clone(),
                    total_transactions: 0,
                    total_gains: Decimal::ZERO,
                    total_losses: Decimal::ZERO,
                    net_pnl: Decimal::ZERO,
                }
            );
            
            summary.total_transactions += 1;
            if gain.gain_loss >= Decimal::ZERO {
                summary.total_gains += gain.gain_loss;
            } else {
                summary.total_losses += gain.gain_loss.abs();
            }
            summary.net_pnl += gain.gain_loss;
        }
        
        let net_short_term = short_term_gains - short_term_losses;
        let net_long_term = long_term_gains - long_term_losses;
        let total_net = net_short_term + net_long_term;
        
        info!(
            "Tax report generated for {}: {} transactions, net P&L: {}",
            year, transactions.len(), total_net
        );
        
        TaxReport {
            year,
            generated_at: Utc::now(),
            short_term_gains,
            short_term_losses,
            long_term_gains,
            long_term_losses,
            net_short_term,
            net_long_term,
            total_net,
            wash_sale_adjustments,
            transactions,
            summary_by_symbol: symbol_summary,
        }
    }
    
    /// Generate Schedule D (Capital Gains)
    pub fn generate_schedule_d(&self, report: &TaxReport) -> ScheduleD {
        ScheduleD {
            tax_year: report.year,
            short_term_gains: report.short_term_gains,
            short_term_losses: report.short_term_losses,
            net_short_term: report.net_short_term,
            long_term_gains: report.long_term_gains,
            long_term_losses: report.long_term_losses,
            net_long_term: report.net_long_term,
            total_net_gain_loss: report.total_net,
            capital_loss_carryover: self.calculate_carryover(report),
        }
    }
    
    /// Generate Form 8949 transactions
    pub fn generate_form_8949(&self, report: &TaxReport, term: TermType) -> Form8949 {
        let transactions: Vec<_> = report.transactions.iter()
            .filter(|t| t.term == term)
            .cloned()
            .collect();
        
        let total_proceeds: Decimal = transactions.iter().map(|t| t.proceeds).sum();
        let total_cost: Decimal = transactions.iter().map(|t| t.cost_basis).sum();
        let total_adjustments: Decimal = transactions.iter().map(|t| t.wash_sale_adjustment).sum();
        let total_gain_loss: Decimal = transactions.iter().map(|t| t.gain_loss).sum();
        
        Form8949 {
            tax_year: report.year,
            term,
            transactions,
            total_proceeds,
            total_cost,
            total_adjustments,
            total_gain_loss,
        }
    }
    
    /// Calculate capital loss carryover
    fn calculate_carryover(&self, report: &TaxReport) -> CapitalLossCarryover {
        // Simplified calculation
        let max_deduction = Decimal::from(3000);
        let loss_available = if report.total_net < Decimal::ZERO {
            report.total_net.abs()
        } else {
            Decimal::ZERO
        };
        
        let current_deduction = loss_available.min(max_deduction);
        let carryover = loss_available - current_deduction;
        
        CapitalLossCarryover {
            short_term_carryover: if report.net_short_term < Decimal::ZERO {
                report.net_short_term.abs()
            } else {
                Decimal::ZERO
            },
            long_term_carryover: if report.net_long_term < Decimal::ZERO {
                report.net_long_term.abs()
            } else {
                Decimal::ZERO
            },
            total_carryover: carryover,
        }
    }
    
    /// Export to CSV format
    pub fn export_csv(&self, report: &TaxReport) -> String {
        let mut csv = String::new();
        
        // Header
        csv.push_str("Symbol,Quantity,Purchase Date,Sale Date,Cost Basis,Proceeds,Gain/Loss,Term\n");
        
        // Transactions
        for t in &report.transactions {
            let term = match t.term {
                TermType::ShortTerm => "Short",
                TermType::LongTerm => "Long",
            };
            
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                t.symbol, t.quantity, 
                t.purchase_date.format("%Y-%m-%d"),
                t.sale_date.format("%Y-%m-%d"),
                t.cost_basis, t.proceeds, t.gain_loss,
                term
            ));
        }
        
        csv
    }
    
    /// Get tax filing deadline for year
    pub fn filing_deadline(&self, year: i32) -> NaiveDate {
        match self.jurisdiction {
            TaxJurisdiction::USA => {
                // April 15th following year
                NaiveDate::from_ymd_opt(year + 1, 4, 15)
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(year + 1, 4, 15).unwrap())
            }
            _ => NaiveDate::from_ymd_opt(year + 1, 4, 15).unwrap(),
        }
    }
}

impl Default for TaxReportingEngine {
    fn default() -> Self {
        Self::new(TaxJurisdiction::USA)
    }
}

/// Schedule D (Capital Gains and Losses)
#[derive(Debug, Clone)]
pub struct ScheduleD {
    pub tax_year: i32,
    pub short_term_gains: Decimal,
    pub short_term_losses: Decimal,
    pub net_short_term: Decimal,
    pub long_term_gains: Decimal,
    pub long_term_losses: Decimal,
    pub net_long_term: Decimal,
    pub total_net_gain_loss: Decimal,
    pub capital_loss_carryover: CapitalLossCarryover,
}

/// Form 8949 (Sales and Other Dispositions)
#[derive(Debug, Clone)]
pub struct Form8949 {
    pub tax_year: i32,
    pub term: TermType,
    pub transactions: Vec<ReportTransaction>,
    pub total_proceeds: Decimal,
    pub total_cost: Decimal,
    pub total_adjustments: Decimal,
    pub total_gain_loss: Decimal,
}

/// Capital loss carryover
#[derive(Debug, Clone)]
pub struct CapitalLossCarryover {
    pub short_term_carryover: Decimal,
    pub long_term_carryover: Decimal,
    pub total_carryover: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_gain(symbol: &str, gain: Decimal, short_term: bool, year: i32) -> RealizedGain {
        let now = Utc::now();
        let purchase_date = now - chrono::Duration::days(if short_term { 30 } else { 400 });
        
        RealizedGain {
            lot_id: Uuid::new_v4(),
            symbol: symbol.to_string(),
            quantity: Decimal::from(100),
            purchase_date,
            sale_date: now.with_year(year).unwrap_or(now),
            cost_basis: Decimal::from(10000),
            proceeds: Decimal::from(10000) + gain,
            gain_loss: gain,
            is_short_term: short_term,
            wash_sale_adjusted: false,
            adjustment_amount: Decimal::ZERO,
        }
    }

    #[test]
    fn test_reporting_engine_creation() {
        let engine = TaxReportingEngine::new(TaxJurisdiction::USA);
        assert_eq!(engine.filing_deadline(2024).month(), 4);
    }

    #[test]
    fn test_generate_report() {
        let engine = TaxReportingEngine::new(TaxJurisdiction::USA);
        let year = Utc::now().year();
        
        let gains = vec![
            create_test_gain("AAPL", Decimal::from(1000), true, year),
            create_test_gain("MSFT", Decimal::from(-500), false, year),
        ];
        
        let report = engine.generate_report(year, &gains, vec![]);
        
        assert_eq!(report.year, year);
        assert_eq!(report.transactions.len(), 2);
        assert_eq!(report.short_term_gains, Decimal::from(1000));
        assert_eq!(report.long_term_losses, Decimal::from(500));
    }

    #[test]
    fn test_schedule_d_generation() {
        let engine = TaxReportingEngine::new(TaxJurisdiction::USA);
        let year = Utc::now().year();
        
        let gains = vec![
            create_test_gain("AAPL", Decimal::from(1000), true, year),
            create_test_gain("GOOGL", Decimal::from(2000), false, year),
        ];
        
        let report = engine.generate_report(year, &gains, vec![]);
        let schedule_d = engine.generate_schedule_d(&report);
        
        assert_eq!(schedule_d.tax_year, year);
        assert_eq!(schedule_d.short_term_gains, Decimal::from(1000));
        assert_eq!(schedule_d.long_term_gains, Decimal::from(2000));
    }

    #[test]
    fn test_csv_export() {
        let engine = TaxReportingEngine::new(TaxJurisdiction::USA);
        let year = Utc::now().year();
        
        let gains = vec![create_test_gain("AAPL", Decimal::from(1000), true, year)];
        let report = engine.generate_report(year, &gains, vec![]);
        let csv = engine.export_csv(&report);
        
        assert!(csv.contains("Symbol,Quantity"));
        assert!(csv.contains("AAPL"));
    }

    #[test]
    fn test_capital_loss_carryover() {
        let engine = TaxReportingEngine::new(TaxJurisdiction::USA);
        let year = Utc::now().year();
        
        let gains = vec![create_test_gain("AAPL", Decimal::from(-10000), true, year)];
        let report = engine.generate_report(year, &gains, vec![]);
        let schedule_d = engine.generate_schedule_d(&report);
        
        assert!(schedule_d.capital_loss_carryover.total_carryover > Decimal::ZERO);
    }
}
