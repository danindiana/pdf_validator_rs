//! File scanning and collection functionality

pub mod file_scanner;
pub mod duplicate_detector;

pub use file_scanner::{collect_pdf_files, ValidationResult};
pub use duplicate_detector::{compute_file_hash, find_duplicates, DuplicateInfo};
