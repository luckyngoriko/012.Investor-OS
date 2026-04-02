//! MiCA (Markets in Crypto-Assets Regulation) Compliance
//!
//! Implements key MiCA requirements:
//! - Crypto-asset classification (ART / EMT / Other)
//! - Whitepaper requirement determination
//! - Reserve requirements per asset category

use serde::{Deserialize, Serialize};

// ─── Types ───────────────────────────────────────────────────────────────────

/// Crypto-asset category under MiCA regulation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoAssetCategory {
    /// Asset-Referenced Token — pegged to a basket of assets (e.g. stablecoins referencing multiple currencies)
    Art,
    /// E-Money Token — pegged to a single official currency (e.g. USDC, USDT, EURC)
    Emt,
    /// Other crypto-assets — utility tokens, payment tokens, pure cryptocurrencies
    Other,
}

impl std::fmt::Display for CryptoAssetCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoAssetCategory::Art => write!(f, "Asset-Referenced Token (ART)"),
            CryptoAssetCategory::Emt => write!(f, "E-Money Token (EMT)"),
            CryptoAssetCategory::Other => write!(f, "Other Crypto-Asset"),
        }
    }
}

/// Reserve requirements for a given crypto-asset category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveInfo {
    pub category: CryptoAssetCategory,
    /// Whether a reserve of assets is required
    pub reserve_required: bool,
    /// Minimum reserve ratio (1.0 = 100% backing)
    pub min_reserve_ratio: f64,
    /// Whether segregation of reserve assets is mandatory
    pub segregation_required: bool,
    /// Description of the reserve requirements
    pub description: String,
}

/// Classification result for a crypto asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub symbol: String,
    pub category: CryptoAssetCategory,
    pub whitepaper_required: bool,
    pub reserve_info: ReserveInfo,
}

// ─── MicaClassifier ──────────────────────────────────────────────────────────

/// MiCA regulation classifier
#[derive(Debug, Clone)]
pub struct MicaClassifier;

impl MicaClassifier {
    pub fn new() -> Self {
        Self
    }

    /// Classify a crypto asset by symbol into the appropriate MiCA category
    ///
    /// Classification rules:
    /// - EMT: tokens pegged to a single fiat currency (USDT, USDC, BUSD, TUSD, DAI, EURC, EURT)
    /// - ART: tokens pegged to a basket of assets (PAXG = gold-backed)
    /// - Other: all other crypto-assets (BTC, ETH, SOL, etc.)
    pub fn classify_asset(&self, symbol: &str) -> CryptoAssetCategory {
        let upper = symbol.to_uppercase();

        // Strip trading pair suffixes for classification
        let base = strip_pair_suffix(&upper);

        // EMT: single-currency stablecoins
        if matches!(
            base.as_str(),
            "USDT" | "USDC" | "BUSD" | "TUSD" | "DAI" | "EURC" | "EURT" | "FDUSD" | "PYUSD"
        ) {
            return CryptoAssetCategory::Emt;
        }

        // ART: multi-asset-referenced tokens
        if matches!(base.as_str(), "PAXG" | "XAUT" | "DIEM" | "LIBRA") {
            return CryptoAssetCategory::Art;
        }

        // Everything else (BTC, ETH, SOL, BNB, XRP, ADA, DOGE, AVAX, etc.)
        CryptoAssetCategory::Other
    }

    /// Determine whether a crypto-asset whitepaper is required under MiCA
    ///
    /// All categories require a whitepaper except for:
    /// - Assets already listed and traded before MiCA entry into force (grandfathering)
    /// - Very small offerings (< EUR 1M in 12 months) — not modelled here
    pub fn whitepaper_required(&self, category: CryptoAssetCategory) -> bool {
        // Under MiCA, ALL categories require a whitepaper for new issuance
        match category {
            CryptoAssetCategory::Art => true,
            CryptoAssetCategory::Emt => true,
            CryptoAssetCategory::Other => true,
        }
    }

    /// Get reserve requirements for a given crypto-asset category
    pub fn reserve_requirements(&self, category: CryptoAssetCategory) -> ReserveInfo {
        match category {
            CryptoAssetCategory::Art => ReserveInfo {
                category,
                reserve_required: true,
                min_reserve_ratio: 1.0,
                segregation_required: true,
                description: "ART issuers must maintain a reserve of assets equal to \
                    100% of the outstanding token value. Reserve assets must be \
                    segregated and held with a custodian. Regular audits required."
                    .to_string(),
            },
            CryptoAssetCategory::Emt => ReserveInfo {
                category,
                reserve_required: true,
                min_reserve_ratio: 1.0,
                segregation_required: true,
                description: "EMT issuers must maintain reserves in the referenced \
                    currency at 100% backing. Reserves must be placed with credit \
                    institutions. Only authorized credit institutions or e-money \
                    institutions may issue EMTs."
                    .to_string(),
            },
            CryptoAssetCategory::Other => ReserveInfo {
                category,
                reserve_required: false,
                min_reserve_ratio: 0.0,
                segregation_required: false,
                description: "Other crypto-assets do not have specific reserve \
                    requirements under MiCA, but issuers must publish a \
                    crypto-asset whitepaper and comply with conduct rules."
                    .to_string(),
            },
        }
    }

    /// Full classification: category + whitepaper + reserves in one call
    pub fn full_classification(&self, symbol: &str) -> ClassificationResult {
        let category = self.classify_asset(symbol);
        let whitepaper_required = self.whitepaper_required(category);
        let reserve_info = self.reserve_requirements(category);
        ClassificationResult {
            symbol: symbol.to_string(),
            category,
            whitepaper_required,
            reserve_info,
        }
    }
}

impl Default for MicaClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Strip common trading pair suffixes to get the base asset symbol
fn strip_pair_suffix(symbol: &str) -> String {
    let suffixes = ["USDT", "BUSD", "USD", "EUR", "BTC", "ETH", "BNB"];
    // Only strip if the result is non-empty and the symbol is long enough
    for suffix in &suffixes {
        if symbol.len() > suffix.len() && symbol.ends_with(suffix) {
            let base = &symbol[..symbol.len() - suffix.len()];
            if !base.is_empty() {
                return base.to_string();
            }
        }
    }
    symbol.to_string()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn classifier() -> MicaClassifier {
        MicaClassifier::new()
    }

    // ── Asset classification tests ──

    #[test]
    fn test_classify_btc_as_other() {
        let c = classifier();
        assert_eq!(c.classify_asset("BTC"), CryptoAssetCategory::Other);
    }

    #[test]
    fn test_classify_btcusdt_as_other() {
        let c = classifier();
        // "BTCUSDT" → strip "USDT" → base "BTC" → Other
        assert_eq!(c.classify_asset("BTCUSDT"), CryptoAssetCategory::Other);
    }

    #[test]
    fn test_classify_eth_as_other() {
        let c = classifier();
        assert_eq!(c.classify_asset("ETH"), CryptoAssetCategory::Other);
    }

    #[test]
    fn test_classify_usdt_as_emt() {
        let c = classifier();
        assert_eq!(c.classify_asset("USDT"), CryptoAssetCategory::Emt);
    }

    #[test]
    fn test_classify_usdc_as_emt() {
        let c = classifier();
        assert_eq!(c.classify_asset("USDC"), CryptoAssetCategory::Emt);
    }

    #[test]
    fn test_classify_dai_as_emt() {
        let c = classifier();
        assert_eq!(c.classify_asset("DAI"), CryptoAssetCategory::Emt);
    }

    #[test]
    fn test_classify_paxg_as_art() {
        let c = classifier();
        assert_eq!(c.classify_asset("PAXG"), CryptoAssetCategory::Art);
    }

    #[test]
    fn test_classify_case_insensitive() {
        let c = classifier();
        assert_eq!(c.classify_asset("btc"), CryptoAssetCategory::Other);
        assert_eq!(c.classify_asset("usdt"), CryptoAssetCategory::Emt);
        assert_eq!(c.classify_asset("paxg"), CryptoAssetCategory::Art);
    }

    // ── Whitepaper tests ──

    #[test]
    fn test_whitepaper_required_all() {
        let c = classifier();
        assert!(c.whitepaper_required(CryptoAssetCategory::Art));
        assert!(c.whitepaper_required(CryptoAssetCategory::Emt));
        assert!(c.whitepaper_required(CryptoAssetCategory::Other));
    }

    // ── Reserve requirements tests ──

    #[test]
    fn test_reserve_art_100_pct() {
        let c = classifier();
        let info = c.reserve_requirements(CryptoAssetCategory::Art);
        assert!(info.reserve_required);
        assert_eq!(info.min_reserve_ratio, 1.0);
        assert!(info.segregation_required);
    }

    #[test]
    fn test_reserve_emt_100_pct() {
        let c = classifier();
        let info = c.reserve_requirements(CryptoAssetCategory::Emt);
        assert!(info.reserve_required);
        assert_eq!(info.min_reserve_ratio, 1.0);
        assert!(info.segregation_required);
    }

    #[test]
    fn test_reserve_other_none() {
        let c = classifier();
        let info = c.reserve_requirements(CryptoAssetCategory::Other);
        assert!(!info.reserve_required);
        assert_eq!(info.min_reserve_ratio, 0.0);
        assert!(!info.segregation_required);
    }

    // ── Full classification tests ──

    #[test]
    fn test_full_classification_btc() {
        let c = classifier();
        let result = c.full_classification("BTCUSDT");
        assert_eq!(result.category, CryptoAssetCategory::Other);
        assert!(result.whitepaper_required);
        assert!(!result.reserve_info.reserve_required);
    }

    #[test]
    fn test_full_classification_usdt() {
        let c = classifier();
        let result = c.full_classification("USDT");
        assert_eq!(result.category, CryptoAssetCategory::Emt);
        assert!(result.whitepaper_required);
        assert!(result.reserve_info.reserve_required);
        assert_eq!(result.reserve_info.min_reserve_ratio, 1.0);
    }

    // ── Display tests ──

    #[test]
    fn test_category_display() {
        assert_eq!(
            format!("{}", CryptoAssetCategory::Art),
            "Asset-Referenced Token (ART)"
        );
        assert_eq!(
            format!("{}", CryptoAssetCategory::Emt),
            "E-Money Token (EMT)"
        );
        assert_eq!(
            format!("{}", CryptoAssetCategory::Other),
            "Other Crypto-Asset"
        );
    }

    // ── Strip pair suffix tests ──

    #[test]
    fn test_strip_pair_suffix() {
        assert_eq!(strip_pair_suffix("BTCUSDT"), "BTC");
        assert_eq!(strip_pair_suffix("ETHBUSD"), "ETH");
        assert_eq!(strip_pair_suffix("BTC"), "BTC");
        assert_eq!(strip_pair_suffix("SOLUSDT"), "SOL");
    }
}
