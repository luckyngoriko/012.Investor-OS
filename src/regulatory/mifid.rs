//! MiFID II Regulatory Compliance (Markets in Financial Instruments Directive II)
//!
//! Implements key MiFID II requirements:
//! - Article 26 Transaction Reporting (LEI, instrument ID, price, quantity, timestamp, venue)
//! - Best Execution assessment (comparing executed price vs market reference)
//! - Client Classification (Retail / Professional / Eligible Counterparty)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Types ───────────────────────────────────────────────────────────────────

/// Client category under MiFID II
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientCategory {
    /// Default classification — highest protection
    Retail,
    /// Meets experience / portfolio / transaction thresholds
    Professional,
    /// Central banks, credit institutions, investment firms, etc.
    EligibleCounterparty,
}

/// Input for client classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientProfile {
    pub user_id: Uuid,
    /// Number of significant-size trades in the last 4 quarters
    pub trades_last_4q: u32,
    /// Portfolio value in EUR
    pub portfolio_value_eur: f64,
    /// Years of relevant professional experience in the financial sector
    pub professional_experience_years: u32,
    /// Whether the entity is a regulated institution (bank, fund, etc.)
    pub is_regulated_entity: bool,
}

/// Trade input for transaction reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeInput {
    pub trade_id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub venue: String,
    pub executed_at: DateTime<Utc>,
}

/// MiFID II Article 26 transaction report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MifidReport {
    pub report_id: Uuid,
    pub trade_id: Uuid,
    /// Legal Entity Identifier (placeholder — real LEI from GLEIF registry)
    pub lei: String,
    /// ISIN or internal instrument identifier
    pub instrument_id: String,
    pub price: f64,
    pub quantity: f64,
    pub side: String,
    pub venue: String,
    pub executed_at: DateTime<Utc>,
    pub reported_at: DateTime<Utc>,
}

/// Result of a best-execution check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestExecutionResult {
    pub trade_id: Uuid,
    /// Executed price
    pub executed_price: f64,
    /// Reference market price at time of execution
    pub market_price: f64,
    /// Price deviation: (executed - market) / market  (negative = better for buy)
    pub deviation_pct: f64,
    /// Whether execution meets best-execution threshold (within 0.5%)
    pub is_best_execution: bool,
    pub assessment: String,
}

// ─── MifidReporter ───────────────────────────────────────────────────────────

/// MiFID II compliance reporter
#[derive(Debug, Clone)]
pub struct MifidReporter {
    /// Entity LEI (assigned by GLEIF)
    lei: String,
}

impl MifidReporter {
    /// Create a new reporter with the firm's LEI
    pub fn new(lei: impl Into<String>) -> Self {
        Self { lei: lei.into() }
    }

    /// Generate an Article 26 transaction report for a trade
    pub fn transaction_report(&self, trade: &TradeInput) -> MifidReport {
        MifidReport {
            report_id: Uuid::new_v4(),
            trade_id: trade.trade_id,
            lei: self.lei.clone(),
            instrument_id: symbol_to_instrument_id(&trade.symbol),
            price: trade.price,
            quantity: trade.quantity,
            side: trade.side.clone(),
            venue: trade.venue.clone(),
            executed_at: trade.executed_at,
            reported_at: Utc::now(),
        }
    }

    /// Check best execution by comparing executed price against market reference price
    pub fn best_execution_check(
        &self,
        trade: &TradeInput,
        market_price: f64,
    ) -> BestExecutionResult {
        let deviation = if market_price > 0.0 {
            (trade.price - market_price) / market_price
        } else {
            0.0
        };
        let deviation_pct = deviation * 100.0;

        // Best execution threshold: within 0.5% of market price
        let threshold = 0.5;
        let is_best = deviation_pct.abs() <= threshold;

        let assessment = if is_best {
            format!(
                "Best execution achieved: deviation {:.3}% within {:.1}% threshold",
                deviation_pct, threshold
            )
        } else {
            format!(
                "Best execution NOT achieved: deviation {:.3}% exceeds {:.1}% threshold",
                deviation_pct, threshold
            )
        };

        BestExecutionResult {
            trade_id: trade.trade_id,
            executed_price: trade.price,
            market_price,
            deviation_pct,
            is_best_execution: is_best,
            assessment,
        }
    }

    /// Classify a client according to MiFID II categories
    ///
    /// Professional criteria (must meet at least 2 of 3):
    /// 1. >= 10 significant-size trades per quarter (40 in 4 quarters)
    /// 2. Portfolio value > EUR 500,000
    /// 3. >= 1 year professional experience in the financial sector
    ///
    /// Eligible Counterparty: regulated financial institution
    pub fn client_classification(&self, profile: &ClientProfile) -> ClientCategory {
        // Eligible counterparties are always regulated entities
        if profile.is_regulated_entity {
            return ClientCategory::EligibleCounterparty;
        }

        // Professional: must meet at least 2 of 3 criteria
        let mut criteria_met = 0u8;

        // Criterion 1: >= 40 significant trades in last 4 quarters
        if profile.trades_last_4q >= 40 {
            criteria_met += 1;
        }

        // Criterion 2: portfolio > EUR 500,000
        if profile.portfolio_value_eur > 500_000.0 {
            criteria_met += 1;
        }

        // Criterion 3: >= 1 year financial sector experience
        if profile.professional_experience_years >= 1 {
            criteria_met += 1;
        }

        if criteria_met >= 2 {
            ClientCategory::Professional
        } else {
            ClientCategory::Retail
        }
    }
}

/// Convert a trading symbol to an instrument identifier (placeholder for real ISIN lookup)
fn symbol_to_instrument_id(symbol: &str) -> String {
    // In production, this would look up the ISIN from a reference data provider.
    // For now, prefix with internal namespace.
    format!("IOS:{}", symbol.to_uppercase())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_reporter() -> MifidReporter {
        MifidReporter::new("529900ABCDEF0123456789")
    }

    fn sample_trade() -> TradeInput {
        TradeInput {
            trade_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            symbol: "BTCUSDT".to_string(),
            side: "buy".to_string(),
            price: 65000.0,
            quantity: 0.5,
            venue: "binance".to_string(),
            executed_at: Utc::now(),
        }
    }

    // ── Client classification tests ──

    #[test]
    fn test_classify_retail_default() {
        let reporter = test_reporter();
        let profile = ClientProfile {
            user_id: Uuid::new_v4(),
            trades_last_4q: 5,
            portfolio_value_eur: 10_000.0,
            professional_experience_years: 0,
            is_regulated_entity: false,
        };
        assert_eq!(
            reporter.client_classification(&profile),
            ClientCategory::Retail
        );
    }

    #[test]
    fn test_classify_professional_two_criteria() {
        let reporter = test_reporter();
        let profile = ClientProfile {
            user_id: Uuid::new_v4(),
            trades_last_4q: 50,
            portfolio_value_eur: 600_000.0,
            professional_experience_years: 0,
            is_regulated_entity: false,
        };
        assert_eq!(
            reporter.client_classification(&profile),
            ClientCategory::Professional
        );
    }

    #[test]
    fn test_classify_professional_all_three() {
        let reporter = test_reporter();
        let profile = ClientProfile {
            user_id: Uuid::new_v4(),
            trades_last_4q: 100,
            portfolio_value_eur: 1_000_000.0,
            professional_experience_years: 5,
            is_regulated_entity: false,
        };
        assert_eq!(
            reporter.client_classification(&profile),
            ClientCategory::Professional
        );
    }

    #[test]
    fn test_classify_eligible_counterparty() {
        let reporter = test_reporter();
        let profile = ClientProfile {
            user_id: Uuid::new_v4(),
            trades_last_4q: 0,
            portfolio_value_eur: 0.0,
            professional_experience_years: 0,
            is_regulated_entity: true,
        };
        assert_eq!(
            reporter.client_classification(&profile),
            ClientCategory::EligibleCounterparty
        );
    }

    #[test]
    fn test_classify_retail_one_criterion_only() {
        let reporter = test_reporter();
        let profile = ClientProfile {
            user_id: Uuid::new_v4(),
            trades_last_4q: 50,
            portfolio_value_eur: 100_000.0,
            professional_experience_years: 0,
            is_regulated_entity: false,
        };
        // Only trades criterion met (1 of 3) → Retail
        assert_eq!(
            reporter.client_classification(&profile),
            ClientCategory::Retail
        );
    }

    // ── Transaction report tests ──

    #[test]
    fn test_transaction_report_fields() {
        let reporter = test_reporter();
        let trade = sample_trade();
        let report = reporter.transaction_report(&trade);

        assert_eq!(report.trade_id, trade.trade_id);
        assert_eq!(report.lei, "529900ABCDEF0123456789");
        assert_eq!(report.instrument_id, "IOS:BTCUSDT");
        assert_eq!(report.price, 65000.0);
        assert_eq!(report.quantity, 0.5);
        assert_eq!(report.side, "buy");
        assert_eq!(report.venue, "binance");
    }

    // ── Best execution tests ──

    #[test]
    fn test_best_execution_within_threshold() {
        let reporter = test_reporter();
        let trade = sample_trade(); // price = 65000.0
        let market_price = 65100.0; // slight deviation

        let result = reporter.best_execution_check(&trade, market_price);
        assert!(result.is_best_execution);
        assert!(result.deviation_pct.abs() < 0.5);
    }

    #[test]
    fn test_best_execution_exceeds_threshold() {
        let reporter = test_reporter();
        let mut trade = sample_trade();
        trade.price = 66000.0; // overpaid significantly
        let market_price = 65000.0;

        let result = reporter.best_execution_check(&trade, market_price);
        assert!(!result.is_best_execution);
        // (66000 - 65000) / 65000 * 100 ≈ 1.538%
        assert!(result.deviation_pct > 1.0);
    }

    #[test]
    fn test_best_execution_zero_market_price() {
        let reporter = test_reporter();
        let trade = sample_trade();
        let result = reporter.best_execution_check(&trade, 0.0);
        // deviation should be 0 when market price is 0 (avoid division by zero)
        assert_eq!(result.deviation_pct, 0.0);
        assert!(result.is_best_execution);
    }
}
