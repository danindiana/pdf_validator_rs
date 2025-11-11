//! PDF Validator Library
//! 
//! A high-performance PDF validation library with support for parallel processing.

pub mod core;
pub mod scanner;
pub mod reporting;

pub use core::validator;
pub use scanner::file_scanner;
pub use reporting::report_writer;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::core::validator::{
        validate_pdf, validate_pdf_with_lopdf, validate_pdf_basic,
        validate_pdf_detailed, validate_pdf_lenient, // validate_pdf_rendering
    };
    pub use crate::scanner::file_scanner::{collect_pdf_files, ValidationResult};
    pub use crate::scanner::duplicate_detector::{compute_file_hash, find_duplicates, DuplicateInfo};
    pub use crate::reporting::report_writer::{write_report, write_simple_report};
}
