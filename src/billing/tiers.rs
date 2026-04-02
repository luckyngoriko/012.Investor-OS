//! Subscription tier definitions and feature gating rules.
//!
//! Each tier gates: max brokers, max symbols, allowed ML models,
//! allowed trading modes, live-trading access, and monthly price.

use serde::Serialize;

/// Subscription tiers available in Investor OS.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Free tier — paper trading, 1 broker, 2 symbols, basic models.
    Free,
    /// Pro tier ($79/mo) — live trading, 3 brokers, 20 symbols, all 8 models.
    Pro,
    /// Enterprise tier ($299/mo) — unlimited brokers/symbols, custom models, full auto.
    Enterprise,
}

impl Tier {
    /// Parse a tier from a string slug. Defaults to [`Tier::Free`] for unknown values.
    pub fn from_str(s: &str) -> Self {
        match s {
            "pro" => Self::Pro,
            "enterprise" => Self::Enterprise,
            _ => Self::Free,
        }
    }

    /// Return the string slug for this tier.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }

    /// Maximum number of broker connections allowed.
    pub fn max_brokers(&self) -> usize {
        match self {
            Self::Free => 1,
            Self::Pro => 3,
            Self::Enterprise => usize::MAX,
        }
    }

    /// Maximum number of tracked symbols allowed.
    pub fn max_symbols(&self) -> usize {
        match self {
            Self::Free => 2,
            Self::Pro => 20,
            Self::Enterprise => usize::MAX,
        }
    }

    /// List of ML model slugs the tier has access to.
    pub fn allowed_models(&self) -> Vec<&'static str> {
        match self {
            Self::Free => vec!["catboost", "garch"],
            Self::Pro => vec![
                "catboost",
                "garch",
                "finbert",
                "chronos",
                "hrm",
                "ets",
                "skfolio",
                "consensus",
            ],
            Self::Enterprise => vec![
                "catboost",
                "garch",
                "finbert",
                "chronos",
                "hrm",
                "ets",
                "skfolio",
                "consensus",
                "custom",
            ],
        }
    }

    /// List of trading mode slugs the tier has access to.
    pub fn allowed_modes(&self) -> Vec<&'static str> {
        match self {
            Self::Free => vec!["signal"],
            Self::Pro => vec!["signal", "semi_auto"],
            Self::Enterprise => vec!["signal", "semi_auto", "full_auto"],
        }
    }

    /// Whether live (non-paper) trading is permitted.
    pub fn live_trading(&self) -> bool {
        !matches!(self, Self::Free)
    }

    /// Monthly price in whole USD.
    pub fn price_monthly_usd(&self) -> u32 {
        match self {
            Self::Free => 0,
            Self::Pro => 79,
            Self::Enterprise => 299,
        }
    }

    /// Check whether a specific model slug is allowed for this tier.
    pub fn can_use_model(&self, model: &str) -> bool {
        self.allowed_models().contains(&model)
    }

    /// Check whether a specific trading mode slug is allowed for this tier.
    pub fn can_use_mode(&self, mode: &str) -> bool {
        self.allowed_modes().contains(&mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_str() {
        assert_eq!(Tier::from_str("free"), Tier::Free);
        assert_eq!(Tier::from_str("pro"), Tier::Pro);
        assert_eq!(Tier::from_str("enterprise"), Tier::Enterprise);
        // Unknown defaults to Free
        assert_eq!(Tier::from_str("unknown"), Tier::Free);
        assert_eq!(Tier::from_str(""), Tier::Free);
    }

    #[test]
    fn test_tier_as_str_roundtrip() {
        for tier in [Tier::Free, Tier::Pro, Tier::Enterprise] {
            assert_eq!(Tier::from_str(tier.as_str()), tier);
        }
    }

    #[test]
    fn test_free_tier_limits() {
        let tier = Tier::Free;
        assert_eq!(tier.max_brokers(), 1);
        assert_eq!(tier.max_symbols(), 2);
        assert!(!tier.live_trading());
        assert_eq!(tier.price_monthly_usd(), 0);
    }

    #[test]
    fn test_pro_tier_limits() {
        let tier = Tier::Pro;
        assert_eq!(tier.max_brokers(), 3);
        assert_eq!(tier.max_symbols(), 20);
        assert!(tier.live_trading());
        assert_eq!(tier.price_monthly_usd(), 79);
    }

    #[test]
    fn test_enterprise_tier_limits() {
        let tier = Tier::Enterprise;
        assert_eq!(tier.max_brokers(), usize::MAX);
        assert_eq!(tier.max_symbols(), usize::MAX);
        assert!(tier.live_trading());
        assert_eq!(tier.price_monthly_usd(), 299);
    }

    #[test]
    fn test_free_allowed_models() {
        let tier = Tier::Free;
        let models = tier.allowed_models();
        assert_eq!(models.len(), 2);
        assert!(models.contains(&"catboost"));
        assert!(models.contains(&"garch"));
        assert!(!models.contains(&"finbert"));
        assert!(!models.contains(&"custom"));
    }

    #[test]
    fn test_pro_allowed_models() {
        let tier = Tier::Pro;
        let models = tier.allowed_models();
        assert_eq!(models.len(), 8);
        assert!(models.contains(&"catboost"));
        assert!(models.contains(&"consensus"));
        assert!(!models.contains(&"custom"));
    }

    #[test]
    fn test_enterprise_allowed_models() {
        let tier = Tier::Enterprise;
        let models = tier.allowed_models();
        assert_eq!(models.len(), 9);
        assert!(models.contains(&"custom"));
    }

    #[test]
    fn test_free_allowed_modes() {
        let tier = Tier::Free;
        let modes = tier.allowed_modes();
        assert_eq!(modes, vec!["signal"]);
    }

    #[test]
    fn test_pro_allowed_modes() {
        let tier = Tier::Pro;
        let modes = tier.allowed_modes();
        assert_eq!(modes, vec!["signal", "semi_auto"]);
        assert!(!modes.contains(&"full_auto"));
    }

    #[test]
    fn test_enterprise_allowed_modes() {
        let tier = Tier::Enterprise;
        let modes = tier.allowed_modes();
        assert_eq!(modes, vec!["signal", "semi_auto", "full_auto"]);
    }

    #[test]
    fn test_can_use_model() {
        assert!(Tier::Free.can_use_model("catboost"));
        assert!(!Tier::Free.can_use_model("finbert"));
        assert!(Tier::Pro.can_use_model("finbert"));
        assert!(!Tier::Pro.can_use_model("custom"));
        assert!(Tier::Enterprise.can_use_model("custom"));
    }

    #[test]
    fn test_can_use_mode() {
        assert!(Tier::Free.can_use_mode("signal"));
        assert!(!Tier::Free.can_use_mode("semi_auto"));
        assert!(Tier::Pro.can_use_mode("semi_auto"));
        assert!(!Tier::Pro.can_use_mode("full_auto"));
        assert!(Tier::Enterprise.can_use_mode("full_auto"));
    }

    #[test]
    fn test_tier_serialize() {
        let json = serde_json::to_string(&Tier::Pro).unwrap();
        assert_eq!(json, "\"pro\"");
        let json = serde_json::to_string(&Tier::Enterprise).unwrap();
        assert_eq!(json, "\"enterprise\"");
        let json = serde_json::to_string(&Tier::Free).unwrap();
        assert_eq!(json, "\"free\"");
    }
}
