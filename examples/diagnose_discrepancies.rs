//! Diagnostic tool to analyze PDF validation discrepancies
//!
//! This tool examines PDFs that fail Rust validation but pass Python validation
//! to understand the specific errors lopdf reports.

use pdf_validator_rs::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file_list.txt>", args[0]);
        eprintln!("       {} <single_pdf_file.pdf>", args[0]);
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);

    if input_path.extension().and_then(|s| s.to_str()) == Some("txt") {
        // Process file list
        process_file_list(&input_path);
    } else {
        // Process single file
        process_single_file(&input_path);
    }
}

fn process_file_list(list_path: &PathBuf) {
    let file = File::open(list_path).expect("Failed to open file list");
    let reader = BufReader::new(file);

    println!("PDF Validation Diagnostic Report");
    println!("================================\n");

    let mut total = 0;
    let mut error_categories: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        let path = PathBuf::from(line.trim());

        if !path.exists() {
            eprintln!("File not found: {:?}", path);
            continue;
        }

        total += 1;
        let (is_valid, error_msg) = validate_pdf_detailed(&path);

        if !is_valid {
            if let Some(msg) = error_msg {
                // Extract error category
                let category = extract_error_category(&msg);
                error_categories.entry(category.clone())
                    .or_insert_with(Vec::new)
                    .push(path.to_string_lossy().to_string());

                println!("FILE: {}", path.display());
                println!("  STATUS: INVALID");
                println!("  ERROR: {}", msg);
                println!();
            }
        }
    }

    println!("\n=== SUMMARY ===");
    println!("Total files analyzed: {}", total);
    println!("\nError categories:");

    let mut categories: Vec<_> = error_categories.iter().collect();
    categories.sort_by_key(|(_, files)| std::cmp::Reverse(files.len()));

    for (category, files) in categories {
        println!("  {} ({} files)", category, files.len());
    }
}

fn process_single_file(path: &PathBuf) {
    if !path.exists() {
        eprintln!("File not found: {:?}", path);
        std::process::exit(1);
    }

    println!("Analyzing: {}", path.display());
    println!("==================================================\n");

    // Test with lopdf detailed
    let (is_valid, error_msg) = validate_pdf_detailed(path);
    println!("Lopdf validation: {}", if is_valid { "VALID" } else { "INVALID" });
    if let Some(msg) = error_msg {
        println!("  Error: {}", msg);
    }
    println!();

    // Test with basic validation
    let basic_valid = validate_pdf_basic(path);
    println!("Basic validation: {}", if basic_valid { "VALID" } else { "INVALID" });
    println!();

    // File metadata
    if let Ok(metadata) = std::fs::metadata(path) {
        println!("File size: {} bytes", metadata.len());
    }
}

fn extract_error_category(error_msg: &str) -> String {
    // Extract the main error category from lopdf error messages
    if error_msg.contains("Xref") {
        "Xref/Cross-reference table error".to_string()
    } else if error_msg.contains("EOF") || error_msg.contains("end of file") {
        "EOF/End of file error".to_string()
    } else if error_msg.contains("Invalid") || error_msg.contains("invalid") {
        "Invalid structure/syntax".to_string()
    } else if error_msg.contains("encrypt") || error_msg.contains("Encrypt") {
        "Encryption error".to_string()
    } else if error_msg.contains("object") {
        "Object reference error".to_string()
    } else if error_msg.contains("stream") {
        "Stream error".to_string()
    } else {
        error_msg.split(':').next().unwrap_or("Unknown").to_string()
    }
}
