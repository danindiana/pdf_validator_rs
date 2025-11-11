use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

// Import from our modularized library
use pdf_validator_rs::prelude::*;

/// Checkpoint data for resuming validation
#[derive(Serialize, Deserialize)]
struct Checkpoint {
    completed_paths: Vec<PathBuf>,
    timestamp: SystemTime,
    total_files: usize,
}

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

    /// Resume from a previous checkpoint file
    #[arg(long)]
    resume_from: Option<PathBuf>,

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

    // Set up graceful shutdown handler
    let shutdown_requested = Arc::new(AtomicBool::new(false));
    let shutdown_flag = shutdown_requested.clone();
    
    ctrlc::set_handler(move || {
        eprintln!("\n⚠️  Shutdown requested. Finishing current files...");
        shutdown_flag.store(true, Ordering::SeqCst);
    })
    .context("Error setting Ctrl-C handler")?;

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

    // Load checkpoint if resuming
    let mut completed_files: HashSet<PathBuf> = HashSet::new();
    if let Some(ref checkpoint_path) = cli.resume_from {
        if checkpoint_path.exists() {
            match load_checkpoint(checkpoint_path) {
                Ok(checkpoint) => {
                    completed_files = checkpoint.completed_paths.into_iter().collect();
                    println!("📂 Resuming from checkpoint: {}", checkpoint_path.display());
                    println!("   Already validated {} files", completed_files.len());
                    println!();
                }
                Err(e) => {
                    eprintln!("⚠️  Warning: Failed to load checkpoint: {}", e);
                    eprintln!("   Starting fresh validation...\n");
                }
            }
        } else {
            eprintln!("⚠️  Warning: Checkpoint file not found: {}", checkpoint_path.display());
            eprintln!("   Starting fresh validation...\n");
        }
    }

    // Collect PDF files
    let all_pdf_files = collect_pdf_files(&cli.directory, cli.recursive)?;
    
    // Filter out already-completed files
    let pdf_files: Vec<PathBuf> = all_pdf_files
        .into_iter()
        .filter(|path| !completed_files.contains(path))
        .collect();
    
    let total_files = pdf_files.len();
    let already_completed = completed_files.len();

    if total_files == 0 && already_completed > 0 {
        println!("✅ All {} PDF files already validated!", already_completed);
        return Ok(());
    } else if total_files == 0 {
        println!("No PDF files found in the specified directory.");
        return Ok(());
    }

    if already_completed > 0 {
        println!("Found {} new PDF file(s) to validate ({} already completed)\n", 
            total_files, already_completed);
    } else {
        println!("Found {} PDF file(s) to validate\n", total_files);
    }

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
    let check_rendering = !cli.no_render_check;
    let use_lenient = cli.lenient;
    let shutdown_check = shutdown_requested.clone();
    
    // Partial results file for incremental saving
    let partial_output = PathBuf::from(format!("{}.partial", cli.output.display()));
    let checkpoint_output = PathBuf::from(format!("{}.checkpoint", cli.output.display()));
    
    // Thread-safe accumulator for completed paths
    let completed_paths: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    let completed_clone = completed_paths.clone();

    let results: Vec<ValidationResult> = pdf_files
        .par_iter()
        .progress_with(progress.clone())
        .filter_map(|path| {
            // Check if shutdown was requested
            if shutdown_check.load(Ordering::SeqCst) {
                return None; // Stop processing new files
            }
            
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
            
            // Track completed path for checkpoint
            if let Ok(mut paths) = completed_clone.lock() {
                paths.push(path.clone());
            }

            Some(ValidationResult {
                path: path.clone(),
                is_valid,
            })
        })
        .collect();

    // Display progress summary
    let processed_count = results.len();
    let was_interrupted = shutdown_requested.load(Ordering::SeqCst);
    
    // Save checkpoint if interrupted
    if was_interrupted {
        if let Ok(paths) = completed_paths.lock() {
            // Add previously completed files from checkpoint
            let mut all_completed: Vec<PathBuf> = completed_files.into_iter().collect();
            all_completed.extend(paths.clone());
            
            if let Err(e) = save_checkpoint(&checkpoint_output, all_completed, total_files + already_completed) {
                eprintln!("⚠️  Warning: Failed to save checkpoint: {}", e);
            } else {
                eprintln!("💾 Checkpoint saved to: {}", checkpoint_output.display());
                eprintln!("💡 Resume later with: --resume-from {}", checkpoint_output.display());
            }
        }
    }
    
    if !cli.batch {
        if was_interrupted {
            progress.finish_and_clear();
            eprintln!("\n⏹️  Graceful shutdown complete");
            eprintln!("📊 Processed {}/{} files ({:.1}%)", 
                processed_count, 
                total_files,
                (processed_count as f64 / total_files as f64) * 100.0
            );
        } else {
            progress.finish_with_message("Validation complete!");
        }
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
    let output_file = if was_interrupted {
        // Write to partial file if interrupted
        &partial_output
    } else {
        // Use final output if completed
        &cli.output
    };
    
    write_report(
        output_file,
        &results,
        duplicates.as_deref(),
    )?;
    
    if was_interrupted {
        println!("Partial results saved to: {:?}", partial_output);
    } else {
        println!("Detailed report saved to: {:?}", cli.output);
        // Clean up checkpoint if we completed successfully
        let _ = fs::remove_file(&checkpoint_output);
    }

    Ok(())
}

/// Load checkpoint from file
fn load_checkpoint(path: &PathBuf) -> Result<Checkpoint> {
    let file = File::open(path)
        .context("Failed to open checkpoint file")?;
    let checkpoint: Checkpoint = serde_json::from_reader(file)
        .context("Failed to parse checkpoint file")?;
    Ok(checkpoint)
}

/// Save checkpoint to file
fn save_checkpoint(path: &PathBuf, completed_paths: Vec<PathBuf>, total_files: usize) -> Result<()> {
    let checkpoint = Checkpoint {
        completed_paths,
        timestamp: SystemTime::now(),
        total_files,
    };
    let file = File::create(path)
        .context("Failed to create checkpoint file")?;
    serde_json::to_writer_pretty(file, &checkpoint)
        .context("Failed to write checkpoint file")?;
    Ok(())
}
