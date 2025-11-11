use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CLOSED: u8 = 0;
const OPEN: u8 = 1;
const HALF_OPEN: u8 = 2;

/// Circuit breaker to prevent repeatedly processing toxic PDFs
/// 
/// States:
/// - CLOSED: Normal operation, all requests allowed
/// - OPEN: Too many failures, reject all requests
/// - HALF_OPEN: Testing if issue resolved after cooldown
pub struct CircuitBreaker {
    state: AtomicU8,
    failure_count: AtomicUsize,
    failure_threshold: usize,
    last_failure_time: AtomicU64,
    cooldown_duration: Duration,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    /// 
    /// # Arguments
    /// * `failure_threshold` - Number of consecutive failures before opening (e.g., 10)
    /// * `cooldown_duration` - Time to wait before attempting recovery (e.g., 60 seconds)
    pub fn new(failure_threshold: usize, cooldown_duration: Duration) -> Self {
        Self {
            state: AtomicU8::new(CLOSED),
            failure_count: AtomicUsize::new(0),
            failure_threshold,
            last_failure_time: AtomicU64::new(0),
            cooldown_duration,
        }
    }
    
    /// Check if circuit breaker is currently open (rejecting requests)
    pub fn is_open(&self) -> bool {
        let state = self.state.load(Ordering::Acquire);
        if state == OPEN {
            // Check if cooldown period has elapsed
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let last_failure = self.last_failure_time.load(Ordering::Acquire);
            
            if now.saturating_sub(last_failure) > self.cooldown_duration.as_secs() {
                // Transition to half-open to test recovery
                self.state.store(HALF_OPEN, Ordering::Release);
                false
            } else {
                true
            }
        } else {
            false
        }
    }
    
    /// Record a successful operation
    /// Resets failure count and closes circuit if in half-open state
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Release);
        self.state.store(CLOSED, Ordering::Release);
    }
    
    /// Record a failed operation
    /// Increments failure count and opens circuit if threshold exceeded
    pub fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::AcqRel) + 1;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_failure_time.store(now, Ordering::Release);
        
        if failures >= self.failure_threshold {
            self.state.store(OPEN, Ordering::Release);
        }
    }
    
    /// Get current failure count
    pub fn failure_count(&self) -> usize {
        self.failure_count.load(Ordering::Acquire)
    }
    
    /// Get current state as string for logging
    pub fn state_name(&self) -> &'static str {
        match self.state.load(Ordering::Acquire) {
            CLOSED => "CLOSED",
            OPEN => "OPEN",
            HALF_OPEN => "HALF_OPEN",
            _ => "UNKNOWN",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        
        assert!(!cb.is_open());
        
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_open());
        
        cb.record_failure();
        assert!(cb.is_open());
    }
    
    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);
        
        cb.record_success();
        assert_eq!(cb.failure_count(), 0);
        assert!(!cb.is_open());
    }
    
    #[test]
    fn test_circuit_breaker_transitions_to_half_open() {
        let cb = CircuitBreaker::new(2, Duration::from_secs(1));
        
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_open());
        
        // Wait for cooldown
        thread::sleep(Duration::from_secs(2));
        
        // Should transition to half-open
        assert!(!cb.is_open());
        assert_eq!(cb.state_name(), "HALF_OPEN");
    }
}
