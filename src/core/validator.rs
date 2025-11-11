//! PDF validation logic

use anyhow::Result;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[cfg(feature = "rendering")]
use pdfium_render::prelude::*;

use super::circuit_breaker::CircuitBreaker;
use std::time::Duration;

// Circuit breaker for tracking repeated failures
lazy_static::lazy_static! {
    static ref CIRCUIT_BREAKER: CircuitBreaker = CircuitBreaker::new(10, Duration::from_secs(60));
}

/// Quick pre-validation before attempting full parse
/// Checks PDF magic bytes, file size, and EOF marker
fn quick_validate(path: &Path) -> Result<()> {
    let mut file = File::open(path)?;
    
    // 1. Check PDF magic bytes (%PDF-)
    let mut header = [0u8; 8];
    file.read_exact(&mut header)?;
    if &header[0..5] != b"%PDF-" {
        anyhow::bail!("Invalid PDF header");
    }
    
    // 2. Check file size
    let metadata = file.metadata()?;
    let file_size = metadata.len();
    
    if file_size > 500_000_000 { // 500MB
        anyhow::bail!("File too large: {} bytes", file_size);
    }
    
    if file_size < 100 {
        anyhow::bail!("File too small: {} bytes", file_size);
    }
    
    // 3. Check for EOF marker (%%EOF) in last 1KB
    let tail_size = std::cmp::min(1024, file_size);
    file.seek(SeekFrom::End(-(tail_size as i64)))?;
    let mut tail = vec![0u8; tail_size as usize];
    file.read_exact(&mut tail)?;
    
    if !tail.windows(5).any(|w| w == b"%%EOF") {
        anyhow::bail!("Missing %%EOF marker");
    }
    
    Ok(())
}

/// Validate a PDF file
///
/// # Arguments
/// * `path` - Path to the PDF file
/// * `verbose` - Whether to print verbose error messages
///
/// # Returns
/// `true` if the PDF is valid, `false` otherwise
pub fn validate_pdf(path: &Path, verbose: bool) -> bool {
    // Quick pre-validation (no semaphore needed)
    if let Err(e) = quick_validate(path) {
        if verbose {
            eprintln!("Quick validation failed for {:?}: {}", path, e);
        }
        return false;
    }
    
    // Try using pdf-rs for robust validation (thread-safe, pure Rust)
    match validate_pdf_with_pdf_rs(path) {
        Ok(valid) => {
            if verbose && !valid {
                eprintln!("Invalid (pdf-rs): {:?}", path);
            }
            valid
        }
        Err(e) => {
            if verbose {
                eprintln!("Error validating {:?}: {}", path, e);
            }
            // Fallback to basic validation
            validate_pdf_basic(path)
        }
    }
}

/// Validate PDF using pdf-rs library (pure Rust, thread-safe)
pub fn validate_pdf_with_pdf_rs(path: &Path) -> Result<bool> {
    // Check circuit breaker first
    if CIRCUIT_BREAKER.is_open() {
        anyhow::bail!("Circuit breaker is OPEN - too many recent failures");
    }
    
    // pdf-rs is thread-safe, no semaphore needed
    match pdf::file::FileOptions::cached().open(path) {
        Ok(pdf_file) => {
            CIRCUIT_BREAKER.record_success();
            
            // Check if document has pages
            let num_pages = pdf_file.num_pages();
            if num_pages == 0 {
                Ok(false)
            } else {
                // Verify we can actually access at least one page
                match pdf_file.get_page(0) {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
        }
        Err(e) => {
            CIRCUIT_BREAKER.record_failure();
            anyhow::bail!("pdf-rs parse error: {}", e)
        }
    }
}

/// Basic PDF validation (fallback when pdf-rs fails)
pub fn validate_pdf_basic(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut buffer = Vec::new();
    if file.read_to_end(&mut buffer).is_err() {
        return false;
    }

    // Check for PDF header
    if buffer.len() < 5 || &buffer[0..5] != b"%PDF-" {
        return false;
    }

    // Check for EOF marker
    buffer.windows(5).any(|window| window == b"%%EOF")
}

/// Validate PDF with detailed error information
pub fn validate_pdf_detailed(path: &Path) -> Result<bool> {
    // Quick pre-validation
    quick_validate(path)?;
    
    // Check circuit breaker
    if CIRCUIT_BREAKER.is_open() {
        anyhow::bail!("Circuit breaker is OPEN - too many recent failures");
    }
    
    // pdf-rs is thread-safe, no semaphore needed
    match pdf::file::FileOptions::cached().open(path) {
        Ok(pdf_file) => {
            CIRCUIT_BREAKER.record_success();
            
            let num_pages = pdf_file.num_pages();
            if num_pages == 0 {
                anyhow::bail!("PDF has no pages")
            }
            Ok(true)
        }
        Err(e) => {
            CIRCUIT_BREAKER.record_failure();
            Err(e.into())
        }
    }
}

/// Lenient PDF validation that tries multiple strategies
pub fn validate_pdf_lenient(path: &Path) -> bool {
    // Try quick validation first
    if quick_validate(path).is_err() {
        // If quick validation fails, still try basic validation
        return validate_pdf_basic(path);
    }
    
    // Try pdf-rs (thread-safe, pure Rust)
    if let Ok(true) = validate_pdf_with_pdf_rs(path) {
        return true;
    }
    
    // Fallback to basic
    validate_pdf_basic(path)
}

#[cfg(feature = "rendering")]
pub fn validate_pdf_rendering(path: &Path) -> Result<bool> {
    use pdfium_render::prelude::*;

    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())?,
    );

    let document = pdfium.load_pdf_from_file(path, None)?;

    for page_index in 0..document.pages().len() {
        let page = document.pages().get(page_index)?;
        let _render_config = page.render()?;
        // Could add more sophisticated checks here
    }

    Ok(true)
}
