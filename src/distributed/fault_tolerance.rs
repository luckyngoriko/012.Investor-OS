//! Fault Tolerance - Circuit Breaker and Retry Logic
//! Sprint 49: Distributed Inference

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BreakerState {
    /// Normal operation
    Closed,
    /// Failure threshold reached, rejecting requests
    Open,
    /// Testing if service recovered
    HalfOpen,
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    failure_threshold: u32,
    success_threshold: u32,
    reset_timeout: Duration,
    state: Mutex<BreakerState>,
    failures: AtomicU32,
    successes: AtomicU32,
    last_failure: Mutex<Option<Instant>>,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold: 3,
            reset_timeout,
            state: Mutex::new(BreakerState::Closed),
            failures: AtomicU32::new(0),
            successes: AtomicU32::new(0),
            last_failure: Mutex::new(None),
        }
    }
    
    /// Check if request can proceed
    pub fn can_execute(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        
        match *state {
            BreakerState::Closed => true,
            BreakerState::Open => {
                // Check if timeout elapsed
                if let Some(last) = *self.last_failure.lock().unwrap() {
                    if last.elapsed() >= self.reset_timeout {
                        *state = BreakerState::HalfOpen;
                        self.failures.store(0, Ordering::Relaxed);
                        return true;
                    }
                }
                false
            }
            BreakerState::HalfOpen => true,
        }
    }
    
    /// Record successful request
    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        
        match *state {
            BreakerState::HalfOpen => {
                let successes = self.successes.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.success_threshold {
                    *state = BreakerState::Closed;
                    self.failures.store(0, Ordering::Relaxed);
                    self.successes.store(0, Ordering::Relaxed);
                }
            }
            BreakerState::Closed => {
                // Reset failures on success in closed state
                self.failures.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }
    
    /// Record failed request
    pub fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure.lock().unwrap() = Some(Instant::now());
        
        let mut state = self.state.lock().unwrap();
        
        match *state {
            BreakerState::Closed => {
                if failures >= self.failure_threshold {
                    *state = BreakerState::Open;
                }
            }
            BreakerState::HalfOpen => {
                *state = BreakerState::Open;
                self.successes.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }
    
    /// Get current state
    pub fn state(&self) -> BreakerState {
        *self.state.lock().unwrap()
    }
    
    /// Check if breaker is open
    pub fn is_open(&self) -> bool {
        self.state() == BreakerState::Open
    }
}

/// Retry policy with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Create new retry policy
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }
    
    /// Calculate delay for attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }
        
        let delay_ms = self.initial_delay.as_millis() as f64 
            * self.backoff_multiplier.powi(attempt as i32 - 1);
        
        let delay = Duration::from_millis(delay_ms as u64);
        delay.min(self.max_delay)
    }
    
    /// Execute function with retry
    pub async fn retry<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let mut last_error = None;
        
        for attempt in 0..self.max_attempts {
            if attempt > 0 {
                let delay = self.delay_for_attempt(attempt);
                tokio::time::sleep(delay).await;
            }
            
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => last_error = Some(e),
            }
        }
        
        Err(last_error.unwrap())
    }
}

/// Node circuit breaker manager
pub struct NodeCircuitBreakers {
    breakers: Arc<Mutex<std::collections::HashMap<String, Arc<CircuitBreaker>>>>,
    default_config: CircuitBreakerConfig,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub reset_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
        }
    }
}

impl NodeCircuitBreakers {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            breakers: Arc::new(Mutex::new(std::collections::HashMap::new())),
            default_config: CircuitBreakerConfig::default(),
        }
    }
    
    /// Get or create breaker for node
    pub fn get(&self, node_id: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.breakers.lock().unwrap();
        
        breakers.entry(node_id.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::new(
                    self.default_config.failure_threshold,
                    self.default_config.reset_timeout,
                ))
            })
            .clone()
    }
    
    /// Remove breaker for node
    pub fn remove(&self, node_id: &str) {
        let mut breakers = self.breakers.lock().unwrap();
        breakers.remove(node_id);
    }
}

impl Default for NodeCircuitBreakers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker_closed_to_open() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(30));
        
        assert_eq!(cb.state(), BreakerState::Closed);
        assert!(cb.can_execute());
        
        // Record failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Closed);
        
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Open);
        assert!(!cb.can_execute());
    }
    
    #[test]
    fn test_circuit_breaker_success_resets() {
        let cb = CircuitBreaker::new(5, Duration::from_secs(30));
        
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failures.load(Ordering::Relaxed), 2);
        
        cb.record_success();
        assert_eq!(cb.failures.load(Ordering::Relaxed), 0);
    }
    
    #[test]
    fn test_retry_policy_delay() {
        let policy = RetryPolicy::default();
        
        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(400));
    }
    
    #[test]
    fn test_retry_policy_max_delay() {
        let policy = RetryPolicy {
            max_attempts: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 10.0,
        };
        
        // Should be capped at max_delay
        assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(5));
    }
}
