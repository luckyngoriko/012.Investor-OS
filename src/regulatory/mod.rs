//! Regulatory Compliance Module (Wave 4 Task 25)
//!
//! MiFID II and MiCA regulatory compliance for EU markets.
//! Separate from the `compliance` module (which is behind `eu_compliance` feature flag).
//!
//! - MiFID II: transaction reporting, best execution, client classification
//! - MiCA: crypto-asset classification, whitepaper requirements, reserve rules

pub mod mica;
pub mod mifid;

pub use mica::{ClassificationResult, CryptoAssetCategory, MicaClassifier, ReserveInfo};
pub use mifid::{
    BestExecutionResult, ClientCategory, ClientProfile, MifidReport, MifidReporter, TradeInput,
};
