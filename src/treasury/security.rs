//! Treasury Security - 2FA, Whitelists, Limits

use super::{Currency, Decimal, WithdrawalDestination};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;

/// Security manager for treasury operations
#[derive(Debug, Clone)]
pub struct SecurityManager {
    /// Whitelisted withdrawal addresses (crypto)
    whitelisted_addresses: HashSet<String>,
    /// Whitelisted bank accounts (fiat)
    whitelisted_accounts: HashSet<String>,
    /// 2FA settings
    tfa_settings: TfaSettings,
    /// Withdrawal limits (public for access)
    pub limits: WithdrawalLimits,
    /// Recent 2FA codes (for verification)
    recent_codes: Vec<(String, DateTime<Utc>)>, // (code, expiry)
}

#[derive(Debug, Clone)]
pub struct TfaSettings {
    pub enabled: bool,
    pub method: TfaMethod,
    /// Require 2FA for withdrawals above this amount
    pub threshold: Decimal,
}

#[derive(Debug, Clone)]
pub enum TfaMethod {
    None,
    Totp,           // Time-based OTP (Google Authenticator)
    Sms,            // SMS code
    Email,          // Email code
    HardwareKey,    // YubiKey, etc.
}

#[derive(Debug, Clone)]
pub struct WithdrawalLimits {
    /// Daily limit in USD
    pub daily_usd: Decimal,
    /// Single transaction limit
    pub per_transaction: Decimal,
    /// Monthly limit
    pub monthly_usd: Decimal,
}

impl WithdrawalLimits {
    pub fn new(daily: Decimal, per_tx: Decimal, monthly: Decimal) -> Self {
        Self {
            daily_usd: daily,
            per_transaction: per_tx,
            monthly_usd: monthly,
        }
    }
}

impl Default for WithdrawalLimits {
    fn default() -> Self {
        Self {
            daily_usd: Decimal::from(100000),      // $100k/day
            per_transaction: Decimal::from(50000), // $50k/tx
            monthly_usd: Decimal::from(1000000),   // $1M/month
        }
    }
}

/// Security check result
#[derive(Debug, Clone)]
pub enum SecurityCheck {
    Passed,
    Requires2FA { method: TfaMethod },
    Failed(String),
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            whitelisted_addresses: HashSet::new(),
            whitelisted_accounts: HashSet::new(),
            tfa_settings: TfaSettings {
                enabled: true,
                method: TfaMethod::Totp,
                threshold: Decimal::from(10000), // $10k threshold
            },
            limits: WithdrawalLimits::default(),
            recent_codes: Vec::new(),
        }
    }
    
    /// Check if withdrawal needs 2FA
    pub fn check_withdrawal_requirements(
        &self,
        amount: Decimal,
        _currency: Currency,
        destination: &WithdrawalDestination,
    ) -> SecurityCheck {
        // Check amount threshold
        if self.tfa_settings.enabled && amount >= self.tfa_settings.threshold {
            // Check if destination is whitelisted
            let is_whitelisted = match destination {
                WithdrawalDestination::CryptoWallet { address, .. } => {
                    self.whitelisted_addresses.contains(address)
                }
                WithdrawalDestination::BankAccount { account_number, .. } => {
                    self.whitelisted_accounts.contains(account_number)
                }
                _ => false,
            };
            
            if !is_whitelisted {
                return SecurityCheck::Requires2FA {
                    method: self.tfa_settings.method.clone(),
                };
            }
        }
        
        SecurityCheck::Passed
    }
    
    /// Verify 2FA code
    pub fn verify_2fa(&self, code: &str) -> bool {
        // In production: Verify TOTP/SMS/Email
        // For testing: Accept "123456" as valid
        code == "123456" || self.recent_codes.iter().any(|(c, _)| c == code)
    }
    
    /// Generate 2FA code (for testing)
    pub fn generate_code(&mut self) -> String {
        use rand::Rng;
        let code = format!("{:06}", rand::thread_rng().gen_range(0..1000000));
        let expiry = Utc::now() + Duration::minutes(5);
        self.recent_codes.push((code.clone(), expiry));
        
        // Clean expired codes
        self.recent_codes.retain(|(_, exp)| *exp > Utc::now());
        
        code
    }
    
    /// Add address to whitelist
    pub fn whitelist_address(&mut self, address: String) {
        self.whitelisted_addresses.insert(address);
    }
    
    /// Add account to whitelist
    pub fn whitelist_account(&mut self, account: String) {
        self.whitelisted_accounts.insert(account);
    }
    
    /// Remove from whitelist
    pub fn remove_address(&mut self, address: &str) {
        self.whitelisted_addresses.remove(address);
    }
    
    /// Check limits
    pub fn check_limits(
        &self,
        amount: Decimal,
        today_withdrawn: Decimal,
        this_month_withdrawn: Decimal,
    ) -> std::result::Result<(), String> {
        // Check per-transaction limit
        if amount > self.limits.per_transaction {
            return Err(format!(
                "Amount {} exceeds per-transaction limit {}",
                amount, self.limits.per_transaction
            ));
        }
        
        // Check daily limit
        if today_withdrawn + amount > self.limits.daily_usd {
            return Err(format!(
                "Would exceed daily limit. Available: {}",
                self.limits.daily_usd - today_withdrawn
            ));
        }
        
        // Check monthly limit
        if this_month_withdrawn + amount > self.limits.monthly_usd {
            return Err(format!(
                "Would exceed monthly limit. Available: {}",
                self.limits.monthly_usd - this_month_withdrawn
            ));
        }
        
        Ok(())
    }
    
    /// Check if address is blacklisted (always returns false in paper trading)
    pub async fn is_blacklisted(&self, _address: &str) -> bool {
        // In production: Check against Chainalysis, Elliptic, etc.
        false
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_security_manager_new() {
        let security = SecurityManager::new();
        assert!(security.tfa_settings.enabled);
    }
    
    #[test]
    fn test_2fa_verification() {
        let security = SecurityManager::new();
        
        // Test code verification
        assert!(security.verify_2fa("123456")); // Test code
        assert!(!security.verify_2fa("000000"));
        assert!(!security.verify_2fa("invalid"));
    }
    
    #[test]
    fn test_whitelist() {
        let mut security = SecurityManager::new();
        
        let address = "0x1234567890abcdef".to_string();
        security.whitelist_address(address.clone());
        
        assert!(security.whitelisted_addresses.contains(&address));
        
        security.remove_address(&address);
        assert!(!security.whitelisted_addresses.contains(&address));
    }
    
    #[test]
    fn test_limits_check() {
        let security = SecurityManager::new();
        
        // Should pass - within limits
        assert!(security.check_limits(
            Decimal::from(10000),
            Decimal::from(50000),
            Decimal::from(500000)
        ).is_ok());
        
        // Should fail - exceeds per-transaction
        assert!(security.check_limits(
            Decimal::from(60000), // > $50k per tx
            Decimal::ZERO,
            Decimal::ZERO
        ).is_err());
        
        // Should fail - exceeds daily
        assert!(security.check_limits(
            Decimal::from(10000),
            Decimal::from(95000), // Already $95k today
            Decimal::ZERO
        ).is_err());
    }
    
    #[test]
    fn test_withdrawal_requires_2fa_above_threshold() {
        let security = SecurityManager::new();
        
        let destination = WithdrawalDestination::CryptoWallet {
            address: "0xunknown".to_string(),
            chain: "ETH".to_string(),
        };
        
        // Below threshold - should pass
        let check = security.check_withdrawal_requirements(
            Decimal::from(5000), // < $10k threshold
            Currency::USD,
            &destination
        );
        
        assert!(matches!(check, SecurityCheck::Passed));
        
        // Above threshold, not whitelisted - requires 2FA
        let check = security.check_withdrawal_requirements(
            Decimal::from(50000), // > $10k threshold
            Currency::USD,
            &destination
        );
        
        assert!(matches!(check, SecurityCheck::Requires2FA { .. }));
    }
    
    #[test]
    fn test_whitelisted_destination_skips_2fa() {
        let mut security = SecurityManager::new();
        let address = "0xknown".to_string();
        security.whitelist_address(address.clone());
        
        let destination = WithdrawalDestination::CryptoWallet {
            address,
            chain: "ETH".to_string(),
        };
        
        // Even above threshold, whitelisted passes
        let check = security.check_withdrawal_requirements(
            Decimal::from(50000),
            Currency::USD,
            &destination
        );
        
        assert!(matches!(check, SecurityCheck::Passed));
    }
}
