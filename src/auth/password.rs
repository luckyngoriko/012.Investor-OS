use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::auth::error::AuthError;

/// Hash a plaintext password with Argon2id (OWASP recommended).
/// Returns a PHC-formatted string suitable for DB storage.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::PasswordHash(e.to_string()))?;
    Ok(hash.to_string())
}

/// Verify a plaintext password against a stored Argon2id PHC hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed = PasswordHash::new(hash).map_err(|e| AuthError::PasswordHash(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

/// Validate password meets minimum complexity requirements.
/// Returns Ok(()) if valid, Err(WeakPassword) with reason otherwise.
pub fn validate_password_policy(password: &str) -> Result<(), AuthError> {
    if password.len() < 12 {
        return Err(AuthError::WeakPassword(
            "must be at least 12 characters".to_string(),
        ));
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(AuthError::WeakPassword(
            "must contain at least one uppercase letter".to_string(),
        ));
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(AuthError::WeakPassword(
            "must contain at least one lowercase letter".to_string(),
        ));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(AuthError::WeakPassword(
            "must contain at least one digit".to_string(),
        ));
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(AuthError::WeakPassword(
            "must contain at least one special character".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let password = "hunter2-strong-password!";
        let hash = hash_password(password).unwrap();

        // PHC string starts with $argon2id$
        assert!(
            hash.starts_with("$argon2id$"),
            "Expected Argon2id PHC string, got: {hash}"
        );

        // Correct password verifies
        assert!(verify_password(password, &hash).unwrap());

        // Wrong password does not verify
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn policy_rejects_short_password() {
        let result = validate_password_policy("Short1!");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("12 characters"), "Got: {msg}");
    }

    #[test]
    fn policy_rejects_no_uppercase() {
        let result = validate_password_policy("alllowercase1!");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("uppercase"), "Got: {msg}");
    }

    #[test]
    fn policy_rejects_no_digit() {
        let result = validate_password_policy("NoDigitsHere!!");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("digit"), "Got: {msg}");
    }

    #[test]
    fn policy_rejects_no_special() {
        let result = validate_password_policy("NoSpecialChar1A");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("special"), "Got: {msg}");
    }

    #[test]
    fn policy_accepts_strong_password() {
        assert!(validate_password_policy("Strong1Pass!@#").is_ok());
    }

    #[test]
    fn different_hashes_for_same_password() {
        let password = "same-password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Different salts produce different hashes
        assert_ne!(hash1, hash2);

        // Both verify against the original password
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }
}
