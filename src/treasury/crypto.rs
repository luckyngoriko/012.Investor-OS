//! Crypto Custody - Paper Trading Implementation
//!
//! NOTE: This is a PAPER TRADING implementation for demo/testing.
//! For production with real crypto, integrate with:
//! - Fireblocks (https://www.fireblocks.com/)
//! - Copper (https://copper.co/)
//! - Coinbase Prime
//! - Or self-custody with MPC

use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Storage type for crypto assets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageType {
    Hot,  // Online wallet for immediate trading
    Warm, // Semi-offline for regular withdrawals
    Cold, // Offline multi-sig for long-term storage
}

/// Cold storage information
#[derive(Debug, Clone)]
pub struct ColdStorageInfo {
    pub total_btc: Decimal,
    pub total_eth: Decimal,
    pub multi_sig_threshold: u8,
    pub signers: Vec<String>,
    pub hardware_security_modules: u8,
    pub geo_redundancy: u8,
}

impl Default for ColdStorageInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl ColdStorageInfo {
    pub fn new() -> Self {
        Self {
            total_btc: Decimal::ZERO,
            total_eth: Decimal::ZERO,
            multi_sig_threshold: 3,
            signers: vec![
                "Signer-1".to_string(),
                "Signer-2".to_string(),
                "Signer-3".to_string(),
                "Signer-4".to_string(),
                "Signer-5".to_string(),
            ],
            hardware_security_modules: 2,
            geo_redundancy: 3,
        }
    }
}

/// Pending withdrawal tracking
#[derive(Debug, Clone)]
struct PendingWithdrawal {
    withdrawal: Withdrawal,
    requested_at: DateTime<Utc>,
}

/// Pending deposit tracking
#[derive(Debug, Clone)]
struct PendingDeposit {
    deposit: Deposit,
    requested_at: DateTime<Utc>,
}

/// Crypto custody - Paper Trading implementation
#[derive(Debug)]
pub struct CryptoCustody {
    // In-memory storage for paper trading
    balances: Arc<Mutex<HashMap<Currency, Decimal>>>,
    pending_deposits: Arc<Mutex<HashMap<Uuid, PendingDeposit>>>,
    pending_withdrawals: Arc<Mutex<HashMap<Uuid, PendingWithdrawal>>>,
    cold_storage_balances: Arc<Mutex<HashMap<Currency, Decimal>>>,
}

impl CryptoCustody {
    pub async fn new() -> Result<Self> {
        let mut balances = HashMap::new();
        // Initialize with zero balances for all supported currencies
        for currency in [
            Currency::BTC,
            Currency::ETH,
            Currency::USDT,
            Currency::USDC,
            Currency::SOL,
            Currency::ADA,
            Currency::DOT,
            Currency::DAI,
        ] {
            balances.insert(currency, Decimal::ZERO);
        }

        Ok(Self {
            balances: Arc::new(Mutex::new(balances)),
            pending_deposits: Arc::new(Mutex::new(HashMap::new())),
            pending_withdrawals: Arc::new(Mutex::new(HashMap::new())),
            cold_storage_balances: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Generate deposit address for a currency.
    ///
    /// Requires `fireblocks` feature flag for real custody integration.
    /// Without it, returns an error directing the user to configure a custody provider.
    #[cfg(feature = "fireblocks")]
    pub fn generate_address(
        &self,
        _currency: Currency,
    ) -> std::result::Result<String, super::TreasuryError> {
        // Real Fireblocks integration would go here via the FireblocksCustody client
        Err(super::TreasuryError::ProviderError(
            "Fireblocks address generation not yet configured. Set FIREBLOCKS_API_KEY and FIREBLOCKS_SECRET.".to_string(),
        ))
    }

    /// Generate deposit address for a currency.
    ///
    /// Without the `fireblocks` feature, crypto deposit is not available.
    #[cfg(not(feature = "fireblocks"))]
    pub fn generate_address(
        &self,
        _currency: Currency,
    ) -> std::result::Result<String, super::TreasuryError> {
        Err(super::TreasuryError::ConfigError(
            "Crypto deposits require custody provider configuration. Enable the 'fireblocks' feature.".to_string(),
        ))
    }

    /// Generate address with specific storage type
    pub async fn generate_address_with_type(
        &self,
        currency: Currency,
        storage_type: StorageType,
    ) -> Result<String> {
        let prefix = match storage_type {
            StorageType::Hot => "hot",
            StorageType::Warm => "warm",
            StorageType::Cold => "cold",
        };

        match currency {
            Currency::BTC if storage_type == StorageType::Cold => Ok(format!(
                "bc1q{}",
                &Uuid::new_v4().to_string().replace("-", "").to_lowercase()[..24]
            )),
            Currency::ETH if storage_type == StorageType::Cold => Ok(format!(
                "0x{}",
                &Uuid::new_v4().to_string().replace("-", "").to_lowercase()[..40]
            )),
            _ => Ok(format!(
                "{}_{}_{}",
                prefix,
                currency.as_str(),
                Uuid::new_v4()
            )),
        }
    }

    /// Credit balance (called when deposit is received)
    pub fn credit_balance(&self, currency: Currency, amount: Decimal) -> Result<()> {
        let mut balances = self
            .balances
            .lock()
            .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;

        let current = balances.get(&currency).copied().unwrap_or(Decimal::ZERO);
        balances.insert(currency, current + amount);

        Ok(())
    }

    /// Debit balance (called when withdrawal is confirmed)
    pub fn debit_balance(&self, currency: Currency, amount: Decimal) -> Result<()> {
        let mut balances = self
            .balances
            .lock()
            .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;

        let current = balances.get(&currency).copied().unwrap_or(Decimal::ZERO);
        if current < amount {
            return Err(TreasuryError::InsufficientFunds {
                requested: amount,
                available: current,
                asset: currency.to_string(),
            });
        }

        balances.insert(currency, current - amount);
        Ok(())
    }

    /// Get current balance
    pub fn get_balance_internal(&self, currency: Currency) -> Decimal {
        self.balances
            .lock()
            .ok()
            .and_then(|b| b.get(&currency).copied())
            .unwrap_or(Decimal::ZERO)
    }

    /// Check confirmations (mock for paper trading)
    ///
    /// In production, query blockchain via RPC node
    pub async fn check_confirmations(&self, _tx_hash: &str) -> Result<u32> {
        // Paper trading: always return confirmed
        Ok(6)
    }

    /// Deposit to cold storage (mock)
    pub async fn deposit_to_cold_storage(
        &self,
        currency: Currency,
        amount: Decimal,
    ) -> Result<Deposit> {
        // Debit from hot wallet
        self.debit_balance(currency, amount)?;

        // Credit to cold storage
        {
            let mut cold = self
                .cold_storage_balances
                .lock()
                .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;
            let current = cold.get(&currency).copied().unwrap_or(Decimal::ZERO);
            cold.insert(currency, current + amount);
        }

        let address = self
            .generate_address_with_type(currency, StorageType::Cold)
            .await?;

        Ok(Deposit {
            id: Uuid::new_v4(),
            currency,
            amount,
            status: TransactionStatus::Cleared,
            created_at: Utc::now(),
            cleared_at: Some(Utc::now()),
            source: DepositSource::CryptoWallet {
                address,
                chain: currency.as_str().to_string(),
                tx_hash: format!(
                    "COLD-{}-{}-{}-TX",
                    currency.as_str(),
                    amount,
                    Uuid::new_v4()
                ),
            },
            reference: Some(format!("COLD-TX-{}", Uuid::new_v4())),
        })
    }

    /// Get cold storage balance
    pub async fn get_cold_balance(&self, currency: Currency) -> Result<Decimal> {
        let cold = self
            .cold_storage_balances
            .lock()
            .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;
        Ok(cold.get(&currency).copied().unwrap_or(Decimal::ZERO))
    }

    /// Get comprehensive cold storage information
    pub async fn get_cold_storage_info(&self) -> Result<ColdStorageInfo> {
        let mut info = ColdStorageInfo::new();

        info.total_btc = self
            .get_cold_balance(Currency::BTC)
            .await
            .unwrap_or(Decimal::ZERO);
        info.total_eth = self
            .get_cold_balance(Currency::ETH)
            .await
            .unwrap_or(Decimal::ZERO);

        Ok(info)
    }

    /// Get deposit address for a currency
    pub async fn get_deposit_address(&self, currency: Currency) -> Result<String> {
        self.generate_address(currency)
    }

    /// Initiate withdrawal (mock implementation)
    pub async fn initiate_withdrawal(
        &self,
        currency: Currency,
        amount: Decimal,
        destination: WithdrawalDestination,
    ) -> Result<Withdrawal> {
        let withdrawal = Withdrawal {
            id: Uuid::new_v4(),
            currency,
            amount,
            status: TransactionStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            destination,
            fees: Decimal::try_from(0.0001).unwrap(), // Network fee estimate
            reference: Some(format!("WITHDRAWAL-{}", Uuid::new_v4())),
        };

        // Store in pending
        {
            let mut pending = self
                .pending_withdrawals
                .lock()
                .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;
            pending.insert(
                withdrawal.id,
                PendingWithdrawal {
                    withdrawal: withdrawal.clone(),
                    requested_at: Utc::now(),
                },
            );
        }

        Ok(withdrawal)
    }

    /// Confirm withdrawal (process blockchain confirmation)
    pub async fn confirm_withdrawal(&self, withdrawal_id: Uuid) -> Result<Withdrawal> {
        let mut pending = self
            .pending_withdrawals
            .lock()
            .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;

        let pending_withdrawal = pending.remove(&withdrawal_id).ok_or_else(|| {
            TreasuryError::TransactionFailed(format!("Withdrawal {} not found", withdrawal_id))
        })?;

        let mut withdrawal = pending_withdrawal.withdrawal;
        withdrawal.status = TransactionStatus::Cleared;
        withdrawal.completed_at = Some(Utc::now());

        // Actually deduct the balance now
        self.debit_balance(withdrawal.currency, withdrawal.amount + withdrawal.fees)?;

        Ok(withdrawal)
    }
}

#[async_trait]
impl CryptoCustodyTrait for CryptoCustody {
    async fn initiate_deposit(&self, currency: Currency, amount: Decimal) -> Result<Deposit> {
        let address = self.generate_address(currency)?;

        let deposit = Deposit {
            id: Uuid::new_v4(),
            currency,
            amount,
            status: TransactionStatus::Pending,
            created_at: Utc::now(),
            cleared_at: None,
            source: DepositSource::CryptoWallet {
                address,
                chain: currency.as_str().to_string(),
                tx_hash: format!("PENDING-{}", Uuid::new_v4()),
            },
            reference: Some(format!("DEPOSIT-{}", Uuid::new_v4())),
        };

        {
            let mut pending = self
                .pending_deposits
                .lock()
                .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;
            pending.insert(
                deposit.id,
                PendingDeposit {
                    deposit: deposit.clone(),
                    requested_at: Utc::now(),
                },
            );
        }

        Ok(deposit)
    }

    async fn confirm_deposit(&self, deposit_id: Uuid) -> Result<Deposit> {
        let mut pending = self
            .pending_deposits
            .lock()
            .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;

        let pending_deposit = pending
            .remove(&deposit_id)
            .ok_or(TreasuryError::DepositNotFound(deposit_id))?;

        let mut deposit = pending_deposit.deposit;
        deposit.status = TransactionStatus::Cleared;
        deposit.cleared_at = Some(Utc::now());

        if let DepositSource::CryptoWallet { tx_hash, .. } = &mut deposit.source {
            if tx_hash.starts_with("PENDING-") {
                *tx_hash = format!("CONFIRMED-{}", Uuid::new_v4());
            }
        }

        self.credit_balance(deposit.currency, deposit.amount)?;

        Ok(deposit)
    }

    async fn initiate_withdrawal(
        &self,
        currency: Currency,
        amount: Decimal,
        destination: WithdrawalDestination,
    ) -> Result<Withdrawal> {
        let withdrawal = Withdrawal {
            id: Uuid::new_v4(),
            currency,
            amount,
            status: TransactionStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            destination,
            fees: Decimal::try_from(0.0001).unwrap(), // Network fee estimate
            reference: Some(format!("WITHDRAWAL-{}", Uuid::new_v4())),
        };

        // Store in pending
        {
            let mut pending = self
                .pending_withdrawals
                .lock()
                .map_err(|_| TreasuryError::TransactionFailed("Lock poisoned".to_string()))?;
            pending.insert(
                withdrawal.id,
                PendingWithdrawal {
                    withdrawal: withdrawal.clone(),
                    requested_at: Utc::now(),
                },
            );
        }

        Ok(withdrawal)
    }

    async fn get_deposit_address(&self, currency: Currency) -> Result<String> {
        self.generate_address(currency)
    }

    async fn get_balance(&self, currency: Currency) -> Result<Decimal> {
        Ok(self.get_balance_internal(currency))
    }
}

/// Crypto custody interface
#[async_trait]
pub trait CryptoCustodyTrait: Send + Sync {
    async fn initiate_deposit(&self, currency: Currency, amount: Decimal) -> Result<Deposit>;
    async fn confirm_deposit(&self, deposit_id: Uuid) -> Result<Deposit>;
    async fn initiate_withdrawal(
        &self,
        currency: Currency,
        amount: Decimal,
        destination: WithdrawalDestination,
    ) -> Result<Withdrawal>;
    async fn get_deposit_address(&self, currency: Currency) -> Result<String>;
    async fn get_balance(&self, currency: Currency) -> Result<Decimal>;
}

/// Cold storage wallet (multi-sig)
pub struct ColdStorage {
    pub threshold: u8,
    pub total_signers: u8,
}

impl ColdStorage {
    pub fn new(threshold: u8, total: u8) -> Self {
        Self {
            threshold,
            total_signers: total,
        }
    }

    /// Move funds from hot wallet to cold storage
    pub async fn sweep_to_cold(&self, amount: Decimal, currency: Currency) -> Result<String> {
        println!(
            "[PAPER TRADING] Sweeping {} {} to cold storage",
            amount,
            currency.as_str()
        );
        Ok(format!("sweep-tx-{}", Uuid::new_v4()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_crypto_deposit_requires_provider() {
        let custody = CryptoCustody::new().await.unwrap();

        // Without custody provider, initiate_deposit fails at address generation
        let result = custody
            .initiate_deposit(Currency::BTC, Decimal::try_from(0.5).unwrap())
            .await;

        assert!(
            result.is_err(),
            "initiate_deposit should fail without custody provider"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("custody")
                || err_msg.contains("configuration")
                || err_msg.contains("configured"),
            "Error should mention missing provider: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_confirm_unknown_deposit_fails() {
        let custody = CryptoCustody::new().await.unwrap();
        let result = custody.confirm_deposit(Uuid::new_v4()).await;
        assert!(matches!(result, Err(TreasuryError::DepositNotFound(_))));
    }

    #[tokio::test]
    async fn test_generate_address_requires_provider() {
        let custody = CryptoCustody::new().await.unwrap();

        // Without a custody provider configured, address generation returns an error
        let btc_result = custody.generate_address(Currency::BTC);
        assert!(btc_result.is_err(), "BTC address requires custody provider");

        let eth_result = custody.generate_address(Currency::ETH);
        assert!(eth_result.is_err(), "ETH address requires custody provider");

        let sol_result = custody.generate_address(Currency::SOL);
        assert!(sol_result.is_err(), "SOL address requires custody provider");

        // Verify error message mentions configuration
        let err_msg = btc_result.unwrap_err().to_string();
        assert!(
            err_msg.contains("configuration") || err_msg.contains("configured"),
            "Error should mention missing configuration: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_balance_credit_debit() {
        let custody = CryptoCustody::new().await.unwrap();

        // Credit
        custody
            .credit_balance(Currency::BTC, Decimal::from(10))
            .unwrap();
        assert_eq!(
            custody.get_balance_internal(Currency::BTC),
            Decimal::from(10)
        );

        // Debit
        custody
            .debit_balance(Currency::BTC, Decimal::from(3))
            .unwrap();
        assert_eq!(
            custody.get_balance_internal(Currency::BTC),
            Decimal::from(7)
        );

        // Insufficient funds
        let result = custody.debit_balance(Currency::BTC, Decimal::from(100));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_withdrawal_flow() {
        let custody = CryptoCustody::new().await.unwrap();

        // Credit first
        custody
            .credit_balance(Currency::ETH, Decimal::from(100))
            .unwrap();

        // Initiate withdrawal
        let withdrawal = custody
            .initiate_withdrawal(
                Currency::ETH,
                Decimal::from(50),
                WithdrawalDestination::CryptoWallet {
                    address: "0x1234567890abcdef".to_string(),
                    chain: "ETH".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(withdrawal.status, TransactionStatus::Pending);
        assert_eq!(withdrawal.amount, Decimal::from(50));

        // Balance still unchanged (pending)
        assert_eq!(
            custody.get_balance_internal(Currency::ETH),
            Decimal::from(100)
        );

        // Confirm withdrawal
        let confirmed = custody.confirm_withdrawal(withdrawal.id).await.unwrap();
        assert_eq!(confirmed.status, TransactionStatus::Cleared);

        // Balance updated
        let expected_balance =
            Decimal::from(100) - Decimal::from(50) - Decimal::try_from(0.0001).unwrap();
        assert_eq!(
            custody.get_balance_internal(Currency::ETH),
            expected_balance
        ); // 100 - 50 - fees
    }
}
