//! Treasury Security - 2FA, Whitelists, Limits, Sanctions Screening

use super::{Currency, Decimal, WithdrawalDestination};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashSet;

type HmacSha256 = Hmac<Sha256>;

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
    /// TOTP secret for HMAC-based OTP verification (base32-encoded)
    totp_secret: Option<Vec<u8>>,
    /// Blacklisted addresses for sanctions screening
    blacklisted_addresses: HashSet<String>,
    /// Blacklisted address prefixes (partial match)
    blacklisted_prefixes: Vec<String>,
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
    Totp,        // Time-based OTP (Google Authenticator)
    Sms,         // SMS code
    Email,       // Email code
    HardwareKey, // YubiKey, etc.
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
        let totp_secret = std::env::var("TREASURY_TOTP_SECRET")
            .ok()
            .and_then(|s| Self::decode_base32(&s));

        let mut blacklisted_addresses = HashSet::new();
        let mut blacklisted_prefixes = Vec::new();

        // Load blacklist from environment (comma-separated)
        if let Ok(list) = std::env::var("TREASURY_BLACKLISTED_ADDRESSES") {
            for addr in list.split(',') {
                let addr = addr.trim().to_lowercase();
                if !addr.is_empty() {
                    blacklisted_addresses.insert(addr);
                }
            }
        }
        if let Ok(list) = std::env::var("TREASURY_BLACKLISTED_PREFIXES") {
            for prefix in list.split(',') {
                let prefix = prefix.trim().to_lowercase();
                if !prefix.is_empty() {
                    blacklisted_prefixes.push(prefix);
                }
            }
        }

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
            totp_secret,
            blacklisted_addresses,
            blacklisted_prefixes,
        }
    }

    /// Decode a base32-encoded string to bytes
    fn decode_base32(input: &str) -> Option<Vec<u8>> {
        let input = input.trim().to_uppercase();
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut bits = 0u64;
        let mut bit_count = 0u8;
        let mut output = Vec::new();

        for &byte in input.as_bytes() {
            if byte == b'=' {
                break;
            }
            let val = alphabet.iter().position(|&c| c == byte)? as u64;
            bits = (bits << 5) | val;
            bit_count += 5;
            if bit_count >= 8 {
                bit_count -= 8;
                output.push((bits >> bit_count) as u8);
                bits &= (1 << bit_count) - 1;
            }
        }
        Some(output)
    }

    /// Generate TOTP code for current time window
    fn generate_totp(secret: &[u8], time_step: u64) -> String {
        let time_bytes = time_step.to_be_bytes();
        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
        mac.update(&time_bytes);
        let result = mac.finalize().into_bytes();

        let offset = (result[result.len() - 1] & 0x0f) as usize;
        let code = ((result[offset] as u32 & 0x7f) << 24)
            | ((result[offset + 1] as u32) << 16)
            | ((result[offset + 2] as u32) << 8)
            | (result[offset + 3] as u32);

        format!("{:06}", code % 1_000_000)
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

    /// Verify 2FA code against TOTP secret or recent generated codes.
    /// Checks current time window and one adjacent window to handle clock skew.
    pub fn verify_2fa(&self, code: &str) -> bool {
        // Validate code format: must be exactly 6 digits
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // Check against recently generated codes (for SMS/Email method)
        let now = Utc::now();
        if self
            .recent_codes
            .iter()
            .any(|(c, exp)| c == code && *exp > now)
        {
            return true;
        }

        // Check against TOTP secret if configured
        if let Some(ref secret) = self.totp_secret {
            let time_step = (now.timestamp() as u64) / 30;
            // Check current window and ±1 window for clock skew tolerance
            for offset in [0, 1, u64::MAX] {
                let step = time_step.wrapping_add(offset);
                if Self::generate_totp(secret, step) == code {
                    return true;
                }
            }
        }

        false
    }

    /// Generate a random 2FA code and store it with expiry
    pub fn generate_code(&mut self) -> String {
        use rand::Rng;
        let code = format!("{:06}", rand::thread_rng().gen_range(0..1000000));
        let expiry = Utc::now() + Duration::minutes(5);
        self.recent_codes.push((code.clone(), expiry));

        // Clean expired codes
        self.recent_codes.retain(|(_, exp)| *exp > Utc::now());

        code
    }

    /// Configure TOTP secret for this security manager
    pub fn set_totp_secret(&mut self, secret_base32: &str) {
        self.totp_secret = Self::decode_base32(secret_base32);
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

    /// Add address to blacklist
    pub fn blacklist_address(&mut self, address: String) {
        self.blacklisted_addresses.insert(address.to_lowercase());
    }

    /// Add prefix to blacklist (partial matching)
    pub fn blacklist_prefix(&mut self, prefix: String) {
        self.blacklisted_prefixes.push(prefix.to_lowercase());
    }

    /// Check if address is blacklisted against the configured deny-list.
    /// Checks exact match and prefix match.
    pub async fn is_blacklisted(&self, address: &str) -> bool {
        let addr_lower = address.trim().to_lowercase();

        // Exact match
        if self.blacklisted_addresses.contains(&addr_lower) {
            return true;
        }

        // Prefix match
        for prefix in &self.blacklisted_prefixes {
            if addr_lower.starts_with(prefix) {
                return true;
            }
        }

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
        assert_eq!(security.tfa_settings.threshold, Decimal::from(10000));
    }

    #[test]
    fn test_2fa_rejects_hardcoded_bypass() {
        let security = SecurityManager::new();
        // "123456" must NOT be accepted without TOTP secret or generated code
        assert!(!security.verify_2fa("123456"));
    }

    #[test]
    fn test_2fa_rejects_invalid_format() {
        let security = SecurityManager::new();
        assert!(!security.verify_2fa(""));
        assert!(!security.verify_2fa("12345")); // too short
        assert!(!security.verify_2fa("1234567")); // too long
        assert!(!security.verify_2fa("abcdef")); // non-digit
        assert!(!security.verify_2fa("12 456")); // contains space
    }

    #[test]
    fn test_2fa_with_generated_code() {
        let mut security = SecurityManager::new();
        let code = security.generate_code();

        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
        assert!(security.verify_2fa(&code));

        // Wrong code should fail
        let wrong = if code == "000000" {
            "111111".to_string()
        } else {
            "000000".to_string()
        };
        assert!(!security.verify_2fa(&wrong));
    }

    #[test]
    fn test_2fa_totp_verification() {
        let mut security = SecurityManager::new();
        // Set a known TOTP secret (base32: JBSWY3DPEHPK3PXP)
        security.set_totp_secret("JBSWY3DPEHPK3PXP");
        assert!(security.totp_secret.is_some());

        // Generate the expected TOTP for the current time step
        let time_step = (Utc::now().timestamp() as u64) / 30;
        let expected =
            SecurityManager::generate_totp(security.totp_secret.as_ref().unwrap(), time_step);

        assert_eq!(expected.len(), 6);
        assert!(security.verify_2fa(&expected));
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
    fn test_limits_check_within_bounds() {
        let security = SecurityManager::new();
        let result = security.check_limits(
            Decimal::from(10000),
            Decimal::from(50000),
            Decimal::from(500000),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_limits_check_exceeds_per_transaction() {
        let security = SecurityManager::new();
        let result = security.check_limits(
            Decimal::from(60000), // > $50k per tx limit
            Decimal::ZERO,
            Decimal::ZERO,
        );
        let err = result.unwrap_err();
        assert!(err.contains("per-transaction limit"));
    }

    #[test]
    fn test_limits_check_exceeds_daily() {
        let security = SecurityManager::new();
        let result = security.check_limits(
            Decimal::from(10000),
            Decimal::from(95000), // Already $95k today
            Decimal::ZERO,
        );
        let err = result.unwrap_err();
        assert!(err.contains("daily limit"));
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
            Decimal::from(5000),
            Currency::USD,
            &destination,
        );
        assert!(matches!(check, SecurityCheck::Passed));

        // Above threshold, not whitelisted - requires 2FA
        let check = security.check_withdrawal_requirements(
            Decimal::from(50000),
            Currency::USD,
            &destination,
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

        let check = security.check_withdrawal_requirements(
            Decimal::from(50000),
            Currency::USD,
            &destination,
        );
        assert!(matches!(check, SecurityCheck::Passed));
    }

    #[tokio::test]
    async fn test_blacklist_exact_match() {
        let mut security = SecurityManager::new();
        security.blacklist_address("0xbadactor123".to_string());

        assert!(security.is_blacklisted("0xbadactor123").await);
        assert!(security.is_blacklisted("0xBADACTOR123").await); // case-insensitive
        assert!(!security.is_blacklisted("0xgoodactor456").await);
    }

    #[tokio::test]
    async fn test_blacklist_prefix_match() {
        let mut security = SecurityManager::new();
        security.blacklist_prefix("0xdead".to_string());

        assert!(security.is_blacklisted("0xdead0000000000").await);
        assert!(security.is_blacklisted("0xDEADbeef").await); // case-insensitive
        assert!(!security.is_blacklisted("0xalive0000000").await);
    }

    #[tokio::test]
    async fn test_blacklist_empty_address() {
        let security = SecurityManager::new();
        assert!(!security.is_blacklisted("").await);
    }

    #[test]
    fn test_base32_decode() {
        // "JBSWY3DPEHPK3PXP" decodes to "Hello!ÞP" (known test vector)
        let result = SecurityManager::decode_base32("JBSWY3DPEHPK3PXP");
        assert!(result.is_some());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(bytes[0], b'H');
        assert_eq!(bytes[1], b'e');
    }
}
