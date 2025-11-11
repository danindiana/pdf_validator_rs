//! PDF validation logic

use anyhow::Result;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[cfg(feature = "rendering")]
use pdfium_render::prelude::*;

/// Validate a PDF file
///
/// # Arguments
/// * `path` - Path to the PDF file
/// * `verbose` - Whether to print verbose error messages
///
/// # Returns
/// `true` if the PDF is valid, `false` otherwise
pub fn validate_pdf(path: &Path, verbose: bool) -> bool {
    // Try using lopdf first for robust validation
    match validate_pdf_with_lopdf(path) {
        Ok(valid) => {
            if verbose && !valid {
                eprintln!("Invalid (lopdf): {:?}", path);
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

/// Validate PDF using lopdf library
pub fn validate_pdf_with_lopdf(path: &Path) -> Result<bool> {
    match lopdf::Document::load(path) {
        Ok(doc) => {
            // Check if document has pages
            if doc.get_pages().is_empty() {
                return Ok(false);
            }
            Ok(true)
        }
        Err(e) => {
            // Return the error for detailed logging
            anyhow::bail!("lopdf parse error: {}", e);
        }
    }
}

/// Validate PDF with detailed error information
///
/// # Arguments
/// * `path` - Path to the PDF file
///
/// # Returns
/// Tuple of (is_valid, error_message)
pub fn validate_pdf_detailed(path: &Path) -> (bool, Option<String>) {
    match lopdf::Document::load(path) {
        Ok(doc) => {
            if doc.get_pages().is_empty() {
                (false, Some("No pages found in document".to_string()))
            } else {
                (true, None)
            }
        }
        Err(e) => {
            (false, Some(format!("lopdf error: {}", e)))
        }
    }
}

/// Lenient PDF validation - tries multiple methods
///
/// # Arguments
/// * `path` - Path to the PDF file
/// * `verbose` - Whether to print verbose error messages
///
/// # Returns
/// `true` if the PDF is valid by any method, `false` otherwise
pub fn validate_pdf_lenient(path: &Path, verbose: bool) -> bool {
    // Try lopdf first (strict)
    match validate_pdf_with_lopdf(path) {
        Ok(true) => return true,
        Ok(false) | Err(_) => {
            // Fall through to other methods
        }
    }

    // Try basic validation (more lenient)
    if validate_pdf_basic(path) {
        if verbose {
            println!("Valid (basic check): {:?}", path);
        }
        return true;
    }

    // Try super-basic check - just see if it looks like a PDF
    if validate_pdf_super_lenient(path) {
        if verbose {
            println!("Valid (lenient check): {:?}", path);
        }
        return true;
    }

    false
}

/// Super lenient PDF validation - just checks for PDF markers
///
/// This is more permissive than basic validation:
/// - Allows smaller files
/// - Doesn't require xref table
/// - Just checks for PDF header and some EOF marker
fn validate_pdf_super_lenient(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut content = Vec::new();
    if file.read_to_end(&mut content).is_err() {
        return false;
    }

    // More lenient size check (200 bytes instead of 1000)
    if content.len() < 200 {
        return false;
    }

    // Check PDF header (anywhere in first 1KB)
    let header_region = if content.len() > 1024 {
        &content[..1024]
    } else {
        &content[..]
    };

    if !header_region.windows(4).any(|w| w == b"%PDF") {
        return false;
    }

    // Check for any EOF-like marker (more lenient)
    if !content.windows(4).any(|w| w == b"%%EO" || w == b"%EOF" || w == b"EOF\n") {
        return false;
    }

    true
}

/// Validate PDF with rendering (requires 'rendering' feature)
///
/// # Arguments
/// * `path` - Path to the PDF file
/// * `max_pages` - Maximum number of pages to check (0 = all)
///
/// # Returns
/// `true` if the PDF can be rendered, `false` otherwise
#[cfg(feature = "rendering")]
pub fn validate_pdf_rendering(path: &Path, max_pages: usize) -> bool {
    use std::fs;

    // Try to load with pdfium
    let pdfium = match Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")).ok()?
    ) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let document = match pdfium.load_pdf_from_file(path, None) {
        Ok(doc) => doc,
        Err(_) => return false,
    };

    let page_count = document.pages().len();
    if page_count == 0 {
        return false;
    }

    let check_count = if max_pages == 0 || max_pages > page_count {
        page_count
    } else {
        max_pages
    };

    // Try to render the first few pages
    for i in 0..check_count {
        if let Ok(page) = document.pages().get(i) {
            // Try to render to bitmap
            match page.render_with_config(&PdfRenderConfig::default()) {
                Ok(_) => continue,
                Err(_) => return false,
            }
        } else {
            return false;
        }
    }

    true
}

/// Stub for rendering validation when feature is not enabled
#[cfg(not(feature = "rendering"))]
pub fn validate_pdf_rendering(_path: &Path, _max_pages: usize) -> bool {
    // Rendering not supported without feature flag
    // Return true to not fail validation
    true
}

/// Basic PDF validation without external libraries
///
/// Checks:
/// - PDF header (%PDF)
/// - EOF marker (%%EOF)
/// - xref table presence
/// - Minimum file size
pub fn validate_pdf_basic(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut content = Vec::new();
    if file.read_to_end(&mut content).is_err() {
        return false;
    }

    // Check minimum size
    if content.len() < 1000 {
        return false;
    }

    // Check PDF header
    if !content.starts_with(b"%PDF") {
        return false;
    }

    // Check for EOF marker (in last 1KB)
    let tail_start = if content.len() > 1024 {
        content.len() - 1024
    } else {
        0
    };
    
    if !content[tail_start..].windows(5).any(|w| w == b"%%EOF") {
        return false;
    }

    // Check for xref table
    if !content.windows(4).any(|w| w == b"xref") {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_pdf_basic_valid() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // Create a minimal PDF that meets the size requirement (>1000 bytes)
        let mut pdf_content = Vec::new();
        pdf_content.extend_from_slice(b"%PDF-1.4\n");
        pdf_content.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        pdf_content.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        pdf_content.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n");
        // Pad to meet minimum size
        pdf_content.extend_from_slice(&vec![b' '; 800]);
        pdf_content.extend_from_slice(b"\nxref\n0 4\n0000000000 65535 f\n");
        pdf_content.extend_from_slice(b"0000000009 00000 n\n0000000058 00000 n\n0000000115 00000 n\n");
        pdf_content.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n900\n%%EOF");

        temp_file.write_all(&pdf_content).unwrap();
        assert!(validate_pdf_basic(temp_file.path()));
    }

    #[test]
    fn test_validate_pdf_basic_invalid_header() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let invalid_pdf = b"NOTAPDF\nxref\ntrailer\n%%EOF";
        temp_file.write_all(invalid_pdf).unwrap();
        
        assert!(!validate_pdf_basic(temp_file.path()));
    }
}
