//! PDF file scanning and collection

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Result of validating a single PDF file
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub path: PathBuf,
    pub is_valid: bool,
}

/// Collect all PDF files from a directory
///
/// # Arguments
/// * `dir` - Directory to scan
/// * `recursive` - Whether to scan subdirectories recursively
///
/// # Returns
/// Vector of PDF file paths
pub fn collect_pdf_files(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut pdf_files = Vec::new();

    if recursive {
        for entry in WalkDir::new(dir).follow_links(false) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext.to_string_lossy().to_lowercase() == "pdf" {
                        pdf_files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    } else {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(ext) = entry.path().extension() {
                    if ext.to_string_lossy().to_lowercase() == "pdf" {
                        pdf_files.push(entry.path());
                    }
                }
            }
        }
    }

    Ok(pdf_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_collect_pdf_files_non_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");
        File::create(&pdf_path).unwrap();
        
        let files = collect_pdf_files(temp_dir.path(), false).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], pdf_path);
    }

    #[test]
    fn test_collect_pdf_files_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        
        let pdf1 = temp_dir.path().join("test1.pdf");
        let pdf2 = subdir.join("test2.pdf");
        File::create(&pdf1).unwrap();
        File::create(&pdf2).unwrap();
        
        let files = collect_pdf_files(temp_dir.path(), true).unwrap();
        assert_eq!(files.len(), 2);
    }
}
