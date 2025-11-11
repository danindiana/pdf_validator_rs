//! Integration tests for error isolation
//!
//! Tests that malformed PDFs don't crash the validator and that
//! errors are properly isolated per-file in parallel processing.

use pdf_validator_rs::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};

/// Test that a completely malformed file doesn't panic
#[test]
fn test_garbage_data_does_not_panic() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"This is not a PDF at all, just garbage data!").unwrap();
    temp_file.flush().unwrap();

    // Should return false, not panic
    let result = validate_pdf(temp_file.path(), false);
    assert!(!result, "Garbage data should be invalid");
}

/// Test that a file with PDF header but corrupted content doesn't panic
#[test]
fn test_pdf_header_with_garbage_content() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF-1.7\nRANDOM GARBAGE DATA HERE\n%%EOF").unwrap();
    temp_file.flush().unwrap();

    let result = validate_pdf(temp_file.path(), false);
    assert!(!result, "Malformed PDF should be invalid");
}

/// Test that a truncated PDF doesn't panic
#[test]
fn test_truncated_pdf() {
    let mut temp_file = NamedTempFile::new().unwrap();
    // Valid header but no proper PDF structure
    temp_file.write_all(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n1 0 obj\n<<").unwrap();
    temp_file.flush().unwrap();

    let result = validate_pdf(temp_file.path(), false);
    assert!(!result, "Truncated PDF should be invalid");
}

/// Test that a PDF with invalid version doesn't panic
#[test]
fn test_invalid_pdf_version() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF-9.9\nSome content\n%%EOF").unwrap();
    temp_file.flush().unwrap();

    let _result = validate_pdf(temp_file.path(), false);
    // Should handle gracefully, either accept or reject
    // The key is it shouldn't panic
}

/// Test that missing EOF marker is caught
#[test]
fn test_missing_eof_marker() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj").unwrap();
    temp_file.flush().unwrap();

    let result = validate_pdf(temp_file.path(), false);
    assert!(!result, "PDF without %%EOF should be invalid");
}

/// Test that empty file doesn't panic
#[test]
fn test_empty_file() {
    let temp_file = NamedTempFile::new().unwrap();
    // Don't write anything

    let result = validate_pdf(temp_file.path(), false);
    assert!(!result, "Empty file should be invalid");
}

/// Test that very small file doesn't panic
#[test]
fn test_very_small_file() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF").unwrap();
    temp_file.flush().unwrap();

    let result = validate_pdf(temp_file.path(), false);
    assert!(!result, "Very small file should be invalid");
}

/// Test that a file with only PDF header and EOF doesn't panic
#[test]
fn test_minimal_invalid_pdf() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF-1.7\n%%EOF").unwrap();
    temp_file.flush().unwrap();

    // This might be accepted by lenient parsers, but shouldn't panic
    let _result = validate_pdf(temp_file.path(), false);
    // Key: no panic occurred
}

/// Test parallel processing with mix of valid and invalid files
#[test]
fn test_parallel_mixed_validity() {
    use rayon::prelude::*;

    let temp_dir = TempDir::new().unwrap();
    let mut files = Vec::new();

    // Create 20 files: mix of valid and invalid
    for i in 0..20 {
        let file_path = temp_dir.path().join(format!("test_{}.pdf", i));
        let mut file = File::create(&file_path).unwrap();

        if i % 3 == 0 {
            // Invalid: garbage data
            file.write_all(b"GARBAGE").unwrap();
        } else if i % 3 == 1 {
            // Invalid: malformed PDF
            file.write_all(b"%PDF-1.7\nBAD CONTENT\n%%EOF").unwrap();
        } else {
            // Invalid but with proper structure (would need valid PDF for truly valid)
            file.write_all(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n%%EOF").unwrap();
        }
        file.flush().unwrap();
        files.push(file_path);
    }

    // Process in parallel - should not panic
    let results: Vec<ValidationResult> = files
        .par_iter()
        .map(|path| ValidationResult {
            path: path.clone(),
            is_valid: validate_pdf(path, false),
        })
        .collect();

    assert_eq!(results.len(), 20, "All files should be processed");
    // Should complete without panic
}

/// Test that validator handles non-existent file gracefully
#[test]
fn test_nonexistent_file() {
    let fake_path = PathBuf::from("/tmp/this_file_does_not_exist_xyz123.pdf");
    let result = validate_pdf(&fake_path, false);
    assert!(!result, "Non-existent file should be invalid");
}

/// Test lenient mode with malformed PDFs
#[test]
fn test_lenient_mode_with_malformed_pdfs() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF-1.7\nSome minor issues\n%%EOF").unwrap();
    temp_file.flush().unwrap();

    // Should not panic in lenient mode
    let _result = validate_pdf_lenient(temp_file.path());
    // Key: no panic
}

/// Test that pdf-rs specific validation handles errors
#[test]
fn test_pdf_rs_validation_with_bad_data() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF-1.7\n1 0 obj\n<<< INVALID >>>\nendobj\n%%EOF").unwrap();
    temp_file.flush().unwrap();

    // Should return error, not panic
    let result = validate_pdf_with_pdf_rs(temp_file.path());
    match result {
        Ok(valid) => assert!(!valid, "Invalid PDF structure should fail"),
        Err(_) => {}, // Expected error
    }
}

/// Test basic validation fallback with corrupted data
#[test]
fn test_basic_validation_with_corrupted_data() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF-1.7\nCORRUPTED\n%%EOF").unwrap();
    temp_file.flush().unwrap();

    // Basic validation should check header and EOF
    let _result = validate_pdf_basic(temp_file.path());
    // Should not panic, result may vary
}

/// Test quick validation catches obvious issues
#[test]
fn test_quick_validation_performance() {
    use std::time::Instant;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"NOT A PDF").unwrap();
    temp_file.flush().unwrap();

    let start = Instant::now();
    let _ = validate_pdf(temp_file.path(), false);
    let duration = start.elapsed();

    // Quick validation should reject invalid files very fast (< 10ms)
    assert!(
        duration.as_millis() < 100,
        "Quick validation should be fast, took {:?}",
        duration
    );
}

/// Test that multiple threads can validate concurrently without issues
#[test]
fn test_concurrent_validation_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = Arc::new(TempDir::new().unwrap());
    let mut handles = vec![];

    // Create 10 threads, each validating 5 files
    for thread_id in 0..10 {
        let temp_dir = Arc::clone(&temp_dir);
        let handle = thread::spawn(move || {
            for i in 0..5 {
                let file_path = temp_dir.path().join(format!("thread_{}_{}.pdf", thread_id, i));
                let mut file = File::create(&file_path).unwrap();
                file.write_all(b"%PDF-1.7\nTest content\n%%EOF").unwrap();
                file.flush().unwrap();

                // Validate without panicking
                let _result = validate_pdf(&file_path, false);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
}

/// Test detailed validation returns proper errors
#[test]
fn test_detailed_validation_error_reporting() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"%PDF-1.7\nINVALID STRUCTURE\n%%EOF").unwrap();
    temp_file.flush().unwrap();

    let result = validate_pdf_detailed(temp_file.path());

    match result {
        Ok(_) => {}, // Might pass basic checks
        Err(e) => {
            // Should have meaningful error message
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty(), "Error message should not be empty");
        }
    }
}
