# Research Questions: Memory-Safe PDF Validation in Rust

## Context
We're building a high-performance parallel PDF validator in Rust that processes hundreds of thousands of PDFs using the Rayon parallelism library. We're experiencing memory corruption issues (double-free errors, heap corruption) when using the `lopdf` crate in multi-threaded contexts.

## Current Problem Summary
- **Crate**: `lopdf` v0.34 (pure Rust PDF parser)
- **Issue**: Memory corruption when parsing malformed PDFs in parallel
  - "double free or corruption (out)" errors
  - "free(): invalid size" errors  
  - Panics in `hashbrown` (HashMap implementation): "assertion failed: buckets.is_power_of_two()"
- **Current mitigation**: Global mutex serializing all lopdf operations (kills performance)
- **Architecture**: Rayon parallel iterator processing 200k+ PDFs with 32 worker threads

## Research Questions

### 1. PDF Library Safety in Rust
**Question**: What are the most memory-safe and production-ready PDF parsing libraries available in Rust as of 2025? Please compare:
- `lopdf` - pure Rust, but experiencing memory corruption
- `pdf` crate - alternative pure Rust parser
- `pdfium-render` - wrapper around Google's PDFium (C++ library)
- `pdf-extract` - text extraction focused
- `mupdf` bindings - if available
- Any other maintained PDF libraries

For each library, provide:
- Memory safety guarantees (especially with malformed PDFs)
- Thread safety characteristics
- Performance characteristics for validation workloads
- Production readiness and maintenance status
- Known issues with parallel processing

### 2. Concurrency Control Patterns
**Question**: What are the best practices in Rust for limiting concurrency when calling potentially unsafe operations in a parallel workload?

Specifically compare these approaches:
- **Global Mutex** (current approach - too restrictive)
- **Semaphore** (limiting to N concurrent operations)
  - Which Rust semaphore implementation? (tokio::sync::Semaphore, parking_lot, custom Arc<Mutex<usize>>)
  - Optimal permit count for I/O-bound PDF parsing?
- **Process isolation** (spawn separate processes)
- **Thread pool partitioning** (dedicated thread pool for unsafe operations)
- **Work-stealing queues** with bounded concurrency

What are the performance trade-offs of each approach?

### 3. FFI and Memory Safety
**Question**: When wrapping C/C++ libraries (like PDFium) in Rust:
- What are best practices for ensuring memory safety across FFI boundaries?
- How do you prevent double-free errors when C++ code is called from multiple Rust threads?
- Should we use process isolation, or are there thread-safe FFI patterns?
- What are the pitfalls of using `pdfium-render` or similar C++ wrappers in highly parallel contexts?

### 4. Error Isolation in Parallel Workloads
**Question**: What are Rust best practices for isolating errors/panics when processing untrusted input (malformed PDFs) in parallel?

Consider:
- `std::panic::catch_unwind` effectiveness and limitations
- When does catch_unwind NOT prevent crashes (unsafe code, FFI, memory corruption)?
- Process isolation vs thread-level isolation
- Designing fault-tolerant parallel pipelines in Rayon
- Circuit breaker patterns for toxic inputs

### 5. Rayon-Specific Patterns
**Question**: What are the recommended patterns for using Rayon with potentially unsafe or panicking operations?

- How to limit concurrency in specific stages of a Rayon pipeline?
- Custom thread pool configuration for mixed safe/unsafe workloads
- Chunking strategies to reduce contention
- Error handling and recovery in Rayon iterators

### 6. Memory Corruption Diagnosis
**Question**: How to diagnose and debug memory corruption issues in Rust that involve:
- FFI calls or unsafe code in dependencies
- Multi-threaded contexts
- Intermittent failures (heap corruption, double-free)

Tools and techniques:
- MIRI (Rust interpreter for detecting undefined behavior)
- AddressSanitizer, ThreadSanitizer, MemorySanitizer
- Valgrind/Helgrind
- Debugging strategies for corruption in dependency crates
- How to determine if a crate has memory safety issues

### 7. Production Deployment Patterns
**Question**: For a production PDF validation service processing millions of files:
- What architecture would you recommend? (single process, worker pool, serverless, etc.)
- How to balance throughput vs stability when dealing with untrusted inputs?
- Monitoring and alerting strategies for memory issues
- Graceful degradation patterns (fallback libraries, skip toxic files)
- Resource limits (memory, file size, timeout) best practices

### 8. Semaphore Implementation Details
**Question**: If implementing concurrency limiting via semaphore in Rust:
- Should we use `tokio::sync::Semaphore` in non-async code? (using `blocking_acquire()`)
- Or use `parking_lot`'s synchronization primitives?
- Or implement custom semaphore with `Arc<Mutex<usize>>` + `Condvar`?
- What are the performance characteristics of each approach?
- What's the optimal permit count for I/O-bound PDF parsing? (e.g., 4, 8, 16 concurrent lopdf operations)

### 9. Alternative Validation Strategies
**Question**: Beyond parsing the entire PDF structure, what are lightweight PDF validation techniques?

- Header/footer magic bytes validation
- Structural validation without full parsing
- Cross-reference table validation
- Incremental validation (fail-fast approaches)
- Hybrid approaches (lightweight check first, full parse on demand)

### 10. Real-World Case Studies
**Question**: Are there open-source projects or case studies of high-performance PDF processing in Rust?
- How do they handle malformed inputs?
- What libraries do they use?
- What concurrency patterns do they employ?
- Any documented issues and solutions?

## Specific Technical Details to Address

### Current Code Architecture
```rust
// Current problematic approach:
lazy_static! {
    static ref LOPDF_MUTEX: Mutex<()> = Mutex::new(());
}

// In validation function:
let _guard = LOPDF_MUTEX.lock().unwrap();
match lopdf::Document::load(&path) {
    Ok(doc) => { /* validate */ }
    Err(e) => { /* handle */ }
}
// Guard held until end of scope - complete serialization
```

### Desired Properties
1. **Throughput**: Validate 200k+ PDFs efficiently (currently ~900 files in 81 seconds before crash)
2. **Stability**: Handle malformed PDFs without crashing
3. **Resource utilization**: High disk I/O throughput, efficient CPU usage
4. **Scalability**: Work with 16-32 worker threads

### Constraints
- Running on Linux with 32 CPU cores
- Processing from disk (not network I/O)
- PDFs range from 100 bytes to 500MB
- Unknown proportion of malformed files
- Must be production-ready (no crashes acceptable)

## Deliverables Requested

1. **Recommended approach** for solving the memory corruption issue
2. **Code examples** or patterns for implementing the solution
3. **Library recommendations** with justification
4. **Performance tuning** guidance (semaphore permits, thread pool sizing, etc.)
5. **Testing strategies** for validating the fix
6. **Monitoring recommendations** for production deployment

## Priority Questions
If limited time, please prioritize:
1. Library comparison (Q1) - is lopdf fundamentally broken for our use case?
2. Concurrency control patterns (Q2, Q8) - semaphore vs other approaches
3. FFI safety (Q3) - if we switch to pdfium-render
4. Real-world examples (Q10) - learn from others' solutions

---

**Target audience**: Experienced Rust systems programmers familiar with concurrent programming, FFI, and production deployment of high-performance services.
