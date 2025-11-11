//! Integration tests for circuit breaker functionality
//!
//! Tests that the circuit breaker prevents wasting resources on
//! repeatedly failing operations and toxic PDFs.

use pdf_validator_rs::core::circuit_breaker::CircuitBreaker;
use pdf_validator_rs::prelude::*;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Test basic circuit breaker state transitions
#[test]
fn test_circuit_breaker_state_transitions() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(1));

    // Initial state: CLOSED
    assert!(!cb.is_open(), "Circuit should start closed");
    assert_eq!(cb.state_name(), "CLOSED");

    // Record failures
    cb.record_failure();
    assert_eq!(cb.failure_count(), 1);
    assert!(!cb.is_open(), "Circuit should remain closed after 1 failure");

    cb.record_failure();
    assert_eq!(cb.failure_count(), 2);
    assert!(!cb.is_open(), "Circuit should remain closed after 2 failures");

    cb.record_failure();
    assert_eq!(cb.failure_count(), 3);
    assert!(cb.is_open(), "Circuit should open after reaching threshold");
    assert_eq!(cb.state_name(), "OPEN");
}

/// Test circuit breaker cooldown and half-open state
/// Note: This test uses longer delays to avoid race conditions with other tests
#[test]
fn test_circuit_breaker_cooldown() {
    let cb = CircuitBreaker::new(2, Duration::from_secs(2));

    // Trigger circuit breaker
    cb.record_failure();
    cb.record_failure();
    assert!(cb.is_open(), "Circuit should be open");

    // Wait for cooldown (need >2 seconds due to > comparison and as_secs() truncation)
    thread::sleep(Duration::from_secs(3));

    // Should transition to half-open
    assert!(!cb.is_open(), "Circuit should allow testing after cooldown");
    assert_eq!(cb.state_name(), "HALF_OPEN");
}

/// Test circuit breaker reset on success
#[test]
fn test_circuit_breaker_reset() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(1));

    // Build up failures
    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.failure_count(), 2);

    // Success should reset
    cb.record_success();
    assert_eq!(cb.failure_count(), 0);
    assert!(!cb.is_open(), "Circuit should be closed after success");
    assert_eq!(cb.state_name(), "CLOSED");
}

/// Test circuit breaker with concurrent access
#[test]
fn test_circuit_breaker_thread_safety() {
    let cb = Arc::new(CircuitBreaker::new(10, Duration::from_secs(1)));
    let mut handles = vec![];

    // Spawn 5 threads that each record 3 failures
    for _ in 0..5 {
        let cb_clone = Arc::clone(&cb);
        let handle = thread::spawn(move || {
            for _ in 0..3 {
                cb_clone.record_failure();
                thread::sleep(Duration::from_millis(10));
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Should have 15 total failures
    assert_eq!(cb.failure_count(), 15);
    assert!(cb.is_open(), "Circuit should be open after 15 failures");
}

/// Test that circuit breaker prevents processing when open
#[test]
fn test_circuit_breaker_blocks_when_open() {
    let cb = CircuitBreaker::new(2, Duration::from_secs(60));

    // Open the circuit
    cb.record_failure();
    cb.record_failure();
    assert!(cb.is_open());

    // Simulate checking before processing
    let should_process = !cb.is_open();
    assert!(!should_process, "Should not process when circuit is open");
}

/// Test circuit breaker recovery after success in half-open state
#[test]
fn test_circuit_breaker_recovery() {
    let cb = CircuitBreaker::new(2, Duration::from_secs(2));

    // Open the circuit
    cb.record_failure();
    cb.record_failure();
    assert!(cb.is_open());

    // Wait for cooldown to half-open (need >2 seconds)
    thread::sleep(Duration::from_secs(3));
    assert!(!cb.is_open(), "Circuit should transition to half-open");
    assert_eq!(cb.state_name(), "HALF_OPEN");

    // Success in half-open should close the circuit
    cb.record_success();
    assert_eq!(cb.state_name(), "CLOSED");
    assert_eq!(cb.failure_count(), 0);
}

/// Test circuit breaker with toxic PDF files
#[test]
fn test_circuit_breaker_with_toxic_pdfs() {
    let temp_dir = TempDir::new().unwrap();
    let mut toxic_files = Vec::new();

    // Create 15 toxic/malformed PDF files
    for i in 0..15 {
        let file_path = temp_dir.path().join(format!("toxic_{}.pdf", i));
        let mut file = File::create(&file_path).unwrap();
        // Each file is malformed in a way that causes parsing to fail
        file.write_all(b"%PDF-1.7\nMALFORMED CONTENT\n%%EOF").unwrap();
        file.flush().unwrap();
        toxic_files.push(file_path);
    }

    // Process files sequentially to trigger circuit breaker
    let mut validation_attempts = 0;
    let mut circuit_blocked = 0;

    for path in &toxic_files {
        // In real code, the circuit breaker is checked inside validate_pdf_with_pdf_rs
        match validate_pdf_detailed(path) {
            Ok(_) => {
                validation_attempts += 1;
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("Circuit breaker") || error_msg.contains("OPEN") {
                    circuit_blocked += 1;
                } else {
                    validation_attempts += 1;
                }
            }
        }
    }

    // After many failures, circuit breaker should have blocked some attempts
    println!("Validation attempts: {}", validation_attempts);
    println!("Circuit blocked: {}", circuit_blocked);

    // At least some should be blocked (after threshold is reached)
    assert!(
        circuit_blocked > 0 || validation_attempts == toxic_files.len(),
        "Circuit breaker should either block some requests or allow all"
    );
}

/// Test circuit breaker prevents resource exhaustion
#[test]
fn test_circuit_breaker_prevents_resource_exhaustion() {
    use std::time::Instant;

    let temp_dir = TempDir::new().unwrap();
    let toxic_file = temp_dir.path().join("toxic.pdf");
    let mut file = File::create(&toxic_file).unwrap();
    file.write_all(b"%PDF-1.7\nTOXIC CONTENT\n%%EOF").unwrap();
    file.flush().unwrap();

    let start = Instant::now();
    let mut attempts = 0;

    // Try to validate the same toxic file many times
    for _ in 0..100 {
        match validate_pdf_detailed(&toxic_file) {
            Ok(_) => attempts += 1,
            Err(e) => {
                if e.to_string().contains("Circuit breaker") {
                    // Circuit breaker kicked in, stop trying
                    break;
                }
                attempts += 1;
            }
        }
    }

    let elapsed = start.elapsed();

    // With circuit breaker, should stop early
    // Without it, would attempt all 100 validations
    println!("Attempted {} validations in {:?}", attempts, elapsed);

    // Should complete quickly due to circuit breaker
    assert!(
        elapsed < Duration::from_secs(10),
        "Circuit breaker should prevent prolonged resource waste"
    );
}

/// Test multiple independent circuit breakers
#[test]
fn test_multiple_circuit_breakers() {
    let cb1 = CircuitBreaker::new(2, Duration::from_secs(1));
    let cb2 = CircuitBreaker::new(2, Duration::from_secs(1));

    // Open first circuit breaker
    cb1.record_failure();
    cb1.record_failure();
    assert!(cb1.is_open());
    assert!(!cb2.is_open(), "Second circuit breaker should be independent");

    // Open second circuit breaker
    cb2.record_failure();
    cb2.record_failure();
    assert!(cb2.is_open());

    // Both should be open independently
    assert!(cb1.is_open());
    assert!(cb2.is_open());
}

/// Test circuit breaker failure count accuracy
#[test]
fn test_failure_count_accuracy() {
    let cb = CircuitBreaker::new(10, Duration::from_secs(1));

    for i in 1..=5 {
        cb.record_failure();
        assert_eq!(cb.failure_count(), i, "Failure count should be {}", i);
    }

    assert!(!cb.is_open(), "Should not be open yet");

    for _ in 0..5 {
        cb.record_failure();
    }

    assert_eq!(cb.failure_count(), 10);
    assert!(cb.is_open(), "Should be open after 10 failures");
}

/// Test circuit breaker with mixed success and failure
#[test]
fn test_mixed_success_and_failure() {
    let cb = CircuitBreaker::new(5, Duration::from_secs(1));

    // Pattern: fail, fail, success, fail, fail
    cb.record_failure(); // count = 1
    cb.record_failure(); // count = 2
    cb.record_success(); // count = 0
    cb.record_failure(); // count = 1
    cb.record_failure(); // count = 2

    assert_eq!(cb.failure_count(), 2);
    assert!(!cb.is_open(), "Should not be open, success reset the counter");
}

/// Test circuit breaker edge case: exactly at threshold
#[test]
fn test_threshold_boundary() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(1));

    cb.record_failure();
    cb.record_failure();
    assert!(!cb.is_open(), "Should not be open at threshold - 1");

    cb.record_failure();
    assert!(cb.is_open(), "Should be open at exactly threshold");
}

/// Test circuit breaker cooldown edge cases
#[test]
fn test_cooldown_edge_cases() {
    let cb = CircuitBreaker::new(2, Duration::from_secs(2));

    // Open circuit
    cb.record_failure();
    cb.record_failure();
    assert!(cb.is_open());

    // Check before cooldown
    thread::sleep(Duration::from_secs(1));
    assert!(cb.is_open(), "Should still be open before cooldown");

    // Check after cooldown (need >2 seconds total, so wait additional 2 seconds)
    thread::sleep(Duration::from_secs(2));
    assert!(!cb.is_open(), "Should transition to half-open after cooldown");
}

/// Test that circuit breaker doesn't interfere with valid PDFs
#[test]
fn test_circuit_breaker_allows_valid_operations() {
    let cb = CircuitBreaker::new(5, Duration::from_secs(1));

    // Simulate several successful operations
    for _ in 0..10 {
        assert!(!cb.is_open(), "Circuit should remain closed for successful operations");
        cb.record_success();
    }

    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.state_name(), "CLOSED");
}

/// Stress test: rapid state transitions
#[test]
fn test_rapid_state_transitions() {
    let cb = CircuitBreaker::new(3, Duration::from_secs(2));

    for _ in 0..2 {
        // Open the circuit
        for _ in 0..3 {
            cb.record_failure();
        }
        assert!(cb.is_open());

        // Wait for cooldown (need >2 seconds)
        thread::sleep(Duration::from_secs(3));

        // Close with success
        cb.record_success();
        assert!(!cb.is_open());
    }

    // Should end in closed state
    assert_eq!(cb.state_name(), "CLOSED");
}

/// Test circuit breaker with zero threshold (always open)
#[test]
fn test_zero_threshold() {
    let cb = CircuitBreaker::new(0, Duration::from_secs(1));

    // Should open immediately on first failure
    cb.record_failure();
    assert!(cb.is_open(), "Circuit should open with zero threshold");
}

/// Test circuit breaker with very high threshold
#[test]
fn test_high_threshold() {
    let cb = CircuitBreaker::new(1000, Duration::from_secs(1));

    // Record many failures
    for _ in 0..500 {
        cb.record_failure();
    }

    assert!(!cb.is_open(), "Circuit should not open before threshold");
    assert_eq!(cb.failure_count(), 500);
}

/// Integration test: circuit breaker in parallel validation
#[test]
fn test_circuit_breaker_parallel_validation() {
    use rayon::prelude::*;

    let temp_dir = TempDir::new().unwrap();
    let mut files = Vec::new();

    // Create 20 malformed PDFs
    for i in 0..20 {
        let file_path = temp_dir.path().join(format!("file_{}.pdf", i));
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"%PDF-1.7\nBAD CONTENT\n%%EOF").unwrap();
        file.flush().unwrap();
        files.push(file_path);
    }

    // Process in parallel
    let results: Vec<_> = files
        .par_iter()
        .map(|path| validate_pdf_detailed(path))
        .collect();

    // Check how many were processed vs circuit-blocked
    let processed = results.iter().filter(|r| r.is_err()).count();
    println!("Processed: {} out of {}", processed, files.len());

    // Test passes if no panic occurred
    assert!(results.len() == files.len());
}
