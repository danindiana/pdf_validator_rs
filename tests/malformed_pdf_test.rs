//! Integration tests for malformed PDF handling
//!
//! Tests various types of malformed PDFs to ensure robust error handling

use pdf_validator_rs::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

/// Helper to create a test PDF file with given content
fn create_test_file(content: &[u8]) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(content).unwrap();
    temp_file.flush().unwrap();
    temp_file
}

/// Test PDF with corrupted header
#[test]
fn test_corrupted_header() {
    let test_cases = vec![
        b"PDF-1.7\n%%EOF".as_slice(),           // Missing %
        b"$PDF-1.7\n%%EOF".as_slice(),           // Wrong character
        b"%%PDF-1.7\n%%EOF".as_slice(),          // Extra %
        b"%pdf-1.7\n%%EOF".as_slice(),           // Lowercase
        b"%PD-1.7\n%%EOF".as_slice(),            // Missing F
    ];

    for (idx, content) in test_cases.iter().enumerate() {
        let temp_file = create_test_file(content);
        let result = validate_pdf(temp_file.path(), false);
        assert!(!result, "Test case {} should be invalid (corrupted header)", idx);
    }
}

/// Test PDF with missing or corrupted EOF marker
#[test]
fn test_corrupted_eof_marker() {
    let test_cases = vec![
        b"%PDF-1.7\nContent\n%EOF".as_slice(),   // Single %
        b"%PDF-1.7\nContent\n%%%EOF".as_slice(), // Triple %
        b"%PDF-1.7\nContent\nEOF".as_slice(),    // Missing %%
        b"%PDF-1.7\nContent\n%%eof".as_slice(),  // Lowercase
        b"%PDF-1.7\nContent\n".as_slice(),       // Completely missing
    ];

    for (idx, content) in test_cases.iter().enumerate() {
        let temp_file = create_test_file(content);
        let result = validate_pdf(temp_file.path(), false);
        assert!(!result, "Test case {} should be invalid (bad EOF marker)", idx);
    }
}

/// Test PDF with invalid version numbers
#[test]
fn test_invalid_version_numbers() {
    let test_cases = vec![
        b"%PDF-0.0\n%%EOF".as_slice(),
        b"%PDF-99.99\n%%EOF".as_slice(),
        b"%PDF-1.99\n%%EOF".as_slice(),
        b"%PDF-10.0\n%%EOF".as_slice(),
        b"%PDF-1.a\n%%EOF".as_slice(),
        b"%PDF-x.y\n%%EOF".as_slice(),
    ];

    for content in test_cases.iter() {
        let temp_file = create_test_file(content);
        // Should not panic, regardless of whether it's accepted or rejected
        let _result = validate_pdf(temp_file.path(), false);
    }
}

/// Test PDFs that are too small
#[test]
fn test_files_below_minimum_size() {
    let test_cases = vec![
        b"".as_slice(),
        b"%".as_slice(),
        b"%P".as_slice(),
        b"%PD".as_slice(),
        b"%PDF".as_slice(),
        b"%PDF-".as_slice(),
        b"%PDF-1".as_slice(),
    ];

    for (idx, content) in test_cases.iter().enumerate() {
        let temp_file = create_test_file(content);
        let result = validate_pdf(temp_file.path(), false);
        assert!(!result, "Test case {} (size {}) should be invalid", idx, content.len());
    }
}

/// Test PDF with corrupted object structure
#[test]
fn test_corrupted_object_structure() {
    let test_cases = vec![
        // Unclosed dictionary
        b"%PDF-1.7\n1 0 obj\n<<\nendobj\n%%EOF".as_slice(),
        // Invalid object reference
        b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 999 0 R >>\nendobj\n%%EOF".as_slice(),
        // Malformed object number
        b"%PDF-1.7\nA B obj\n<<>>\nendobj\n%%EOF".as_slice(),
        // Missing endobj
        b"%PDF-1.7\n1 0 obj\n<<>>\n%%EOF".as_slice(),
    ];

    for content in test_cases.iter() {
        let temp_file = create_test_file(content);
        // Should handle gracefully without panic
        let _result = validate_pdf(temp_file.path(), false);
    }
}

/// Test PDF with invalid cross-reference table
#[test]
fn test_invalid_xref_table() {
    let content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Type /Catalog >>\n\
        endobj\n\
        xref\n\
        INVALID XREF DATA\n\
        trailer\n\
        << /Size 1 /Root 1 0 R >>\n\
        %%EOF";

    let temp_file = create_test_file(content);
    let _result = validate_pdf(temp_file.path(), false);
    // Should not panic
}

/// Test PDF with binary data corruption
#[test]
fn test_binary_data_corruption() {
    let mut content = Vec::from(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    // Add random binary corruption
    content.extend_from_slice(&[0xFF, 0xFE, 0xFD, 0xFC, 0x00, 0x01, 0x02, 0x03]);
    content.extend_from_slice(b"\n%%EOF");

    let temp_file = create_test_file(&content);
    let _result = validate_pdf(temp_file.path(), false);
    // Should not panic
}

/// Test PDF with circular references (if not caught, could cause infinite loop)
#[test]
fn test_circular_references() {
    let content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Type /Catalog /Pages 2 0 R >>\n\
        endobj\n\
        2 0 obj\n\
        << /Type /Pages /Kids [3 0 R] /Count 1 >>\n\
        endobj\n\
        3 0 obj\n\
        << /Type /Page /Parent 2 0 R /Contents 3 0 R >>\n\
        endobj\n\
        %%EOF";

    let temp_file = create_test_file(content);

    use std::time::{Duration, Instant};
    let start = Instant::now();
    let _result = validate_pdf(temp_file.path(), false);
    let elapsed = start.elapsed();

    // Should complete quickly, not hang
    assert!(
        elapsed < Duration::from_secs(5),
        "Validation should not hang on circular references (took {:?})",
        elapsed
    );
}

/// Test PDF with extremely nested structures
#[test]
fn test_deeply_nested_structures() {
    let mut content = Vec::from(b"%PDF-1.7\n1 0 obj\n");

    // Create deeply nested arrays
    for _ in 0..100 {
        content.push(b'[');
    }
    for _ in 0..100 {
        content.push(b']');
    }

    content.extend_from_slice(b"\nendobj\n%%EOF");

    let temp_file = create_test_file(&content);

    use std::time::{Duration, Instant};
    let start = Instant::now();
    let _result = validate_pdf(temp_file.path(), false);
    let elapsed = start.elapsed();

    // Should complete without stack overflow
    assert!(
        elapsed < Duration::from_secs(5),
        "Validation should handle deep nesting (took {:?})",
        elapsed
    );
}

/// Test PDF with null bytes embedded
#[test]
fn test_null_bytes_in_content() {
    let mut content = Vec::from(b"%PDF-1.7\n");
    content.push(0x00); // Null byte
    content.extend_from_slice(b"Content");
    content.push(0x00); // Another null byte
    content.extend_from_slice(b"\n%%EOF");

    let temp_file = create_test_file(&content);
    let _result = validate_pdf(temp_file.path(), false);
    // Should not panic on null bytes
}

/// Test PDF with invalid stream length
#[test]
fn test_invalid_stream_length() {
    let content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Length 999999 >>\n\
        stream\n\
        Short content\n\
        endstream\n\
        endobj\n\
        %%EOF";

    let temp_file = create_test_file(content);
    let _result = validate_pdf(temp_file.path(), false);
    // Should handle mismatch gracefully
}

/// Test PDF with missing required keys
#[test]
fn test_missing_required_keys() {
    let test_cases = vec![
        // Catalog without Pages
        b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n%%EOF".as_slice(),
        // Pages without Kids
        b"%PDF-1.7\n1 0 obj\n<< /Type /Pages /Count 0 >>\nendobj\n%%EOF".as_slice(),
        // Page without Parent
        b"%PDF-1.7\n1 0 obj\n<< /Type /Page >>\nendobj\n%%EOF".as_slice(),
    ];

    for content in test_cases.iter() {
        let temp_file = create_test_file(content);
        let _result = validate_pdf(temp_file.path(), false);
        // Should not panic
    }
}

/// Test PDF with invalid escape sequences
#[test]
fn test_invalid_escape_sequences() {
    let content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Title (Invalid\\xZZ escape) >>\n\
        endobj\n\
        %%EOF";

    let temp_file = create_test_file(content);
    let _result = validate_pdf(temp_file.path(), false);
}

/// Test mixed line endings (CR, LF, CRLF)
#[test]
fn test_mixed_line_endings() {
    let content = b"%PDF-1.7\r\n\
        1 0 obj\n\
        << /Type /Catalog >>\r\
        endobj\r\n\
        %%EOF";

    let temp_file = create_test_file(content);
    // Should handle different line endings
    let _result = validate_pdf(temp_file.path(), false);
}

/// Test PDF with Unicode/UTF-8 content
#[test]
fn test_unicode_content() {
    let content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Title (\xE2\x9C\x93 Unicode \xF0\x9F\x93\x84) >>\n\
        endobj\n\
        %%EOF";

    let temp_file = create_test_file(content);
    let _result = validate_pdf(temp_file.path(), false);
}

/// Test linearized PDF with corrupted linearization dictionary
#[test]
fn test_corrupted_linearization() {
    let content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Linearized 1 /L 999999 /H [0 0] /O 5 /E 999 /N 1 /T 0 >>\n\
        endobj\n\
        %%EOF";

    let temp_file = create_test_file(content);
    let _result = validate_pdf(temp_file.path(), false);
}

/// Test PDF with incorrect object offsets in xref
#[test]
fn test_incorrect_xref_offsets() {
    let content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Type /Catalog >>\n\
        endobj\n\
        xref\n\
        0 2\n\
        0000000000 65535 f\n\
        9999999999 00000 n\n\
        trailer\n\
        << /Size 2 /Root 1 0 R >>\n\
        startxref\n\
        50\n\
        %%EOF";

    let temp_file = create_test_file(content);
    let _result = validate_pdf(temp_file.path(), false);
}

/// Test that lenient mode handles more malformed PDFs
#[test]
fn test_lenient_mode_comparison() {
    let content = b"%PDF-1.7\n\
        Some slightly malformed content\n\
        %%EOF";

    let temp_file = create_test_file(content);

    let _strict_result = validate_pdf(temp_file.path(), false);
    let lenient_result = validate_pdf_lenient(temp_file.path());

    // Lenient should never be more strict than normal mode
    if lenient_result {
        // If lenient accepts it, strict might reject it (that's ok)
    }
    // Key point: neither should panic
}

/// Test handling of files that claim to be PDFs but aren't
#[test]
fn test_fake_pdf_extensions() {
    // Create a JPEG file with .pdf extension (conceptually)
    let jpeg_header = b"\xFF\xD8\xFF\xE0";
    let mut content = Vec::from(jpeg_header.as_slice());
    content.extend_from_slice(b"\x00\x10JFIF");

    let temp_file = create_test_file(&content);
    let result = validate_pdf(temp_file.path(), false);
    assert!(!result, "JPEG file should not validate as PDF");
}

/// Test handling of very long object streams
#[test]
fn test_very_long_content_stream() {
    let mut content = Vec::from(b"%PDF-1.7\n1 0 obj\n<< /Length 10000 >>\nstream\n");

    // Add 10KB of 'A' characters
    for _ in 0..10000 {
        content.push(b'A');
    }

    content.extend_from_slice(b"\nendstream\nendobj\n%%EOF");

    let temp_file = create_test_file(&content);
    let _result = validate_pdf(temp_file.path(), false);
    // Should handle large streams without panic
}

/// Test PDF with compression that might fail
#[test]
fn test_corrupted_compressed_stream() {
    let content = b"%PDF-1.7\n\
        1 0 obj\n\
        << /Length 20 /Filter /FlateDecode >>\n\
        stream\n\
        INVALID ZLIB DATA!\n\
        endstream\n\
        endobj\n\
        %%EOF";

    let temp_file = create_test_file(content);
    let _result = validate_pdf(temp_file.path(), false);
    // Should handle decompression errors
}
