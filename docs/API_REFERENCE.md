# API Reference - PDF Validator v1.0.0

## Table of Contents
- [Library Overview](#library-overview)
- [Module Structure](#module-structure)
- [Core Module](#core-module)
- [Scanner Module](#scanner-module)
- [Reporting Module](#reporting-module)
- [Usage Examples](#usage-examples)

## Library Overview

The PDF Validator library provides a comprehensive API for validating PDF files, detecting duplicates, and generating reports. The library is organized into three main modules:

- `core::validator` - PDF validation functions
- `scanner::file_scanner` - File collection and scanning
- `scanner::duplicate_detector` - Duplicate detection using SHA-256
- `reporting::report_writer` - Report generation

## Module Structure

```rust
use pdf_validator_rs::prelude::*;
```

The prelude module re-exports commonly used types and functions for convenience.

## Core Module

### `core::validator`

PDF validation functions with multiple strategies.

#### Functions

##### `validate_pdf(path: &Path, verbose: bool) -> bool`

Standard PDF validation with fallback mechanisms.

**Parameters:**
- `path: &Path` - Path to the PDF file
- `verbose: bool` - Whether to print verbose error messages

**Returns:**
- `bool` - `true` if the PDF is valid, `false` otherwise

**Behavior:**
1. Attempts validation using lopdf library
2. Falls back to basic validation if lopdf fails
3. Prints errors if verbose mode is enabled

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::prelude::*;

let path = Path::new("document.pdf");
let is_valid = validate_pdf(path, true);

if is_valid {
    println!("PDF is valid");
} else {
    println!("PDF is invalid");
}
```

---

##### `validate_pdf_with_lopdf(path: &Path) -> Result<bool>`

Validate PDF using the lopdf library.

**Parameters:**
- `path: &Path` - Path to the PDF file

**Returns:**
- `Result<bool>` - `Ok(true)` if valid, `Ok(false)` if invalid, `Err` on parsing errors

**Validation Steps:**
1. Attempts to load PDF with lopdf
2. Checks if document has at least one page
3. Returns error on parse failures

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::core::validator::validate_pdf_with_lopdf;

let path = Path::new("document.pdf");
match validate_pdf_with_lopdf(path) {
    Ok(true) => println!("Valid PDF with pages"),
    Ok(false) => println!("Valid PDF but no pages"),
    Err(e) => println!("Parse error: {}", e),
}
```

---

##### `validate_pdf_basic(path: &Path) -> bool`

Basic PDF validation without external libraries.

**Parameters:**
- `path: &Path` - Path to the PDF file

**Returns:**
- `bool` - `true` if basic validation passes

**Validation Checks:**
1. File size >= 1000 bytes
2. Starts with `%PDF` header
3. Contains `%%EOF` marker in last 1KB
4. Contains `xref` table

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::core::validator::validate_pdf_basic;

let path = Path::new("document.pdf");
if validate_pdf_basic(path) {
    println!("Passed basic validation");
}
```

---

##### `validate_pdf_detailed(path: &Path) -> (bool, Option<String>)`

Validate PDF with detailed error information.

**Parameters:**
- `path: &Path` - Path to the PDF file

**Returns:**
- `(bool, Option<String>)` - Tuple of (is_valid, error_message)

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::core::validator::validate_pdf_detailed;

let path = Path::new("document.pdf");
let (is_valid, error_msg) = validate_pdf_detailed(path);

if is_valid {
    println!("Valid PDF");
} else if let Some(err) = error_msg {
    println!("Invalid: {}", err);
}
```

---

##### `validate_pdf_lenient(path: &Path, verbose: bool) -> bool`

Lenient PDF validation with multiple fallback methods.

**Parameters:**
- `path: &Path` - Path to the PDF file
- `verbose: bool` - Whether to print verbose messages

**Returns:**
- `bool` - `true` if any validation method succeeds

**Validation Strategy:**
1. Try lopdf validation (strict)
2. Try basic validation (moderate)
3. Try super-lenient validation (permissive)

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::prelude::*;

let path = Path::new("edge-case.pdf");
let is_valid = validate_pdf_lenient(path, true);
```

---

##### `validate_pdf_rendering(path: &Path, max_pages: usize) -> bool`

Validate PDF by attempting to render pages (requires `rendering` feature).

**Parameters:**
- `path: &Path` - Path to the PDF file
- `max_pages: usize` - Maximum number of pages to check (0 = all pages)

**Returns:**
- `bool` - `true` if rendering succeeds

**Note:** Returns `true` (no-op) when compiled without `rendering` feature.

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::prelude::*;

let path = Path::new("document.pdf");
// Check first 5 pages
let can_render = validate_pdf_rendering(path, 5);
```

---

## Scanner Module

### `scanner::file_scanner`

File collection and scanning functionality.

#### Types

##### `ValidationResult`

Result of validating a single PDF file.

```rust
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub path: PathBuf,
    pub is_valid: bool,
}
```

**Fields:**
- `path: PathBuf` - Path to the PDF file
- `is_valid: bool` - Validation result

---

#### Functions

##### `collect_pdf_files(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>>`

Collect all PDF files from a directory.

**Parameters:**
- `dir: &Path` - Directory to scan
- `recursive: bool` - Whether to scan subdirectories recursively

**Returns:**
- `Result<Vec<PathBuf>>` - Vector of PDF file paths

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::prelude::*;

let dir = Path::new("/path/to/pdfs");
let pdf_files = collect_pdf_files(dir, true).unwrap();

println!("Found {} PDF files", pdf_files.len());
for path in pdf_files {
    println!("  {}", path.display());
}
```

---

### `scanner::duplicate_detector`

Duplicate file detection using SHA-256 hashing.

#### Types

##### `DuplicateInfo`

Information about a group of duplicate files.

```rust
#[derive(Debug, Clone)]
pub struct DuplicateInfo {
    pub hash: String,
    pub paths: Vec<PathBuf>,
}
```

**Fields:**
- `hash: String` - SHA-256 hash (hex-encoded)
- `paths: Vec<PathBuf>` - List of files with identical hash

---

#### Functions

##### `compute_file_hash(path: &Path) -> Result<String>`

Compute SHA-256 hash of a file.

**Parameters:**
- `path: &Path` - Path to the file

**Returns:**
- `Result<String>` - Hex-encoded SHA-256 hash

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::prelude::*;

let path = Path::new("document.pdf");
let hash = compute_file_hash(path).unwrap();
println!("SHA-256: {}", hash);
```

---

##### `find_duplicates(paths: &[PathBuf]) -> Result<Vec<DuplicateInfo>>`

Find duplicate files in a list of paths.

**Parameters:**
- `paths: &[PathBuf]` - List of file paths to check

**Returns:**
- `Result<Vec<DuplicateInfo>>` - Vector of duplicate groups

**Example:**
```rust
use std::path::PathBuf;
use pdf_validator_rs::prelude::*;

let paths = vec![
    PathBuf::from("file1.pdf"),
    PathBuf::from("file2.pdf"),
    PathBuf::from("file3.pdf"),
];

let duplicates = find_duplicates(&paths).unwrap();

for dup in duplicates {
    println!("Hash: {}", dup.hash);
    println!("Files ({} duplicates):", dup.paths.len());
    for path in dup.paths {
        println!("  {}", path.display());
    }
}
```

---

## Reporting Module

### `reporting::report_writer`

Report generation functionality.

#### Functions

##### `write_report(output_path: &Path, results: &[ValidationResult], duplicates: Option<&[DuplicateInfo]>) -> Result<()>`

Write comprehensive validation report to file.

**Parameters:**
- `output_path: &Path` - Path to output file
- `results: &[ValidationResult]` - Validation results
- `duplicates: Option<&[DuplicateInfo]>` - Optional duplicate information

**Returns:**
- `Result<()>` - Success or error

**Report Sections:**
1. Summary statistics
2. Invalid files list
3. Duplicate file groups (if provided)
4. Valid files list

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::prelude::*;

let results = vec![
    ValidationResult {
        path: PathBuf::from("valid.pdf"),
        is_valid: true,
    },
    ValidationResult {
        path: PathBuf::from("invalid.pdf"),
        is_valid: false,
    },
];

let output = Path::new("report.txt");
write_report(output, &results, None).unwrap();
```

---

##### `write_simple_report(output_path: &Path, results: &[ValidationResult]) -> Result<()>`

Write simple validation report (legacy format).

**Parameters:**
- `output_path: &Path` - Path to output file
- `results: &[ValidationResult]` - Validation results

**Returns:**
- `Result<()>` - Success or error

**Format:**
```
VALID: /path/to/file1.pdf
INVALID: /path/to/file2.pdf
```

**Example:**
```rust
use std::path::Path;
use pdf_validator_rs::prelude::*;

let results = vec![/* ... */];
let output = Path::new("simple_report.txt");
write_simple_report(output, &results).unwrap();
```

---

## Usage Examples

### Complete Validation Workflow

```rust
use std::path::Path;
use pdf_validator_rs::prelude::*;

fn main() -> anyhow::Result<()> {
    // 1. Collect PDF files
    let dir = Path::new("/path/to/pdfs");
    let pdf_files = collect_pdf_files(dir, true)?;

    println!("Found {} PDF files", pdf_files.len());

    // 2. Validate each file
    let results: Vec<ValidationResult> = pdf_files
        .iter()
        .map(|path| ValidationResult {
            path: path.clone(),
            is_valid: validate_pdf(path, false),
        })
        .collect();

    // 3. Detect duplicates in valid files
    let valid_paths: Vec<_> = results
        .iter()
        .filter(|r| r.is_valid)
        .map(|r| r.path.clone())
        .collect();

    let duplicates = find_duplicates(&valid_paths)?;

    // 4. Write report
    let report_path = Path::new("validation_report.txt");
    write_report(report_path, &results, Some(&duplicates))?;

    // 5. Print summary
    let valid_count = results.iter().filter(|r| r.is_valid).count();
    let invalid_count = results.len() - valid_count;

    println!("Valid: {}, Invalid: {}", valid_count, invalid_count);
    println!("Duplicates: {} groups", duplicates.len());

    Ok(())
}
```

### Parallel Validation with Rayon

```rust
use rayon::prelude::*;
use std::path::Path;
use pdf_validator_rs::prelude::*;

fn parallel_validate(dir: &Path) -> anyhow::Result<Vec<ValidationResult>> {
    let pdf_files = collect_pdf_files(dir, true)?;

    let results: Vec<ValidationResult> = pdf_files
        .par_iter()  // Parallel iterator
        .map(|path| ValidationResult {
            path: path.clone(),
            is_valid: validate_pdf(path, false),
        })
        .collect();

    Ok(results)
}
```

### Custom Validation Logic

```rust
use std::path::Path;
use pdf_validator_rs::core::validator::*;

fn custom_validate(path: &Path) -> (bool, String) {
    // Try strict validation first
    match validate_pdf_with_lopdf(path) {
        Ok(true) => (true, "Strict validation passed".to_string()),
        Ok(false) => (false, "No pages found".to_string()),
        Err(_) => {
            // Try lenient validation
            if validate_pdf_lenient(path, false) {
                (true, "Lenient validation passed".to_string())
            } else {
                (false, "All validation methods failed".to_string())
            }
        }
    }
}
```

---

## Error Handling

All functions returning `Result` use `anyhow::Result` for flexible error handling.

**Common Patterns:**

```rust
use anyhow::{Context, Result};

// Pattern 1: Propagate errors
fn process_pdfs() -> Result<()> {
    let files = collect_pdf_files(dir, true)?;
    write_report(output, &results, None)?;
    Ok(())
}

// Pattern 2: Add context
fn process_with_context() -> Result<()> {
    let files = collect_pdf_files(dir, true)
        .context("Failed to scan directory")?;
    Ok(())
}

// Pattern 3: Handle specific errors
fn process_with_handling() -> Result<()> {
    match collect_pdf_files(dir, true) {
        Ok(files) => println!("Found {} files", files.len()),
        Err(e) => eprintln!("Error: {}", e),
    }
    Ok(())
}
```

---

## Thread Safety

All validation functions are thread-safe and can be used with parallel iterators (Rayon).

**Guaranteed Thread-Safe:**
- `validate_pdf()`
- `validate_pdf_with_lopdf()`
- `validate_pdf_basic()`
- `validate_pdf_lenient()`
- `compute_file_hash()`

**Not Thread-Safe (External I/O):**
- `write_report()` - Use mutex if writing from multiple threads
- `collect_pdf_files()` - Safe for parallel execution but returns sequentially

---

## Performance Considerations

1. **Validation Functions**: O(n) where n is file size
2. **File Collection**: O(m) where m is number of files
3. **Duplicate Detection**: O(m × n) for hashing, O(m) for grouping
4. **Report Writing**: O(m) for results iteration

**Optimization Tips:**
- Use `validate_pdf_basic()` for quick checks
- Enable parallel processing with Rayon
- Use `--no-render-check` flag for faster validation
- Consider lenient mode only for edge cases

---

**Last Updated**: v1.0.0 (November 10, 2025)
