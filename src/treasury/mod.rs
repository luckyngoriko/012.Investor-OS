//! Treasury Module - Sprint 15
//!
//! Управление на капитал: депозити, тегления, конверсии, yield

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

pub mod fiat;
pub mod crypto;
pub mod fx;
pub mod yield_optimizer;

pub use fiat::FiatGateway;
pub use crypto::CryptoCustody;
pub use fx::FxConverter;
pub use yield_optimizer::YieldOptimizer;

/// Errors that can occur in treasury operations
#[derive(Error, Debug)]
pub enum TreasuryError {
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: Decimal, available: Decimal },
    
    #[error("Currency not supported: {0}")]
    UnsupportedCurrency(String),
    
    #[error("Withdrawal limit exceeded: requested {requested}, limit {limit}")]
    WithdrawalLimitExceeded { requested: Decimal, limit: Decimal },
    
    #[error("Deposit not found: {0}")]
    DepositNotFound(Uuid),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Security check failed: {0}")]
    SecurityCheckFailed(String),
    
    #[error("FX rate not available for {from} -> {to}")]
    FxRateUnavailable { from: Currency, to: Currency },
    
    #[error("Gateway error: {0}")]
    GatewayError(String),
}

pub type Result<T> = std::result::Result<T, TreasuryError>;

/// Supported currencies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    // Fiat
    USD, EUR, GBP, CHF, JPY, CAD, AUD, 
    SGD, HKD, CNY, INR, BRL, MXN,
    
    // Crypto
    BTC, ETH, USDT, USDC, SOL, ADA, DOT, DAI,
    
    // Other
    XAU, // Gold
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Currency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::CHF => "CHF",
            Currency::JPY => "JPY",
            Currency::CAD => "CAD",
            Currency::AUD => "AUD",
            Currency::SGD => "SGD",
            Currency::HKD => "HKD",
            Currency::CNY => "CNY",
            Currency::INR => "INR",
            Currency::BRL => "BRL",
            Currency::MXN => "MXN",
            Currency::BTC => "BTC",
            Currency::ETH => "ETH",
            Currency::USDT => "USDT",
            Currency::USDC => "USDC",
            Currency::SOL => "SOL",
            Currency::ADA => "ADA",
            Currency::DOT => "DOT",
            Currency::DAI => "DAI",
            Currency::XAU => "XAU",
        }
    }
    
    pub fn is_fiat(&self) -> bool {
        matches!(self, 
            Currency::USD | Currency::EUR | Currency::GBP | 
            Currency::CHF | Currency::JPY | Currency::CAD | 
            Currency::AUD | Currency::SGD | Currency::HKD |
            Currency::CNY | Currency::INR | Currency::BRL | 
            Currency::MXN
        )
    }
    
    pub fn is_crypto(&self) -> bool {
        matches!(self,
            Currency::BTC | Currency::ETH | Currency::USDT |
            Currency::USDC | Currency::SOL | Currency::ADA |
            Currency::DOT | Currency::DAI
        )
    }
    
    pub fn decimals(&self) -> u32 {
        if self.is_fiat() {
            2
        } else {
            8 // Crypto default
        }
    }
}

impl std::str::FromStr for Currency {
    type Err = TreasuryError;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "USD" => Ok(Currency::USD),
            "EUR" => Ok(Currency::EUR),
            "GBP" => Ok(Currency::GBP),
            "CHF" => Ok(Currency::CHF),
            "JPY" => Ok(Currency::JPY),
            "CAD" => Ok(Currency::CAD),
            "AUD" => Ok(Currency::AUD),
            "SGD" => Ok(Currency::SGD),
            "HKD" => Ok(Currency::HKD),
            "CNY" => Ok(Currency::CNY),
            "INR" => Ok(Currency::INR),
            "BRL" => Ok(Currency::BRL),
            "MXN" => Ok(Currency::MXN),
            "BTC" => Ok(Currency::BTC),
            "ETH" => Ok(Currency::ETH),
            "USDT" => Ok(Currency::USDT),
            "USDC" => Ok(Currency::USDC),
            "SOL" => Ok(Currency::SOL),
            "ADA" => Ok(Currency::ADA),
            "DOT" => Ok(Currency::DOT),
            "DAI" => Ok(Currency::DAI),
            "XAU" => Ok(Currency::XAU),
            _ => Err(TreasuryError::UnsupportedCurrency(s.to_string())),
        }
    }
}

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,           // Чака обработка
    Processing,        // В процес
    Cleared,           // Завършено успешно
    Failed(String),    // Неуспешно
    Cancelled,         // Отказано
}

/// Deposit transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deposit {
    pub id: Uuid,
    pub currency: Currency,
    pub amount: Decimal,
    pub status: TransactionStatus,
    pub created_at: DateTime<Utc>,
    pub cleared_at: Option<DateTime<Utc>>,
    pub source: DepositSource,
    pub reference: Option<String>,
}

/// Source of deposit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepositSource {
    BankTransfer { bank_name: String, account_last4: String },
    CryptoWallet { address: String, chain: String },
    Card { last4: String },
    Internal { from_account: Uuid },
}

/// Withdrawal transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Withdrawal {
    pub id: Uuid,
    pub currency: Currency,
    pub amount: Decimal,
    pub status: TransactionStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub destination: WithdrawalDestination,
    pub fees: Decimal,
    pub reference: Option<String>,
}

/// Withdrawal destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WithdrawalDestination {
    BankAccount { bank_name: String, account_number: String },
    CryptoWallet { address: String, chain: String },
    Internal { to_account: Uuid },
}

/// FX conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxConversion {
    pub id: Uuid,
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub rate: Decimal,
    pub spread: Decimal, // In basis points
    pub created_at: DateTime<Utc>,
    pub status: TransactionStatus,
}

/// Account balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub currency: Currency,
    pub available: Decimal,      // Свободни за търговия
    pub locked: Decimal,         // Блокирани в поръчки
    pub pending_deposit: Decimal,// Очаквани депозити
    pub pending_withdrawal: Decimal,// Очаквани тегления
}

/// Treasury manager - основен интерфейс
#[derive(Debug)]
pub struct Treasury {
    balances: HashMap<Currency, Balance>,
    fiat_gateway: FiatGateway,
    crypto_custody: CryptoCustody,
    fx_converter: FxConverter,
    yield_optimizer: YieldOptimizer,
    
    // Limits
    daily_withdrawal_limit: Decimal,
    daily_withdrawal_used: Decimal,
    last_reset: DateTime<Utc>,
}

impl Treasury {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            balances: HashMap::new(),
            fiat_gateway: FiatGateway::new().await?,
            crypto_custody: CryptoCustody::new().await?,
            fx_converter: FxConverter::new().await?,
            yield_optimizer: YieldOptimizer::new().await?,
            daily_withdrawal_limit: Decimal::from(100000), // $100k default
            daily_withdrawal_used: Decimal::ZERO,
            last_reset: Utc::now(),
        })
    }
    
    /// Deposit fiat currency
    pub async fn deposit_fiat(&mut self, currency: Currency, amount: Decimal) -> Result<Deposit> {
        if !currency.is_fiat() {
            return Err(TreasuryError::UnsupportedCurrency(
                format!("{} is not a fiat currency", currency.as_str())
            ));
        }
        
        let deposit = self.fiat_gateway.initiate_deposit(currency, amount).await?;
        
        // Update pending balance
        let balance = self.balances.entry(currency).or_insert(Balance {
            currency,
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
            pending_deposit: Decimal::ZERO,
            pending_withdrawal: Decimal::ZERO,
        });
        balance.pending_deposit += amount;
        
        Ok(deposit)
    }
    
    /// Deposit crypto
    pub async fn deposit_crypto(&mut self, currency: Currency, amount: Decimal) -> Result<Deposit> {
        if !currency.is_crypto() {
            return Err(TreasuryError::UnsupportedCurrency(
                format!("{} is not a crypto currency", currency.as_str())
            ));
        }
        
        let deposit = self.crypto_custody.initiate_deposit(currency, amount).await?;
        
        let balance = self.balances.entry(currency).or_insert(Balance {
            currency,
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
            pending_deposit: Decimal::ZERO,
            pending_withdrawal: Decimal::ZERO,
        });
        balance.pending_deposit += amount;
        
        Ok(deposit)
    }
    
    /// Confirm deposit (when it clears)
    pub async fn confirm_deposit(&mut self, deposit_id: Uuid) -> Result<Deposit> {
        // Try fiat first, then crypto
        let deposit = if let Ok(d) = self.fiat_gateway.confirm_deposit(deposit_id).await {
            d
        } else {
            self.crypto_custody.confirm_deposit(deposit_id).await?
        };
        
        if let TransactionStatus::Cleared = deposit.status {
            let balance = self.balances.get_mut(&deposit.currency)
                .ok_or(TreasuryError::DepositNotFound(deposit_id))?;
            
            balance.pending_deposit -= deposit.amount;
            balance.available += deposit.amount;
        }
        
        Ok(deposit)
    }
    
    /// Withdraw funds
    pub async fn withdraw(
        &mut self,
        currency: Currency,
        amount: Decimal,
        destination: WithdrawalDestination,
    ) -> Result<Withdrawal> {
        // Security checks
        self.check_withdrawal_limits(amount).await?;
        self.check_destination_security(&destination).await?;
        
        let balance = self.balances.get(&currency)
            .ok_or(TreasuryError::InsufficientFunds { 
                required: amount, 
                available: Decimal::ZERO 
            })?;
        
        if balance.available < amount {
            return Err(TreasuryError::InsufficientFunds {
                required: amount,
                available: balance.available,
            });
        }
        
        // Execute withdrawal
        let withdrawal = if currency.is_fiat() {
            self.fiat_gateway.initiate_withdrawal(currency, amount, destination).await?
        } else {
            self.crypto_custody.initiate_withdrawal(currency, amount, destination).await?
        };
        
        // Update balances
        let balance = self.balances.get_mut(&currency).unwrap();
        balance.available -= amount;
        balance.pending_withdrawal += amount;
        self.daily_withdrawal_used += amount;
        
        Ok(withdrawal)
    }
    
    /// Convert between currencies
    pub async fn convert(
        &mut self,
        from: Currency,
        to: Currency,
        amount: Decimal,
    ) -> Result<FxConversion> {
        if from == to {
            return Err(TreasuryError::TransactionFailed(
                "Cannot convert to same currency".to_string()
            ));
        }
        
        // Check balance
        let balance = self.balances.get(&from)
            .ok_or(TreasuryError::InsufficientFunds {
                required: amount,
                available: Decimal::ZERO,
            })?;
        
        if balance.available < amount {
            return Err(TreasuryError::InsufficientFunds {
                required: amount,
                available: balance.available,
            });
        }
        
        // Get FX rate and convert
        let rate = self.fx_converter.get_rate(from, to).await?;
        let (to_amount, spread) = self.fx_converter.convert(from, to, amount, rate).await?;
        
        // Update balances
        let from_balance = self.balances.get_mut(&from).unwrap();
        from_balance.available -= amount;
        
        let to_balance = self.balances.entry(to).or_insert(Balance {
            currency: to,
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
            pending_deposit: Decimal::ZERO,
            pending_withdrawal: Decimal::ZERO,
        });
        to_balance.available += to_amount;
        
        Ok(FxConversion {
            id: Uuid::new_v4(),
            from_currency: from,
            to_currency: to,
            from_amount: amount,
            to_amount,
            rate,
            spread,
            created_at: Utc::now(),
            status: TransactionStatus::Cleared,
        })
    }
    
    /// Get best yield opportunity for idle cash
    pub async fn find_best_yield(&self, currency: Currency) -> Result<YieldOpportunity> {
        self.yield_optimizer.find_best(currency).await
    }
    
    /// Get balance for a currency
    pub fn get_balance(&self, currency: Currency) -> Option<&Balance> {
        self.balances.get(&currency)
    }
    
    /// Get all balances
    pub fn get_all_balances(&self) -> &HashMap<Currency, Balance> {
        &self.balances
    }
    
    /// Total equity in USD
    pub async fn total_equity_usd(&self) -> Result<Decimal> {
        let mut total = Decimal::ZERO;
        
        for (currency, balance) in &self.balances {
            let available = balance.available + balance.locked;
            
            if *currency == Currency::USD {
                total += available;
            } else {
                let rate = self.fx_converter.get_rate(*currency, Currency::USD).await?;
                total += available * rate;
            }
        }
        
        Ok(total)
    }
    
    // Private helper methods
    
    async fn check_withdrawal_limits(&self, amount: Decimal) -> Result<()> {
        // Reset daily limit if it's a new day
        let now = Utc::now();
        if now.date_naive() != self.last_reset.date_naive() {
            // TODO: Reset logic
        }
        
        if self.daily_withdrawal_used + amount > self.daily_withdrawal_limit {
            return Err(TreasuryError::WithdrawalLimitExceeded {
                requested: amount,
                limit: self.daily_withdrawal_limit - self.daily_withdrawal_used,
            });
        }
        
        Ok(())
    }
    
    async fn check_destination_security(&self, destination: &WithdrawalDestination) -> Result<()> {
        // TODO: Implement 2FA, whitelist checks, etc.
        match destination {
            WithdrawalDestination::CryptoWallet { address, .. } => {
                if address.is_empty() {
                    return Err(TreasuryError::SecurityCheckFailed(
                        "Invalid crypto address".to_string()
                    ));
                }
            }
            WithdrawalDestination::BankAccount { account_number, .. } => {
                if account_number.is_empty() {
                    return Err(TreasuryError::SecurityCheckFailed(
                        "Invalid bank account".to_string()
                    ));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
}

/// Yield opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldOpportunity {
    pub protocol: String,
    pub currency: Currency,
    pub apy: Decimal,
    pub tvl: Decimal,
    pub risk_score: u8, // 1-10
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_currency_parsing() {
        assert_eq!("USD".parse::<Currency>().unwrap(), Currency::USD);
        assert_eq!("BTC".parse::<Currency>().unwrap(), Currency::BTC);
        assert!("INVALID".parse::<Currency>().is_err());
    }
    
    #[test]
    fn test_currency_classification() {
        assert!(Currency::USD.is_fiat());
        assert!(!Currency::USD.is_crypto());
        
        assert!(Currency::BTC.is_crypto());
        assert!(!Currency::BTC.is_fiat());
    }
    
    #[tokio::test]
    async fn test_treasury_lifecycle() {
        // TODO: Implement full lifecycle test
        // This is the GOLDEN PATH test for Sprint 15
        let mut treasury = Treasury::new().await.unwrap();
        
        // 1. Deposit
        let deposit = treasury.deposit_fiat(Currency::USD, Decimal::from(10000)).await;
        assert!(deposit.is_ok());
        
        // More tests to come...
    }
}
