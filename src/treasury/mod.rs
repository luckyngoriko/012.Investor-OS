//! Treasury Module - Sprint 15
//!
//! Управление на капитал: крипто депозити, тегления, yield
//! 
//! NOTE: Fiat banking and FX conversion removed - requires banking license.
//! For fiat operations, use external payment processors (Stripe, Wise, etc.)
//! and integrate via webhooks to deposit crypto (USDC/USDT).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

pub mod crypto;
pub mod yield_optimizer;
pub mod security;

// Real custody integrations (feature-gated)
#[cfg(feature = "fireblocks")]
pub mod fireblocks;

pub use crypto::CryptoCustody;
pub use yield_optimizer::YieldOptimizer;
pub use security::{SecurityManager, SecurityCheck, TfaMethod};

#[cfg(feature = "fireblocks")]
pub use fireblocks::{FireblocksCustody, FireblocksConfig};

/// Errors that can occur in treasury operations
#[derive(Error, Debug)]
pub enum TreasuryError {
    #[error("Insufficient funds: requested {requested}, available {available}")]
    InsufficientFunds { requested: Decimal, available: Decimal, asset: String },
    
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
    
    #[error("Gateway error: {0}")]
    GatewayError(String),
    
    #[error("Fiat operations not supported. Use external payment processor and deposit USDC/USDT.")]
    FiatNotSupported,
    
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    
    #[error("Asset not found: {0}")]
    AssetNotFound(String),
    
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Security policy violation: {0}")]
    SecurityError(String),
}

pub type Result<T> = std::result::Result<T, TreasuryError>;
pub type TreasuryResult<T> = std::result::Result<T, TreasuryError>;

/// Supported currencies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    // Crypto
    BTC, ETH, USDT, USDC, SOL, ADA, DOT, DAI,
    // Fiat - deprecated, returns error when used
    USD, EUR, GBP,
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Currency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::BTC => "BTC",
            Currency::ETH => "ETH",
            Currency::USDT => "USDT",
            Currency::USDC => "USDC",
            Currency::SOL => "SOL",
            Currency::ADA => "ADA",
            Currency::DOT => "DOT",
            Currency::DAI => "DAI",
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
        }
    }
    
    pub fn is_fiat(&self) -> bool {
        matches!(self, Currency::USD | Currency::EUR | Currency::GBP)
    }
    
    pub fn is_crypto(&self) -> bool {
        !self.is_fiat()
    }
    
    pub fn decimals(&self) -> u32 {
        match self {
            Currency::BTC => 8,
            Currency::ETH | Currency::SOL | Currency::ADA | Currency::DOT => 18,
            Currency::USDT | Currency::USDC | Currency::DAI => 6,
            Currency::USD | Currency::EUR | Currency::GBP => 2,
        }
    }
    
    /// Get approximate USD price for equity calculations
    pub fn usd_price_estimate(&self) -> Decimal {
        match self {
            Currency::USDT | Currency::USDC | Currency::DAI => Decimal::ONE,
            Currency::BTC => Decimal::try_from(45000.0).unwrap_or(Decimal::ZERO),
            Currency::ETH => Decimal::try_from(2500.0).unwrap_or(Decimal::ZERO),
            Currency::SOL => Decimal::try_from(100.0).unwrap_or(Decimal::ZERO),
            Currency::ADA => Decimal::try_from(0.5).unwrap_or(Decimal::ZERO),
            Currency::DOT => Decimal::try_from(7.0).unwrap_or(Decimal::ZERO),
            Currency::USD => Decimal::ONE,
            Currency::EUR => Decimal::try_from(1.1).unwrap_or(Decimal::ZERO),
            Currency::GBP => Decimal::try_from(1.27).unwrap_or(Decimal::ZERO),
        }
    }
}

impl std::str::FromStr for Currency {
    type Err = TreasuryError;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "BTC" => Ok(Currency::BTC),
            "ETH" => Ok(Currency::ETH),
            "USDT" => Ok(Currency::USDT),
            "USDC" => Ok(Currency::USDC),
            "SOL" => Ok(Currency::SOL),
            "ADA" => Ok(Currency::ADA),
            "DOT" => Ok(Currency::DOT),
            "DAI" => Ok(Currency::DAI),
            "USD" | "EUR" | "GBP" => Err(TreasuryError::FiatNotSupported),
            _ => Err(TreasuryError::UnsupportedCurrency(s.to_string())),
        }
    }
}

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,           // Чака обработка
    Processing,        // В процес
    Cleared,           // Завършено успешно (old name)
    Confirmed,         // Завършено успешно (Fireblocks compatible)
    Failed(String),    // Неуспешно
    FailedStatus,      // Неуспешно (simple variant)
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
    CryptoWallet { address: String, chain: String, tx_hash: String },
    BankTransfer { bank_name: String, account_last4: String },
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
    CryptoWallet { address: String, chain: String },
    BankAccount { bank_name: String, account_number: String },
    Internal { to_account: Uuid },
}

/// Generic transaction (for custody providers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub asset_id: String,
    pub amount: Decimal,
    pub destination: String,
    pub status: TransactionStatus,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub tx_hash: Option<String>,
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

/// Currency conversion result (DEPRECATED - requires banking license)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversion {
    pub from_currency: Currency,
    pub to_currency: Currency,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub rate: Decimal,
    pub fees: Decimal,
}

/// Treasury manager - основен интерфейс
#[derive(Debug)]
pub struct Treasury {
    balances: HashMap<Currency, Balance>,
    crypto_custody: CryptoCustody,
    yield_optimizer: YieldOptimizer,
    security_manager: SecurityManager,
    
    // Daily tracking
    daily_withdrawal_used: Decimal,
    last_reset: DateTime<Utc>,
}

impl Treasury {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            balances: HashMap::new(),
            crypto_custody: CryptoCustody::new().await?,
            yield_optimizer: YieldOptimizer::new().await?,
            security_manager: SecurityManager::new(),
            daily_withdrawal_used: Decimal::ZERO,
            last_reset: Utc::now(),
        })
    }
    
    /// Get deposit address for a currency
    pub async fn get_deposit_address(&self, currency: Currency) -> Result<String> {
        self.crypto_custody.get_deposit_address(currency).await
    }
    
    /// Deposit crypto (process incoming deposit from external wallet)
    pub async fn process_deposit(
        &mut self, 
        currency: Currency, 
        amount: Decimal,
        tx_hash: String,
        from_address: String,
        chain: String,
    ) -> Result<Deposit> {
        if !currency.is_crypto() {
            return Err(TreasuryError::UnsupportedCurrency(
                format!("{} is not supported", currency.as_str())
            ));
        }
        
        let deposit = Deposit {
            id: Uuid::new_v4(),
            currency,
            amount,
            status: TransactionStatus::Cleared, // Assume confirmed when we see it
            created_at: Utc::now(),
            cleared_at: Some(Utc::now()),
            source: DepositSource::CryptoWallet { 
                address: from_address, 
                chain,
                tx_hash,
            },
            reference: None,
        };
        
        // Update available balance immediately
        let balance = self.balances.entry(currency).or_insert(Balance {
            currency,
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
            pending_deposit: Decimal::ZERO,
            pending_withdrawal: Decimal::ZERO,
        });
        balance.available += amount;
        
        Ok(deposit)
    }
    
    /// Withdraw crypto with 2FA verification
    pub async fn withdraw(
        &mut self,
        currency: Currency,
        amount: Decimal,
        destination: WithdrawalDestination,
        tfa_code: Option<&str>, // 2FA code for large withdrawals
    ) -> Result<Withdrawal> {
        // Reset daily limit if needed
        self.check_daily_reset();
        
        // Check limits FIRST
        self.security_manager.check_limits(
            amount,
            self.daily_withdrawal_used,
            Decimal::ZERO,
        ).map_err(|_e| TreasuryError::WithdrawalLimitExceeded {
            requested: amount,
            limit: self.security_manager.limits.daily_usd - self.daily_withdrawal_used,
        })?;
        
        // Security check - 2FA required?
        let security_check = self.security_manager.check_withdrawal_requirements(
            amount,
            currency,
            &destination,
        );
        
        match security_check {
            SecurityCheck::Requires2FA { method } => {
                let code = tfa_code.ok_or_else(|| {
                    TreasuryError::SecurityCheckFailed(
                        format!("2FA required for this withdrawal (method: {:?})", method)
                    )
                })?;
                
                if !self.security_manager.verify_2fa(code) {
                    return Err(TreasuryError::SecurityCheckFailed(
                        "Invalid 2FA code".to_string()
                    ));
                }
            }
            SecurityCheck::Failed(reason) => {
                return Err(TreasuryError::SecurityCheckFailed(reason));
            }
            SecurityCheck::Passed => {}
        }
        
        // Check balance
        let balance = self.balances.get(&currency)
            .ok_or(TreasuryError::InsufficientFunds { 
                requested: amount,
                available: Decimal::ZERO,
                asset: currency.to_string(),
            })?;
        
        if balance.available < amount {
            return Err(TreasuryError::InsufficientFunds {
                requested: amount,
                asset: currency.to_string(),
                available: balance.available,
            });
        }
        
        // Execute withdrawal
        let withdrawal = self.crypto_custody.initiate_withdrawal(
            currency, amount, destination
        ).await?;
        
        // Update balances
        let balance = self.balances.get_mut(&currency).unwrap();
        balance.available -= amount;
        balance.pending_withdrawal += amount;
        self.daily_withdrawal_used += amount;
        
        Ok(withdrawal)
    }
    
    /// Confirm withdrawal (when blockchain confirms)
    pub async fn confirm_withdrawal(&mut self, withdrawal_id: Uuid) -> Result<Withdrawal> {
        let withdrawal = self.crypto_custody.confirm_withdrawal(withdrawal_id).await?;
        
        if let TransactionStatus::Cleared = withdrawal.status {
            let balance = self.balances.get_mut(&withdrawal.currency)
                .ok_or(TreasuryError::InsufficientFunds {
                    requested: Decimal::ZERO,
                    available: Decimal::ZERO,
                    asset: withdrawal.currency.to_string(),
                })?;
            
            balance.pending_withdrawal -= withdrawal.amount;
        }
        
        Ok(withdrawal)
    }
    
    /// Get balance for a currency
    pub fn get_balance(&self, currency: Currency) -> Option<&Balance> {
        self.balances.get(&currency)
    }
    
    /// Get all balances
    pub fn get_all_balances(&self) -> &HashMap<Currency, Balance> {
        &self.balances
    }
    
    /// Lock funds for trading
    pub fn lock_funds(&mut self, currency: Currency, amount: Decimal) -> Result<()> {
        let balance = self.balances.get_mut(&currency)
            .ok_or(TreasuryError::InsufficientFunds {
                requested: amount,
                asset: currency.to_string(),
                available: Decimal::ZERO,
            })?;
        
        if balance.available < amount {
            return Err(TreasuryError::InsufficientFunds {
                requested: amount,
                asset: currency.to_string(),
                available: balance.available,
            });
        }
        
        balance.available -= amount;
        balance.locked += amount;
        
        Ok(())
    }
    
    /// Release locked funds
    pub fn release_funds(&mut self, currency: Currency, amount: Decimal) -> Result<()> {
        let balance = self.balances.get_mut(&currency)
            .ok_or(TreasuryError::InsufficientFunds {
                requested: amount,
                asset: currency.to_string(),
                available: Decimal::ZERO,
            })?;
        
        if balance.locked < amount {
            return Err(TreasuryError::InsufficientFunds {
                requested: amount,
                asset: currency.to_string(),
                available: balance.locked,
            });
        }
        
        balance.locked -= amount;
        balance.available += amount;
        
        Ok(())
    }
    
    /// Calculate total equity in USD
    pub async fn total_equity_usd(&self) -> Result<Decimal> {
        let mut total = Decimal::ZERO;
        
        for (currency, balance) in &self.balances {
            let available_usd = balance.available * currency.usd_price_estimate();
            let locked_usd = balance.locked * currency.usd_price_estimate();
            total += available_usd + locked_usd;
        }
        
        // Add yield positions
        let yield_value = self.yield_optimizer.total_value_usd().await?;
        total += yield_value;
        
        Ok(total)
    }
    
    /// Get total equity (blocking version for compatibility)
    /// 
    /// NOTE: This method returns an approximate value and doesn't include yield positions.
    /// Use `total_equity_usd().await` for accurate async calculation.
    pub fn total_equity(&self) -> Decimal {
        let mut total = Decimal::ZERO;
        
        for (currency, balance) in &self.balances {
            let available_usd = balance.available * currency.usd_price_estimate();
            let locked_usd = balance.locked * currency.usd_price_estimate();
            total += available_usd + locked_usd;
        }
        
        total
    }
    
    /// Get yield opportunities
    pub async fn get_yield_opportunities(&self) -> Result<Vec<yield_optimizer::YieldOpportunity>> {
        self.yield_optimizer.find_opportunities().await
    }
    
    /// Deposit fiat (DEPRECATED - requires banking license)
    pub async fn deposit_fiat(&self, _currency: Currency, _amount: Decimal) -> Result<Deposit> {
        Err(TreasuryError::FiatNotSupported)
    }
    
    /// Confirm deposit (stub for backward compatibility)
    pub async fn confirm_deposit(&self, _deposit_id: Uuid) -> Result<Deposit> {
        Err(TreasuryError::InvalidAmount("Not implemented".to_string()))
    }
    
    /// Convert between currencies (DEPRECATED - requires banking license)
    pub async fn convert(
        &self,
        from: Currency,
        to: Currency,
        _amount: Decimal,
    ) -> Result<Conversion> {
        // Fiat conversion requires banking license
        if from.is_fiat() || to.is_fiat() {
            return Err(TreasuryError::FiatNotSupported);
        }
        
        // For crypto-to-crypto, we would need DEX integration
        // This is not implemented yet
        Err(TreasuryError::FiatNotSupported)
    }
    
    /// Allocate to yield
    pub async fn allocate_to_yield(
        &mut self,
        currency: Currency,
        amount: Decimal,
        protocol: String,
    ) -> Result<yield_optimizer::YieldPosition> {
        // Check balance
        let balance = self.balances.get(&currency)
            .ok_or(TreasuryError::InsufficientFunds {
                requested: amount,
                asset: currency.to_string(),
                available: Decimal::ZERO,
            })?;
        
        if balance.available < amount {
            return Err(TreasuryError::InsufficientFunds {
                requested: amount,
                asset: currency.to_string(),
                available: balance.available,
            });
        }
        
        // Allocate
        let position = self.yield_optimizer.allocate(currency, amount, protocol).await?;
        
        // Update balance
        let balance = self.balances.get_mut(&currency).unwrap();
        balance.available -= amount;
        
        Ok(position)
    }
    
    fn check_daily_reset(&mut self) {
        let now = Utc::now();
        let days_since_reset = (now - self.last_reset).num_days();
        
        if days_since_reset >= 1 {
            self.daily_withdrawal_used = Decimal::ZERO;
            self.last_reset = now;
        }
    }
    
    async fn check_destination_security(&self, destination: &WithdrawalDestination) -> Result<()> {
        match destination {
            WithdrawalDestination::CryptoWallet { address, chain } => {
                // Validate address format
                if address.len() < 10 {
                    return Err(TreasuryError::SecurityCheckFailed(
                        "Invalid wallet address format".to_string()
                    ));
                }
                
                // Check against blacklist
                if self.security_manager.is_blacklisted(address).await {
                    return Err(TreasuryError::SecurityCheckFailed(
                        "Destination address is blacklisted".to_string()
                    ));
                }
                
                // Validate chain
                let valid_chains = ["BTC", "ETH", "SOL", "ADA", "DOT"];
                if !valid_chains.contains(&chain.as_str()) {
                    return Err(TreasuryError::SecurityCheckFailed(
                        format!("Unsupported chain: {}", chain)
                    ));
                }
                
                Ok(())
            }
            WithdrawalDestination::BankAccount { bank_name: _, account_number: _ } => {
                // Fiat withdrawals are not supported without banking license
                Err(TreasuryError::FiatNotSupported)
            }
            WithdrawalDestination::Internal { .. } => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_treasury_lifecycle() {
        let mut treasury = Treasury::new().await.unwrap();
        
        // === 1. DEPOSIT ===
        let deposit = treasury.process_deposit(
            Currency::USDC,
            Decimal::from(10000),
            "0xabc123".to_string(),
            "0xfromaddress".to_string(),
            "ETH".to_string(),
        ).await.expect("Should process deposit");
        
        assert_eq!(deposit.amount, Decimal::from(10000));
        assert_eq!(deposit.currency, Currency::USDC);
        
        // Check balance
        let usdc_balance = treasury.get_balance(Currency::USDC).unwrap();
        assert_eq!(usdc_balance.available, Decimal::from(10000));
        assert_eq!(usdc_balance.pending_deposit, Decimal::ZERO); // Already cleared
        
        // === 2. LOCK FUNDS FOR TRADING ===
        treasury.lock_funds(Currency::USDC, Decimal::from(5000))
            .expect("Should lock funds");
        
        let usdc_balance = treasury.get_balance(Currency::USDC).unwrap();
        assert_eq!(usdc_balance.available, Decimal::from(5000));
        assert_eq!(usdc_balance.locked, Decimal::from(5000));
        
        // === 3. RELEASE FUNDS ===
        treasury.release_funds(Currency::USDC, Decimal::from(2000))
            .expect("Should release funds");
        
        let usdc_balance = treasury.get_balance(Currency::USDC).unwrap();
        assert_eq!(usdc_balance.available, Decimal::from(7000));
        assert_eq!(usdc_balance.locked, Decimal::from(3000));
        
        // === 4. WITHDRAW (small, no 2FA needed) ===
        let withdrawal = treasury
            .withdraw(
                Currency::USDC,
                Decimal::from(1000), // < $10k threshold
                WithdrawalDestination::CryptoWallet {
                    address: "0x1234567890abcdef".to_string(),
                    chain: "ETH".to_string(),
                },
                None, // No 2FA needed for small amount
            )
            .await
            .expect("Should initiate withdrawal");
        
        assert_eq!(withdrawal.amount, Decimal::from(1000));
        assert!(withdrawal.fees >= Decimal::ZERO);
        
        // Check final balance
        let usdc_final = treasury.get_balance(Currency::USDC).unwrap();
        assert_eq!(usdc_final.available, Decimal::from(6000)); // 7000 - 1000
        assert_eq!(usdc_final.pending_withdrawal, Decimal::from(1000));
        
        // === 5. Total Equity ===
        let total_equity = treasury.total_equity_usd().await
            .expect("Should calculate total equity");
        
        // Should be around 9000 USD (9000 USDC equivalent)
        assert!(total_equity >= Decimal::from(8000));
        
        println!("✅ Treasury lifecycle completed successfully!");
        println!("   - Deposited: 10,000 USDC");
        println!("   - Locked: 5,000 USDC");
        println!("   - Released: 2,000 USDC");
        println!("   - Withdrew: 1,000 USDC");
        println!("   - Total equity: ${}", total_equity);
    }
    
    #[tokio::test]
    async fn test_withdrawal_2fa_required_for_large_amounts() {
        let mut treasury = Treasury::new().await.unwrap();
        
        // Deposit first
        treasury.process_deposit(
            Currency::USDC,
            Decimal::from(50000),
            "0xabc123".to_string(),
            "0xfromaddress".to_string(),
            "ETH".to_string(),
        ).await.unwrap();
        
        // Large withdrawal without 2FA should fail
        let result = treasury.withdraw(
            Currency::USDC,
            Decimal::from(20000), // > $10k threshold
            WithdrawalDestination::CryptoWallet {
                address: "0x1234567890abcdef".to_string(),
                chain: "ETH".to_string(),
            },
            None, // No 2FA code
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("2FA required"));
        
        // Large withdrawal WITH 2FA should succeed (test code: "123456")
        let withdrawal = treasury.withdraw(
            Currency::USDC,
            Decimal::from(20000),
            WithdrawalDestination::CryptoWallet {
                address: "0x1234567890abcdef".to_string(),
                chain: "ETH".to_string(),
            },
            Some("123456"), // Valid test 2FA code
        ).await;
        
        assert!(withdrawal.is_ok());
    }
    
    #[tokio::test]
    async fn test_withdrawal_limit_enforcement() {
        let mut treasury = Treasury::new().await.unwrap();
        
        // Deposit
        treasury.process_deposit(
            Currency::USDC,
            Decimal::from(200000),
            "0xabc123".to_string(),
            "0xfromaddress".to_string(),
            "ETH".to_string(),
        ).await.unwrap();
        
        // Try to withdraw more than daily limit ($100k)
        let result = treasury.withdraw(
            Currency::USDC,
            Decimal::from(150000), // > $100k daily limit
            WithdrawalDestination::CryptoWallet {
                address: "0x1234567890abcdef".to_string(),
                chain: "ETH".to_string(),
            },
            None,
        ).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("limit"));
    }
    
    #[tokio::test]
    async fn test_fiat_not_supported() {
        // Verify fiat currencies are not supported
        let result = "USD".parse::<Currency>();
        assert!(result.is_err());
        
        let result = "EUR".parse::<Currency>();
        assert!(result.is_err());
        
        // But crypto works
        let result = "BTC".parse::<Currency>();
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_get_deposit_address() {
        let treasury = Treasury::new().await.unwrap();
        
        let address = treasury.get_deposit_address(Currency::BTC).await;
        assert!(address.is_ok());
        
        let address = treasury.get_deposit_address(Currency::ETH).await;
        assert!(address.is_ok());
    }
}
