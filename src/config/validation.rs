//! Startup environment validation (Sprint 106).
//!
//! Checks required and optional environment variables at boot.
//! Fails fast with a clear error message if critical vars are missing.

use tracing::{info, warn};

/// Required environment variables — startup fails if any are missing.
const REQUIRED_VARS: &[(&str, &str)] = &[("DATABASE_URL", "PostgreSQL connection string")];

/// Optional variables that warrant a warning if missing.
const OPTIONAL_WARNED_VARS: &[(&str, &str, &str)] = &[
    (
        "JWT_SECRET",
        "JWT signing key",
        "a random default will be used (INSECURE for production)",
    ),
    (
        "REDIS_URL",
        "Redis connection",
        "rate limiting will be disabled",
    ),
    (
        "ADMIN_EMAIL",
        "Initial admin email",
        "admin seeding will be skipped",
    ),
];

/// Informational optional variables (logged at debug level).
const OPTIONAL_VARS: &[(&str, &str)] = &[
    ("SERVER_HOST", "127.0.0.1"),
    ("SERVER_PORT", "8080"),
    ("DB_POOL_MAX_CONNECTIONS", "30"),
    ("DB_POOL_MIN_CONNECTIONS", "5"),
    ("AUTH_MAX_FAILED_LOGINS", "5"),
    ("AUTH_LOCKOUT_DURATION_SECS", "900"),
    ("RATE_LIMIT_LOGIN_MAX", "10"),
    ("RATE_LIMIT_LOGIN_WINDOW_SECS", "300"),
    ("SESSION_CLEANUP_INTERVAL_SECS", "3600"),
    ("SESSION_RETENTION_HOURS", "24"),
    ("ENVIRONMENT", "development"),
];

/// Validate environment at startup. Panics if required vars are missing.
pub fn validate_env() {
    let mut errors = Vec::new();

    // Check required vars
    for (var, description) in REQUIRED_VARS {
        if std::env::var(var).is_err() {
            errors.push(format!("  - {var}: {description}"));
        }
    }

    if !errors.is_empty() {
        let msg = format!(
            "\n\nMissing required environment variables:\n{}\n\n\
             Set these variables before starting the server.\n\
             Example: export DATABASE_URL=postgres://user:pass@localhost:5432/investor_os\n",
            errors.join("\n")
        );
        panic!("{msg}");
    }

    // Warn about important optional vars
    for (var, description, consequence) in OPTIONAL_WARNED_VARS {
        if std::env::var(var).is_err() {
            warn!("{var} not set ({description}): {consequence}");
        }
    }

    // Log optional var status
    let mut configured = Vec::new();
    let mut defaulted = Vec::new();
    for (var, default_val) in OPTIONAL_VARS {
        if std::env::var(var).is_ok() {
            configured.push(*var);
        } else {
            defaulted.push(format!("{var}={default_val}"));
        }
    }

    if !configured.is_empty() {
        info!("Configured env vars: {}", configured.join(", "));
    }
    if !defaulted.is_empty() {
        info!("Using defaults: {}", defaulted.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_vars_list_is_not_empty() {
        assert!(!REQUIRED_VARS.is_empty());
    }

    #[test]
    fn optional_vars_have_defaults() {
        for (var, default) in OPTIONAL_VARS {
            assert!(!var.is_empty());
            assert!(!default.is_empty());
        }
    }

    #[test]
    fn optional_warned_vars_have_consequences() {
        for (var, desc, consequence) in OPTIONAL_WARNED_VARS {
            assert!(!var.is_empty());
            assert!(!desc.is_empty());
            assert!(!consequence.is_empty());
        }
    }
}
