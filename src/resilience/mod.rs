//! Resilience patterns for production
//!
//! Circuit breakers, bulkheads, and retry policies for external API calls

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Failing - requests are blocked
    Open,
    /// Testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Success threshold to close circuit from half-open
    pub success_threshold: u32,
    /// Timeout before attempting half-open
    pub timeout_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_duration: Duration::from_secs(30),
        }
    }
}

/// Circuit breaker for external API calls
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: std::sync::RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64, // Unix timestamp
}

/// Result of a circuit breaker check
#[derive(Debug, Clone, Copy)]
pub enum CircuitResult {
    /// Request allowed, execute the operation
    Allowed,
    /// Circuit is open, request blocked
    Open { retry_after: Duration },
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Arc<Self> {
        Arc::new(Self {
            name: name.into(),
            config,
            state: std::sync::RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
        })
    }

    /// Check if request should be allowed
    pub fn check(&self) -> CircuitResult {
        let state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => CircuitResult::Allowed,
            CircuitState::Open => {
                let last_failure = self.last_failure_time.load(Ordering::SeqCst);
                let now = Instant::now().elapsed().as_secs();
                let elapsed = Duration::from_secs(now - last_failure);

                if elapsed >= self.config.timeout_duration {
                    // Transition to half-open
                    *self.state.write().unwrap() = CircuitState::HalfOpen;
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                    info!("Circuit '{}' transitioning to HalfOpen", self.name);
                    CircuitResult::Allowed
                } else {
                    CircuitResult::Open {
                        retry_after: self.config.timeout_duration - elapsed,
                    }
                }
            }
            CircuitState::HalfOpen => CircuitResult::Allowed,
        }
    }

    /// Record a successful call
    pub fn record_success(&self) {
        let state = *self.state.read().unwrap();

        match state {
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if successes >= self.config.success_threshold {
                    *self.state.write().unwrap() = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                    info!(
                        "Circuit '{}' closed after {} successes",
                        self.name, successes
                    );
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        let state = *self.state.read().unwrap();
        let now = Instant::now().elapsed().as_secs();
        self.last_failure_time.store(now, Ordering::SeqCst);

        match state {
            CircuitState::Closed | CircuitState::HalfOpen => {
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if failures >= self.config.failure_threshold {
                    *self.state.write().unwrap() = CircuitState::Open;
                    warn!("Circuit '{}' opened after {} failures", self.name, failures);
                }
            }
            _ => {}
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitState {
        *self.state.read().unwrap()
    }

    /// Execute a function with circuit breaker protection
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        match self.check() {
            CircuitResult::Allowed => match f().await {
                Ok(result) => {
                    self.record_success();
                    Ok(result)
                }
                Err(e) => {
                    self.record_failure();
                    Err(CircuitError::Inner(e))
                }
            },
            CircuitResult::Open { retry_after } => Err(CircuitError::Open {
                circuit: self.name.clone(),
                retry_after,
            }),
        }
    }
}

/// Circuit breaker errors
#[derive(Debug, Clone)]
pub enum CircuitError<E> {
    /// Circuit is open
    Open {
        circuit: String,
        retry_after: Duration,
    },
    /// Inner operation failed
    Inner(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitError::Open {
                circuit,
                retry_after,
            } => {
                write!(
                    f,
                    "Circuit '{}' is open, retry after {:?}",
                    circuit, retry_after
                )
            }
            CircuitError::Inner(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error> std::error::Error for CircuitError<E> {}

/// Collection of circuit breakers for different services
pub struct CircuitBreakerRegistry {
    breakers: std::sync::RwLock<std::collections::HashMap<String, Arc<CircuitBreaker>>>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    /// Create a new registry
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            breakers: std::sync::RwLock::new(std::collections::HashMap::new()),
            default_config: CircuitBreakerConfig::default(),
        })
    }

    /// Get or create a circuit breaker
    pub fn get_or_create(&self, name: &str) -> Arc<CircuitBreaker> {
        {
            let breakers = self.breakers.read().unwrap();
            if let Some(cb) = breakers.get(name) {
                return cb.clone();
            }
        }

        let mut breakers = self.breakers.write().unwrap();
        // Double-check
        if let Some(cb) = breakers.get(name) {
            return cb.clone();
        }

        let cb = CircuitBreaker::new(name, self.default_config.clone());
        breakers.insert(name.to_string(), cb.clone());
        cb
    }

    /// Get circuit breaker state
    pub fn get_state(&self, name: &str) -> Option<CircuitState> {
        let breakers = self.breakers.read().unwrap();
        breakers.get(name).map(|cb| cb.state())
    }

    /// Reset a circuit breaker
    pub fn reset(&self, name: &str) {
        let breakers = self.breakers.write().unwrap();
        if let Some(cb) = breakers.get(name) {
            *cb.state.write().unwrap() = CircuitState::Closed;
            cb.failure_count.store(0, Ordering::SeqCst);
            cb.success_count.store(0, Ordering::SeqCst);
            info!("Circuit '{}' manually reset", name);
        }
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self {
            breakers: std::sync::RwLock::new(std::collections::HashMap::new()),
            default_config: CircuitBreakerConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_creation() {
        let cb = CircuitBreaker::new("test", CircuitBreakerConfig::default());
        assert!(matches!(cb.state(), CircuitState::Closed));
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let cb = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 2,
                timeout_duration: Duration::from_secs(60),
            },
        );

        // Initially closed
        assert!(matches!(cb.check(), CircuitResult::Allowed));

        // Record failures
        cb.record_failure();
        cb.record_failure();
        assert!(matches!(cb.state(), CircuitState::Closed));

        // Third failure opens circuit
        cb.record_failure();
        assert!(matches!(cb.state(), CircuitState::Open));
    }

    #[test]
    fn test_circuit_records_success() {
        let cb = CircuitBreaker::new("test", CircuitBreakerConfig::default());

        cb.record_success();
        assert!(matches!(cb.state(), CircuitState::Closed));
    }

    #[tokio::test]
    async fn test_circuit_call_success() {
        let cb = CircuitBreaker::new("test", CircuitBreakerConfig::default());

        let result: Result<i32, CircuitError<&str>> = cb.call(|| async { Ok(42) }).await;

        assert_eq!(result.unwrap(), 42);
        assert!(matches!(cb.state(), CircuitState::Closed));
    }

    #[tokio::test]
    async fn test_circuit_call_failure() {
        let cb = CircuitBreaker::new(
            "test",
            CircuitBreakerConfig {
                failure_threshold: 1,
                success_threshold: 1,
                timeout_duration: Duration::from_secs(60),
            },
        );

        // First call fails
        let result: Result<i32, CircuitError<&str>> = cb.call(|| async { Err("error") }).await;

        assert!(result.is_err());
        assert!(matches!(cb.state(), CircuitState::Open));

        // Subsequent calls are blocked
        let result: Result<i32, CircuitError<&str>> = cb.call(|| async { Ok(42) }).await;

        assert!(matches!(result, Err(CircuitError::Open { .. })));
    }

    #[test]
    fn test_registry() {
        let registry = CircuitBreakerRegistry::new();

        let cb1 = registry.get_or_create("service-a");
        let cb2 = registry.get_or_create("service-a");
        let cb3 = registry.get_or_create("service-b");

        // Same name returns same instance
        assert!(Arc::ptr_eq(&cb1, &cb2));
        // Different name returns different instance
        assert!(!Arc::ptr_eq(&cb1, &cb3));
    }
}
