//! Fiat Gateway - Bank transfers, SEPA, SWIFT, ACH, Wire

use super::*;
use async_trait::async_trait;

/// Fiat payment gateway interface
#[async_trait]
pub trait FiatGatewayTrait: Send + Sync {
    /// Initiate a fiat deposit
    async fn initiate_deposit(&self, currency: Currency, amount: Decimal) -> Result<Deposit>;
    /// Confirm a deposit has cleared
    async fn confirm_deposit(&self, deposit_id: Uuid) -> Result<Deposit>;
    /// Initiate a withdrawal
    async fn initiate_withdrawal(
        &self,
        currency: Currency,
        amount: Decimal,
        destination: WithdrawalDestination,
    ) -> Result<Withdrawal>;
    /// Get deposit status
    async fn get_deposit_status(&self, deposit_id: Uuid) -> Result<TransactionStatus>;
}

// Re-export methods for direct access
impl FiatGateway {
    /// Initiate a fiat deposit (public wrapper)
    pub async fn initiate_deposit(&self, currency: Currency, amount: Decimal) -> Result<Deposit> {
        <Self as FiatGatewayTrait>::initiate_deposit(self, currency, amount).await
    }
    /// Confirm a deposit (public wrapper)
    pub async fn confirm_deposit(&self, deposit_id: Uuid) -> Result<Deposit> {
        <Self as FiatGatewayTrait>::confirm_deposit(self, deposit_id).await
    }
    /// Initiate a withdrawal (public wrapper)
    pub async fn initiate_withdrawal(
        &self,
        currency: Currency,
        amount: Decimal,
        destination: WithdrawalDestination,
    ) -> Result<Withdrawal> {
        <Self as FiatGatewayTrait>::initiate_withdrawal(self, currency, amount, destination).await
    }
}

/// Fiat gateway implementation
#[derive(Debug)]
pub struct FiatGateway {
    // Bank API connections
    // SEPA provider
    // SWIFT provider
}

impl FiatGateway {
    pub async fn new() -> Result<Self> {
        // TODO: Initialize bank connections
        Ok(Self {})
    }
}

#[async_trait]
impl FiatGatewayTrait for FiatGateway {
    async fn initiate_deposit(&self, currency: Currency, amount: Decimal) -> Result<Deposit> {
        // TODO: Connect to bank API
        // For now, return mock deposit
        Ok(Deposit {
            id: Uuid::new_v4(),
            currency,
            amount,
            status: TransactionStatus::Pending,
            created_at: Utc::now(),
            cleared_at: None,
            source: DepositSource::BankTransfer {
                bank_name: "Mock Bank".to_string(),
                account_last4: "1234".to_string(),
            },
            reference: Some(format!("DEP-{}", Uuid::new_v4())),
        })
    }
    
    async fn confirm_deposit(&self, deposit_id: Uuid) -> Result<Deposit> {
        // TODO: Check bank API for confirmation
        // Mock: always confirm for testing
        Ok(Deposit {
            id: deposit_id,
            currency: Currency::USD,
            amount: Decimal::from(10000),
            status: TransactionStatus::Cleared,
            created_at: Utc::now(),
            cleared_at: Some(Utc::now()),
            source: DepositSource::BankTransfer {
                bank_name: "Mock Bank".to_string(),
                account_last4: "1234".to_string(),
            },
            reference: Some("MOCK".to_string()),
        })
    }
    
    async fn initiate_withdrawal(
        &self,
        currency: Currency,
        amount: Decimal,
        destination: WithdrawalDestination,
    ) -> Result<Withdrawal> {
        // TODO: Connect to bank API for withdrawal
        Ok(Withdrawal {
            id: Uuid::new_v4(),
            currency,
            amount,
            status: TransactionStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            destination,
            fees: Decimal::from(25), // $25 wire fee
            reference: Some(format!("WDR-{}", Uuid::new_v4())),
        })
    }
    
    async fn get_deposit_status(&self, _deposit_id: Uuid) -> Result<TransactionStatus> {
        // TODO: Check actual status from bank
        Ok(TransactionStatus::Cleared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fiat_deposit_clears() {
        let gateway = FiatGateway::new().await.unwrap();
        
        let deposit = gateway
            .initiate_deposit(Currency::USD, Decimal::from(10000))
            .await
            .unwrap();
        
        assert_eq!(deposit.currency, Currency::USD);
        assert_eq!(deposit.amount, Decimal::from(10000));
        assert!(matches!(deposit.status, TransactionStatus::Pending));
        
        // Confirm deposit
        let confirmed = gateway.confirm_deposit(deposit.id).await.unwrap();
        assert!(matches!(confirmed.status, TransactionStatus::Cleared));
    }
    
    #[tokio::test]
    async fn test_fiat_withdrawal() {
        let gateway = FiatGateway::new().await.unwrap();
        
        let withdrawal = gateway
            .initiate_withdrawal(
                Currency::USD,
                Decimal::from(5000),
                WithdrawalDestination::BankAccount {
                    bank_name: "Chase".to_string(),
                    account_number: "****1234".to_string(),
                },
            )
            .await
            .unwrap();
        
        assert_eq!(withdrawal.amount, Decimal::from(5000));
        assert!(withdrawal.fees > Decimal::ZERO);
    }
}
