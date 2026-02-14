//! FX Converter - Currency conversion with best rates

use super::*;

/// FX rate with metadata
#[derive(Debug, Clone)]
pub struct FxRate {
    pub from: Currency,
    pub to: Currency,
    pub rate: Decimal,
    pub spread_bps: Decimal, // Spread in basis points
    pub timestamp: DateTime<Utc>,
    pub source: RateSource,
}

/// Source of FX rate
#[derive(Debug, Clone)]
pub enum RateSource {
    Market,           // Spot market
    Bank,             // Bank rate
    Provider(String), // Third party (e.g., OANDA)
}

/// FX Converter
#[derive(Debug)]
pub struct FxConverter {
    rates: HashMap<(Currency, Currency), FxRate>,
    // Rate providers
    // Cache
}

impl FxConverter {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            rates: HashMap::new(),
        })
    }
    
    /// Get current FX rate
    pub async fn get_rate(&self, from: Currency, to: Currency) -> Result<Decimal> {
        if from == to {
            return Ok(Decimal::ONE);
        }
        
        // Check cache first
        if let Some(rate) = self.rates.get(&(from, to)) {
            // Check if rate is fresh (< 5 minutes)
            let age = Utc::now() - rate.timestamp;
            if age.num_seconds() < 300 {
                return Ok(rate.rate);
            }
        }
        
        // TODO: Fetch from provider (OANDA, Bloomberg, etc.)
        // Mock rates for now
        let rate = self.get_mock_rate(from, to);
        
        Ok(rate)
    }
    
    /// Convert amount with spread
    pub async fn convert(
        &self,
        from: Currency,
        to: Currency,
        amount: Decimal,
        rate: Decimal,
    ) -> Result<(Decimal, Decimal)> {
        // Calculate spread (in basis points)
        let spread_bps = self.calculate_spread(from, to);
        let spread_multiplier = Decimal::ONE - (spread_bps / Decimal::from(10000));
        
        // Apply rate with spread
        let converted = amount * rate * spread_multiplier;
        
        Ok((converted, spread_bps))
    }
    
    /// Calculate conversion cost
    pub fn calculate_cost(&self, from: Currency, to: Currency, amount: Decimal) -> Decimal {
        // Typical FX conversion cost: 0.1% - 0.5%
        let base_cost = amount * "0.001".parse::<Decimal>().unwrap(); // 0.1%
        
        // Higher cost for exotic currencies
        let multiplier = match (from, to) {
            (Currency::USD, Currency::EUR) => Decimal::ONE,
            (Currency::USD, Currency::JPY) => Decimal::ONE,
            _ => "1.5".parse::<Decimal>().unwrap(), // 50% more for exotics
        };
        
        base_cost * multiplier
    }
    
    /// Update rates from market data
    pub async fn refresh_rates(&mut self) -> Result<()> {
        // TODO: Fetch all rates from providers
        // Mock: populate with sample rates
        self.rates.insert(
            (Currency::USD, Currency::EUR),
            FxRate {
                from: Currency::USD,
                to: Currency::EUR,
                rate: "0.92".parse::<Decimal>().unwrap(),
                spread_bps: Decimal::from(10), // 0.1%
                timestamp: Utc::now(),
                source: RateSource::Market,
            },
        );
        
        self.rates.insert(
            (Currency::EUR, Currency::USD),
            FxRate {
                from: Currency::EUR,
                to: Currency::USD,
                rate: "1.09".parse::<Decimal>().unwrap(),
                spread_bps: Decimal::from(10),
                timestamp: Utc::now(),
                source: RateSource::Market,
            },
        );
        
        self.rates.insert(
            (Currency::USD, Currency::GBP),
            FxRate {
                from: Currency::USD,
                to: Currency::GBP,
                rate: "0.79".parse::<Decimal>().unwrap(),
                spread_bps: Decimal::from(15),
                timestamp: Utc::now(),
                source: RateSource::Market,
            },
        );
        
        Ok(())
    }
    
    // Private helpers
    
    fn get_mock_rate(&self, from: Currency, to: Currency) -> Decimal {
        // Mock FX rates
        match (from, to) {
            (Currency::USD, Currency::EUR) => "0.92".parse::<Decimal>().unwrap(),
            (Currency::EUR, Currency::USD) => "1.09".parse::<Decimal>().unwrap(),
            (Currency::USD, Currency::GBP) => "0.79".parse::<Decimal>().unwrap(),
            (Currency::GBP, Currency::USD) => "1.27".parse::<Decimal>().unwrap(),
            (Currency::USD, Currency::JPY) => "149.50".parse::<Decimal>().unwrap(),
            (Currency::USD, Currency::CHF) => "0.88".parse::<Decimal>().unwrap(),
            (Currency::EUR, Currency::GBP) => "0.86".parse::<Decimal>().unwrap(),
            (Currency::BTC, Currency::USD) => "50000.0".parse::<Decimal>().unwrap(),
            (Currency::ETH, Currency::USD) => "3000.0".parse::<Decimal>().unwrap(),
            _ => Decimal::ONE,
        }
    }
    
    fn calculate_spread(&self, from: Currency, to: Currency) -> Decimal {
        // Major pairs: 10 bps (0.1%)
        // Cross pairs: 20-50 bps
        match (from, to) {
            (Currency::USD, Currency::EUR) => Decimal::from(10),
            (Currency::USD, Currency::GBP) => Decimal::from(10),
            (Currency::USD, Currency::JPY) => Decimal::from(10),
            (Currency::EUR, Currency::GBP) => Decimal::from(15),
            _ => Decimal::from(50), // Exotics: 0.5%
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fx_rate_available() {
        let converter = FxConverter::new().await.unwrap();
        
        let rate = converter.get_rate(Currency::USD, Currency::EUR).await.unwrap();
        assert!(rate > Decimal::ZERO);
        assert!(rate < Decimal::from(2)); // Sanity check
    }
    
    #[tokio::test]
    async fn test_fx_conversion() {
        let converter = FxConverter::new().await.unwrap();
        
        let rate = converter.get_rate(Currency::USD, Currency::EUR).await.unwrap();
        let (converted, spread) = converter
            .convert(Currency::USD, Currency::EUR, Decimal::from(1000), rate)
            .await
            .unwrap();
        
        // With spread, we get less than raw rate
        assert!(converted < Decimal::from(1000) * rate);
        assert!(spread > Decimal::ZERO);
    }
    
    #[tokio::test]
    async fn test_same_currency_rate_is_one() {
        let converter = FxConverter::new().await.unwrap();
        
        let rate = converter.get_rate(Currency::USD, Currency::USD).await.unwrap();
        assert_eq!(rate, Decimal::ONE);
    }
}
