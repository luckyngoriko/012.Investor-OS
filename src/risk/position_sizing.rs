//! Position sizing algorithms

use rust_decimal::Decimal;
use tracing::{debug, info};

use super::{RiskError, Result};

/// Position sizing method
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum SizingMethod {
    /// Fixed fractional sizing (risk fixed % of capital per trade)
    #[default]
    FixedFractional,
    /// Kelly Criterion for optimal growth
    KellyCriterion,
    /// Volatility-based sizing (ATR method)
    VolatilityBased,
    /// Fixed dollar amount per position
    FixedAmount,
}


/// Position sizing configuration
#[derive(Debug, Clone)]
pub struct SizingConfig {
    /// Sizing method to use
    pub method: SizingMethod,
    /// Risk percentage per trade (for fixed fractional)
    pub risk_percent: Decimal,
    /// Kelly fraction (0.5 = half Kelly for safety)
    pub kelly_fraction: Decimal,
    /// ATR multiplier for volatility sizing
    pub atr_multiplier: Decimal,
    /// Fixed position size (for fixed amount)
    pub fixed_size: Decimal,
    /// Maximum position size in base currency
    pub max_position_size: Decimal,
    /// Minimum position size
    pub min_position_size: Decimal,
}

impl Default for SizingConfig {
    fn default() -> Self {
        Self {
            method: SizingMethod::FixedFractional,
            risk_percent: Decimal::try_from(0.01).unwrap(), // 1% risk per trade
            kelly_fraction: Decimal::try_from(0.25).unwrap(), // Quarter Kelly
            atr_multiplier: Decimal::from(2),
            fixed_size: Decimal::from(1000),
            max_position_size: Decimal::from(100000),
            min_position_size: Decimal::from(10),
        }
    }
}

/// Position sizer calculates optimal position sizes
#[derive(Debug, Clone)]
pub struct PositionSizer {
    config: SizingConfig,
}

impl PositionSizer {
    /// Create a new position sizer with the given config
    pub fn new(config: SizingConfig) -> Self {
        Self { config }
    }

    /// Calculate position size for a trade
    /// 
    /// # Arguments
    /// * `available_capital` - Total available capital
    /// * `entry_price` - Entry price for the trade
    /// * `stop_loss` - Stop loss price (for risk calculation)
    /// * `volatility` - Optional volatility metric (ATR)
    /// * `win_rate` - Optional win rate for Kelly criterion
    /// * `avg_win_loss_ratio` - Optional avg win/loss ratio
    pub fn calculate_size(
        &self,
        available_capital: Decimal,
        entry_price: Decimal,
        stop_loss: Option<Decimal>,
        volatility: Option<Decimal>,
        win_rate: Option<Decimal>,
        avg_win_loss_ratio: Option<Decimal>,
    ) -> Result<Decimal> {
        let size = match self.config.method {
            SizingMethod::FixedFractional => {
                self.fixed_fractional_size(available_capital, entry_price, stop_loss)
            }
            SizingMethod::KellyCriterion => {
                self.kelly_size(available_capital, win_rate, avg_win_loss_ratio)
            }
            SizingMethod::VolatilityBased => {
                self.volatility_based_size(available_capital, entry_price, volatility)
            }
            SizingMethod::FixedAmount => {
                self.fixed_amount_size(entry_price)
            }
        }?;

        // Apply min/max constraints
        let constrained = size
            .min(self.config.max_position_size)
            .max(self.config.min_position_size);

        debug!(
            "Position sizing: method={:?}, raw_size={}, constrained={}",
            self.config.method, size, constrained
        );

        Ok(constrained)
    }

    /// Fixed fractional sizing: risk fixed % of capital per trade
    /// 
    /// Formula: position_size = (capital * risk_percent) / |entry - stop_loss|
    fn fixed_fractional_size(
        &self,
        capital: Decimal,
        entry_price: Decimal,
        stop_loss: Option<Decimal>,
    ) -> Result<Decimal> {
        let stop = stop_loss.ok_or_else(|| {
            RiskError::CalculationError("Stop loss required for fixed fractional sizing".to_string())
        })?;

        if stop.is_zero() || stop == entry_price {
            return Err(RiskError::CalculationError(
                "Invalid stop loss price".to_string(),
            ));
        }

        let risk_amount = capital * self.config.risk_percent;
        let price_risk = (entry_price - stop).abs();

        let position_value = risk_amount / price_risk * entry_price;
        let shares = position_value / entry_price;

        info!(
            "Fixed fractional: capital={}, risk={}%, risk_amount={}, position_value={}",
            capital, self.config.risk_percent * Decimal::from(100), risk_amount, position_value
        );

        Ok(shares)
    }

    /// Kelly Criterion sizing for optimal growth
    /// 
    /// Formula: f* = (p * b - q) / b
    /// where p = win rate, q = loss rate (1-p), b = avg win / avg loss
    fn kelly_size(
        &self,
        capital: Decimal,
        win_rate: Option<Decimal>,
        avg_win_loss_ratio: Option<Decimal>,
    ) -> Result<Decimal> {
        let p = win_rate.ok_or_else(|| {
            RiskError::CalculationError("Win rate required for Kelly sizing".to_string())
        })?;

        let b = avg_win_loss_ratio.ok_or_else(|| {
            RiskError::CalculationError("Win/loss ratio required for Kelly sizing".to_string())
        })?;

        if p <= Decimal::ZERO || p >= Decimal::ONE {
            return Err(RiskError::CalculationError(
                "Win rate must be between 0 and 1".to_string(),
            ));
        }

        if b <= Decimal::ZERO {
            return Err(RiskError::CalculationError(
                "Win/loss ratio must be positive".to_string(),
            ));
        }

        let q = Decimal::ONE - p; // Loss rate

        // Kelly fraction: (p * b - q) / b
        let kelly = (p * b - q) / b;

        // Apply Kelly fraction (e.g., half Kelly for safety)
        let adjusted_kelly = kelly * self.config.kelly_fraction;

        // Ensure non-negative
        let final_kelly = adjusted_kelly.max(Decimal::ZERO);

        let position_size = capital * final_kelly;

        info!(
            "Kelly sizing: raw_kelly={}%, adjusted_kelly={}%, position_size={}",
            kelly * Decimal::from(100),
            final_kelly * Decimal::from(100),
            position_size
        );

        Ok(position_size)
    }

    /// Volatility-based sizing using ATR
    /// 
    /// Formula: position_size = (capital * risk_percent) / (ATR * multiplier)
    fn volatility_based_size(
        &self,
        capital: Decimal,
        entry_price: Decimal,
        volatility: Option<Decimal>,
    ) -> Result<Decimal> {
        let atr = volatility.ok_or_else(|| {
            RiskError::CalculationError("ATR required for volatility-based sizing".to_string())
        })?;

        if atr.is_zero() {
            return Err(RiskError::InvalidVolatility);
        }

        let risk_amount = capital * self.config.risk_percent;
        let volatility_risk = atr * self.config.atr_multiplier;

        let position_value = risk_amount / volatility_risk * entry_price;
        let shares = position_value / entry_price;

        info!(
            "Volatility sizing: ATR={}, multiplier={}, shares={}",
            atr, self.config.atr_multiplier, shares
        );

        Ok(shares)
    }

    /// Fixed amount sizing
    fn fixed_amount_size(&self, entry_price: Decimal) -> Result<Decimal> {
        if entry_price.is_zero() {
            return Err(RiskError::CalculationError(
                "Entry price cannot be zero".to_string(),
            ));
        }
        Ok(self.config.fixed_size / entry_price)
    }

    /// Calculate maximum position size based on available margin
    pub fn max_size_from_margin(
        available_margin: Decimal,
        entry_price: Decimal,
        leverage: Decimal,
    ) -> Result<Decimal> {
        if entry_price.is_zero() {
            return Err(RiskError::CalculationError(
                "Entry price cannot be zero".to_string(),
            ));
        }

        let max_position_value = available_margin * leverage;
        let max_shares = max_position_value / entry_price;

        Ok(max_shares)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_fractional_sizing() {
        let config = SizingConfig::default();
        let sizer = PositionSizer::new(config);

        let capital = Decimal::from(100000);
        let entry = Decimal::from(100);
        let stop = Decimal::from(98); // 2% stop

        let size = sizer
            .calculate_size(capital, entry, Some(stop), None, None, None)
            .unwrap();

        // Risk = 1% of 100k = 1000
        // Price risk = 100 - 98 = 2
        // Position value = 1000 / 2 * 100 = 50,000
        // Shares = 50,000 / 100 = 500
        assert_eq!(size, Decimal::from(500));
    }

    #[test]
    fn test_kelly_criterion() {
        let config = SizingConfig {
            method: SizingMethod::KellyCriterion,
            kelly_fraction: Decimal::try_from(0.5).unwrap(), // Half Kelly
            ..Default::default()
        };
        let sizer = PositionSizer::new(config);

        let capital = Decimal::from(100000);
        let win_rate = Decimal::try_from(0.55).unwrap(); // 55% win rate
        let win_loss_ratio = Decimal::try_from(2.0).unwrap(); // 2:1 reward/risk

        let size = sizer
            .calculate_size(capital, Decimal::ONE, None, None, Some(win_rate), Some(win_loss_ratio))
            .unwrap();

        // Kelly = (0.55 * 2 - 0.45) / 2 = 0.325
        // Half Kelly = 0.1625
        // Position = 100,000 * 0.1625 = 16,250
        assert!(size > Decimal::ZERO);
        assert!(size < capital);
    }

    #[test]
    fn test_volatility_based_sizing() {
        let config = SizingConfig {
            method: SizingMethod::VolatilityBased,
            atr_multiplier: Decimal::from(2),
            ..Default::default()
        };
        let sizer = PositionSizer::new(config);

        let capital = Decimal::from(100000);
        let entry = Decimal::from(100);
        let atr = Decimal::try_from(2.5).unwrap(); // ATR = $2.50

        let size = sizer
            .calculate_size(capital, entry, None, Some(atr), None, None)
            .unwrap();

        // Risk = 1% of 100k = 1000
        // Volatility risk = 2.5 * 2 = 5
        // Position value = 1000 / 5 * 100 = 20,000
        // Shares = 20,000 / 100 = 200
        assert!(size > Decimal::ZERO);
    }

    #[test]
    fn test_max_position_limits() {
        let config = SizingConfig {
            method: SizingMethod::FixedFractional,
            risk_percent: Decimal::try_from(0.50).unwrap(), // 50% risk (high)
            max_position_size: Decimal::from(1000),
            ..Default::default()
        };
        let sizer = PositionSizer::new(config);

        let capital = Decimal::from(100000);
        let entry = Decimal::from(100);
        let stop = Decimal::from(99);

        let size = sizer
            .calculate_size(capital, entry, Some(stop), None, None, None)
            .unwrap();

        // Should be capped at max_position_size
        assert_eq!(size, Decimal::from(1000));
    }

    #[test]
    fn test_margin_based_max_size() {
        let margin = Decimal::from(10000);
        let entry = Decimal::from(100);
        let leverage = Decimal::from(5);

        let max_size = PositionSizer::max_size_from_margin(margin, entry, leverage).unwrap();

        // Max position value = 10,000 * 5 = 50,000
        // Max shares = 50,000 / 100 = 500
        assert_eq!(max_size, Decimal::from(500));
    }

    #[test]
    fn test_fixed_amount_sizing() {
        let config = SizingConfig {
            method: SizingMethod::FixedAmount,
            fixed_size: Decimal::from(5000),
            ..Default::default()
        };
        let sizer = PositionSizer::new(config);

        let entry = Decimal::from(100);
        let size = sizer
            .calculate_size(Decimal::ZERO, entry, None, None, None, None)
            .unwrap();

        // Shares = 5000 / 100 = 50
        assert_eq!(size, Decimal::from(50));
    }
}
