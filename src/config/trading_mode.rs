//! Trading Mode Configuration
//!
//! Defines the three trading modes:
//! - Manual: Human makes all decisions, AI provides analysis only
//! - SemiAuto: AI proposes, human confirms/rejects, AI executes confirmed
//! - FullyAuto: AI proposes and executes automatically within risk limits

use serde::{Deserialize, Serialize};
use std::fmt;

/// Trading mode enum - controls how autonomous the AI is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TradingMode {
    /// Manual mode: AI analyzes but never executes
    /// - AI generates proposals with CQ scores
    /// - Human must manually enter all trades
    /// - Best for: Learning, testing strategies, maximum control
    Manual,
    
    /// Semi-automatic mode: AI proposes, human confirms, AI executes
    /// - AI generates proposals with CQ scores
    /// - Human reviews and confirms/rejects each proposal
    /// - AI automatically executes confirmed proposals
    /// - Best for: Most users, balance of control and convenience
    #[default]
    SemiAuto,
    
    /// Fully automatic mode: AI proposes and executes within limits
    /// - AI generates proposals with CQ scores
    /// - Auto-executes if CQ >= threshold and within risk limits
    /// - Human notified of all executed trades
    /// - Best for: Experienced users, hands-off approach
    FullyAuto,
}

impl TradingMode {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            TradingMode::Manual => "Manual",
            TradingMode::SemiAuto => "Semi-Auto",
            TradingMode::FullyAuto => "Fully Auto",
        }
    }
    
    /// Get description for UI
    pub fn description(&self) -> &'static str {
        match self {
            TradingMode::Manual => "You make all trading decisions. AI provides analysis only.",
            TradingMode::SemiAuto => "AI proposes trades, you confirm. AI executes confirmed trades.",
            TradingMode::FullyAuto => "AI trades automatically within your risk limits.",
        }
    }
    
    /// Get detailed description
    pub fn detailed_description(&self) -> &'static str {
        match self {
            TradingMode::Manual => {
                "In Manual mode:\n\
                • AI analyzes market data and generates proposals\n\
                • You review proposals with CQ scores and rationales\n\
                • You manually execute trades through your broker\n\
                • AI tracks performance but never executes\n\
                • Best for learning and maximum control"
            }
            TradingMode::SemiAuto => {
                "In Semi-Auto mode:\n\
                • AI analyzes and generates trade proposals\n\
                • You receive notifications for new proposals\n\
                • You confirm or reject each proposal\n\
                • AI automatically executes confirmed trades\n\
                • Best balance of AI power and human oversight"
            }
            TradingMode::FullyAuto => {
                "In Fully Auto mode:\n\
                • AI continuously analyzes market conditions\n\
                • Auto-executes trades when CQ >= threshold\n\
                • Respects all risk limits (position size, VaR, etc.)\n\
                • You receive notifications of all executed trades\n\
                • Kill switch available for emergencies"
            }
        }
    }
    
    /// Check if AI can auto-execute trades
    pub fn can_auto_execute(&self) -> bool {
        matches!(self, TradingMode::SemiAuto | TradingMode::FullyAuto)
    }
    
    /// Check if human confirmation is required
    pub fn requires_human_confirmation(&self) -> bool {
        matches!(self, TradingMode::Manual | TradingMode::SemiAuto)
    }
    
    /// Check if AI can execute without confirmation
    pub fn can_execute_without_confirmation(&self) -> bool {
        matches!(self, TradingMode::FullyAuto)
    }
    
    /// Get icon name for UI
    pub fn icon(&self) -> &'static str {
        match self {
            TradingMode::Manual => "user",
            TradingMode::SemiAuto => "user-cog",
            TradingMode::FullyAuto => "robot",
        }
    }
    
    /// Get color for UI
    pub fn color(&self) -> &'static str {
        match self {
            TradingMode::Manual => "blue",
            TradingMode::SemiAuto => "amber",
            TradingMode::FullyAuto => "emerald",
        }
    }
    
    /// Get security level (1-3, higher is more autonomous)
    pub fn autonomy_level(&self) -> u8 {
        match self {
            TradingMode::Manual => 1,
            TradingMode::SemiAuto => 2,
            TradingMode::FullyAuto => 3,
        }
    }
    
    /// Get recommended for text
    pub fn recommended_for(&self) -> &'static str {
        match self {
            TradingMode::Manual => "Beginners, learning, maximum control",
            TradingMode::SemiAuto => "Most users, balanced approach",
            TradingMode::FullyAuto => "Experienced users, hands-off trading",
        }
    }
}


impl fmt::Display for TradingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Trading mode configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TradingModeConfig {
    /// Current trading mode
    pub mode: TradingMode,
    /// Auto-execution threshold (CQ score 0-100)
    /// Only used in FullyAuto mode
    pub auto_execute_cq_threshold: u8,
    /// Maximum single trade value for auto-execution
    /// Trades above this require manual confirmation even in FullyAuto
    pub max_auto_trade_value: f64,
    /// Notification settings
    pub notifications: ModeNotifications,
}

impl Default for TradingModeConfig {
    fn default() -> Self {
        Self {
            mode: TradingMode::default(),
            auto_execute_cq_threshold: 80, // High threshold for safety
            max_auto_trade_value: 10000.0, // $10k max per auto trade
            notifications: ModeNotifications::default(),
        }
    }
}

/// Notification settings for each mode
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModeNotifications {
    /// Notify on new proposals
    pub on_proposal: bool,
    /// Notify on executed trades
    pub on_execution: bool,
    /// Notify on rejections
    pub on_rejection: bool,
    /// Notify on risk alerts
    pub on_risk_alert: bool,
    /// Email notifications enabled
    pub email_enabled: bool,
    /// Push notifications enabled
    pub push_enabled: bool,
}

impl Default for ModeNotifications {
    fn default() -> Self {
        Self {
            on_proposal: true,
            on_execution: true,
            on_rejection: false,
            on_risk_alert: true,
            email_enabled: true,
            push_enabled: true,
        }
    }
}

impl TradingModeConfig {
    /// Create new config with specific mode
    pub fn new(mode: TradingMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }
    
    /// Check if a trade can be auto-executed
    pub fn can_auto_execute(&self, cq_score: u8, trade_value: f64) -> bool {
        if !self.mode.can_auto_execute() {
            return false;
        }
        
        cq_score >= self.auto_execute_cq_threshold 
            && trade_value <= self.max_auto_trade_value
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.auto_execute_cq_threshold > 100 {
            return Err("CQ threshold must be between 0 and 100".to_string());
        }
        
        if self.max_auto_trade_value <= 0.0 {
            return Err("Max auto trade value must be positive".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trading_mode_properties() {
        assert!(!TradingMode::Manual.can_auto_execute());
        assert!(TradingMode::SemiAuto.can_auto_execute());
        assert!(TradingMode::FullyAuto.can_auto_execute());
        
        assert!(TradingMode::Manual.requires_human_confirmation());
        assert!(TradingMode::SemiAuto.requires_human_confirmation());
        assert!(!TradingMode::FullyAuto.requires_human_confirmation());
        
        assert!(!TradingMode::Manual.can_execute_without_confirmation());
        assert!(!TradingMode::SemiAuto.can_execute_without_confirmation());
        assert!(TradingMode::FullyAuto.can_execute_without_confirmation());
    }
    
    #[test]
    fn test_config_can_auto_execute() {
        let config = TradingModeConfig::new(TradingMode::FullyAuto);
        
        // High CQ, low value - should execute
        assert!(config.can_auto_execute(85, 5000.0));
        
        // Low CQ - should not execute
        assert!(!config.can_auto_execute(70, 5000.0));
        
        // High value - should not execute
        assert!(!config.can_auto_execute(85, 15000.0));
        
        // Manual mode - should never execute
        let manual_config = TradingModeConfig::new(TradingMode::Manual);
        assert!(!manual_config.can_auto_execute(100, 100.0));
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = TradingModeConfig::default();
        assert!(config.validate().is_ok());
        
        config.auto_execute_cq_threshold = 150;
        assert!(config.validate().is_err());
        
        config.auto_execute_cq_threshold = 80;
        config.max_auto_trade_value = -100.0;
        assert!(config.validate().is_err());
    }
}
