//! Report writing functionality

use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::scanner::file_scanner::ValidationResult;
use crate::scanner::duplicate_detector::DuplicateInfo;

/// Write validation results to a file
///
/// # Arguments
/// * `output_path` - Path to output file
/// * `results` - Validation results to write
/// * `duplicates` - Optional duplicate file information
///
/// # Returns
/// Result indicating success or failure
pub fn write_report(
    output_path: &Path,
    results: &[ValidationResult],
    duplicates: Option<&[DuplicateInfo]>,
) -> Result<()> {
    let mut file = File::create(output_path)?;

    // Write header with timestamp
    let now = std::time::SystemTime::now();
    writeln!(file, "PDF Validation Report")?;
    writeln!(file, "====================")?;
    writeln!(file, "Generated: {:?}", now)?;
    writeln!(file)?;

    // Write summary statistics
    let valid_count = results.iter().filter(|r| r.is_valid).count();
    let invalid_count = results.len() - valid_count;

    writeln!(file, "Summary Statistics:")?;
    writeln!(file, "-------------------")?;
    writeln!(file, "  Total files scanned: {}", results.len())?;
    writeln!(file, "  Valid PDF files: {}", valid_count)?;
    writeln!(file, "  Invalid PDF files: {}", invalid_count)?;

    if results.len() > 0 {
        let valid_pct = (valid_count as f64 / results.len() as f64) * 100.0;
        writeln!(file, "  Validation success rate: {:.2}%", valid_pct)?;
    }

    writeln!(file)?;

    // Write invalid files
    if invalid_count > 0 {
        writeln!(file, "Invalid Files:")?;
        writeln!(file, "--------------")?;
        for result in results.iter().filter(|r| !r.is_valid) {
            writeln!(file, "  {}", result.path.display())?;
        }
        writeln!(file)?;
    }

    // Write duplicate files if provided
    if let Some(dups) = duplicates {
        if !dups.is_empty() {
            let total_dups: usize = dups.iter().map(|d| d.paths.len() - 1).sum();
            let total_dup_size = dups.len();

            writeln!(file, "Duplicate Files:")?;
            writeln!(file, "----------------")?;
            writeln!(file, "  Total duplicate groups: {}", total_dup_size)?;
            writeln!(file, "  Total redundant files: {}", total_dups)?;
            writeln!(file)?;

            for (idx, dup) in dups.iter().enumerate() {
                writeln!(file, "  Group {} (Hash: {}...):", idx + 1, &dup.hash[..16])?;
                writeln!(file, "    Files ({} duplicates):", dup.paths.len())?;
                for (file_idx, path) in dup.paths.iter().enumerate() {
                    let marker = if file_idx == 0 { "[KEEP]" } else { "[DUP] " };
                    writeln!(file, "      {} {}", marker, path.display())?;
                }
                writeln!(file)?;
            }
        }
    }

    // Write valid files list
    writeln!(file, "Valid Files:")?;
    writeln!(file, "------------")?;
    writeln!(file, "  Total: {}", valid_count)?;
    writeln!(file)?;
    for result in results.iter().filter(|r| r.is_valid) {
        writeln!(file, "  {}", result.path.display())?;
    }

    Ok(())
}

/// Write simple validation results (legacy format)
///
/// # Arguments
/// * `output_path` - Path to output file
/// * `results` - Validation results to write
pub fn write_simple_report(output_path: &Path, results: &[ValidationResult]) -> Result<()> {
    let mut file = File::create(output_path)?;

    for result in results {
        let status = if result.is_valid { "VALID" } else { "INVALID" };
        writeln!(file, "{}: {}", status, result.path.display())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_report() {
        let temp_file = NamedTempFile::new().unwrap();

        let results = vec![
            ValidationResult {
                path: PathBuf::from("/test/valid.pdf"),
                is_valid: true,
            },
            ValidationResult {
                path: PathBuf::from("/test/invalid.pdf"),
                is_valid: false,
            },
        ];

        write_report(temp_file.path(), &results, None).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("Total files: 2"));
        assert!(content.contains("Valid PDFs: 1"));
        assert!(content.contains("Invalid PDFs: 1"));
    }

    #[test]
    fn test_write_simple_report() {
        let temp_file = NamedTempFile::new().unwrap();

        let results = vec![
            ValidationResult {
                path: PathBuf::from("/test/valid.pdf"),
                is_valid: true,
            },
        ];

        write_simple_report(temp_file.path(), &results).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("VALID: /test/valid.pdf"));
    }
}
