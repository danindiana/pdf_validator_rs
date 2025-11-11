use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

// Import from our modularized library
use pdf_validator_rs::prelude::*;

#[derive(Parser)]
#[command(name = "pdf_validator_rs")]
#[command(about = "High-performance PDF validator written in Rust", long_about = None)]
struct Cli {
    /// Target directory to scan for PDF files
    directory: PathBuf,

    /// Scan directories recursively
    #[arg(short, long)]
    recursive: bool,

    /// Number of parallel worker threads (default: number of CPUs)
    #[arg(short, long)]
    workers: Option<usize>,

    /// Output report filename
    #[arg(short, long, default_value = "validation_report_rust.txt")]
    output: PathBuf,

    /// Delete invalid/corrupted PDF files
    #[arg(long)]
    delete_invalid: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Detect and report duplicate files
    #[arg(long)]
    detect_duplicates: bool,

    /// Delete duplicate PDF files (requires --detect-duplicates)
    #[arg(long)]
    delete_duplicates: bool,

    /// Run in batch mode (no interactive prompts, no progress bar)
    #[arg(long)]
    batch: bool,

    /// Skip rendering quality checks (faster validation)
    #[arg(long)]
    no_render_check: bool,

    /// Use lenient parsing mode (accept more PDFs with minor issues)
    #[arg(long)]
    lenient: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up rayon thread pool
    if let Some(workers) = cli.workers {
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build_global()
            .context("Failed to build thread pool")?;
    }

    let num_threads = rayon::current_num_threads();
    println!("PDF Validator (Rust Edition)");
    println!("Using {} worker thread(s)", num_threads);
    println!();

    // Collect PDF files
    let pdf_files = collect_pdf_files(&cli.directory, cli.recursive)?;
    let total_files = pdf_files.len();

    if total_files == 0 {
        println!("No PDF files found in the specified directory.");
        return Ok(());
    }

    println!("Found {} PDF file(s) to validate\n", total_files);

    // Set up progress bar (skip in batch mode)
    let progress = if cli.batch {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb
    };

    // Validate files in parallel
    let processed = AtomicUsize::new(0);
    let check_rendering = !cli.no_render_check;
    let use_lenient = cli.lenient;

    let results: Vec<ValidationResult> = pdf_files
        .par_iter()
        .map(|path| {
            // Choose validation method based on flags
            let is_valid = if use_lenient {
                // Lenient mode - accept more PDFs
                validate_pdf_lenient(path)
            } else if check_rendering {
                // Strict mode with rendering check
                let basic_valid = validate_pdf(path, cli.verbose);
                if basic_valid && cfg!(feature = "rendering") {
                    // Also check if pages can be rendered
                    // validate_pdf_rendering(path, 5) // Check first 5 pages
                    validate_pdf_lenient(path) // Fallback when rendering not available
                } else {
                    basic_valid
                }
            } else {
                // Normal strict mode
                validate_pdf(path, cli.verbose)
            };

            let count = processed.fetch_add(1, Ordering::Relaxed) + 1;

            if !cli.batch && (count % 100 == 0 || count == total_files) {
                progress.set_position(count as u64);
            }

            ValidationResult {
                path: path.clone(),
                is_valid,
            }
        })
        .collect();

    if !cli.batch {
        progress.finish_with_message("Validation complete!");
        println!();
    }

    // Detect duplicates if requested
    let duplicates = if cli.detect_duplicates || cli.delete_duplicates {
        println!("Detecting duplicate files...");
        let valid_paths: Vec<_> = results
            .iter()
            .filter(|r| r.is_valid)
            .map(|r| r.path.clone())
            .collect();

        match find_duplicates(&valid_paths) {
            Ok(dups) => {
                println!("Found {} groups of duplicate files\n", dups.len());

                // Delete duplicates if requested (keep first file in each group)
                if cli.delete_duplicates && !dups.is_empty() {
                    let mut total_deleted = 0;
                    for dup_group in &dups {
                        // Skip first file (keep it), delete the rest
                        for path in dup_group.paths.iter().skip(1) {
                            match fs::remove_file(path) {
                                Ok(_) => {
                                    total_deleted += 1;
                                    if cli.verbose {
                                        println!("Deleted duplicate: {}", path.display());
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error deleting duplicate {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                    println!("Deleted {} duplicate file(s)\n", total_deleted);
                }

                Some(dups)
            }
            Err(e) => {
                eprintln!("Error detecting duplicates: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Separate valid and invalid files
    let valid_count = results.iter().filter(|r| r.is_valid).count();
    let invalid_count = results.len() - valid_count;

    let invalid_files: Vec<_> = results
        .iter()
        .filter(|r| !r.is_valid)
        .map(|r| &r.path)
        .collect();

    // Print summary
    println!("==================================================");
    println!("VALIDATION COMPLETE");
    println!("==================================================");
    println!("Valid PDF files: {}", valid_count);
    println!("Invalid PDF files: {}", invalid_count);
    println!();

    // Delete invalid files if requested
    if cli.delete_invalid && !invalid_files.is_empty() {
        println!("Deleting {} invalid file(s)...", invalid_files.len());
        let mut deleted_count = 0;
        for path in &invalid_files {
            if let Err(e) = fs::remove_file(path) {
                eprintln!("Error deleting {:?}: {}", path, e);
            } else {
                deleted_count += 1;
            }
        }
        println!("Deleted {} invalid file(s)", deleted_count);
        println!();
    }

    // Write report
    write_report(
        &cli.output,
        &results,
        duplicates.as_deref(),
    )?;
    println!("Detailed report saved to: {:?}", cli.output);

    Ok(())
}
