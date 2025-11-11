# Integration Test Suite

Comprehensive integration tests for pdf_validator_rs focusing on error isolation, malformed PDF handling, and circuit breaker functionality.

## Test Organization

### 1. Error Isolation Tests (`error_isolation_test.rs`)

Tests that ensure errors in individual PDFs don't crash the entire validation process:

- **Garbage data handling** - Non-PDF files
- **Corrupted content** - PDF header with invalid content
- **Truncated files** - Incomplete PDF structures
- **Missing markers** - PDFs without EOF
- **Parallel processing** - Error isolation in concurrent validation
- **Thread safety** - Multi-threaded validation without races

**Key Goals:**
- No panics on invalid input
- Errors isolated per-file
- Parallel processing continues despite individual failures

### 2. Malformed PDF Tests (`malformed_pdf_test.rs`)

Tests various types of malformed PDFs to ensure robust error handling:

- **Corrupted headers** - Invalid PDF magic bytes
- **Corrupted EOF markers** - Missing or malformed end markers
- **Invalid versions** - Out-of-range version numbers
- **Undersized files** - Files below minimum PDF size
- **Corrupted objects** - Invalid PDF object structures
- **Invalid cross-references** - Broken xref tables
- **Binary corruption** - Random binary data
- **Circular references** - Self-referencing objects
- **Deep nesting** - Stack overflow prevention
- **Invalid streams** - Corrupted compression data

**Key Goals:**
- Handle all malformed input gracefully
- No infinite loops or hangs
- Quick rejection of obviously invalid files
- Comparison of strict vs lenient modes

### 3. Circuit Breaker Tests (`circuit_breaker_test.rs`)

Tests the circuit breaker pattern to prevent resource exhaustion from toxic PDFs:

- **State transitions** - CLOSED → OPEN → HALF_OPEN
- **Cooldown behavior** - Time-based recovery
- **Reset on success** - Counter reset after successful operation
- **Thread safety** - Concurrent access to circuit breaker
- **Blocking when open** - Prevents processing during open state
- **Recovery** - Successful validation in half-open closes circuit
- **Toxic file handling** - Real-world scenario with malformed PDFs
- **Resource exhaustion prevention** - Stops repeated failures
- **Threshold accuracy** - Precise failure counting
- **Edge cases** - Boundary conditions and rapid transitions

**Key Goals:**
- Prevent wasting resources on toxic files
- Thread-safe operation
- Proper state machine implementation
- Fast recovery after issues resolve

## Running Tests

### Run all integration tests:
```bash
cargo test --tests
```

### Run specific test suite:
```bash
# Error isolation tests only
cargo test --test error_isolation_test

# Malformed PDF tests only
cargo test --test malformed_pdf_test

# Circuit breaker tests only
cargo test --test circuit_breaker_test
```

### Run specific test:
```bash
cargo test --test circuit_breaker_test test_circuit_breaker_state_transitions
```

### Run with output:
```bash
cargo test --tests -- --nocapture
```

### Run in release mode (faster):
```bash
cargo test --tests --release
```

## Test Coverage

### Current Coverage:

| Category | Test Count | Coverage |
|----------|-----------|----------|
| Error Isolation | 17 | File-level error handling |
| Malformed PDFs | 25 | Various corruption types |
| Circuit Breaker | 21 | State machine & concurrency |
| **Total** | **63** | **Comprehensive** |

### Coverage Areas:

✅ **Covered:**
- Invalid file formats
- Corrupted PDF structures
- Concurrent validation
- Circuit breaker state machine
- Error propagation
- Thread safety
- Resource exhaustion prevention

🔄 **Partial Coverage:**
- Specific PDF features (forms, annotations, etc.)
- Large-scale stress testing (10k+ files)
- Memory profiling under load
- Rendering validation (optional feature)

❌ **Not Covered:**
- Network-based PDF fetching
- Encrypted PDFs
- PDF/A compliance validation
- Digital signature verification

## Test Requirements

### Dependencies:
- `tempfile` - For creating temporary test files
- `rayon` - For parallel processing tests

### System Requirements:
- At least 100MB free disk space (for temporary files)
- Multi-core CPU recommended (for parallel tests)
- Linux/macOS/Windows supported

## Performance Expectations

| Test Suite | Expected Duration | File Operations |
|------------|------------------|-----------------|
| Error Isolation | 1-3 seconds | ~50 temp files |
| Malformed PDFs | 2-5 seconds | ~100 temp files |
| Circuit Breaker | 3-6 seconds | ~50 temp files + delays |
| **Total** | **6-14 seconds** | **~200 temp files** |

## Continuous Integration

These tests are designed for CI/CD pipelines:

```yaml
# .github/workflows/test.yml example
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run integration tests
        run: cargo test --tests --verbose
```

## Debugging Test Failures

### Enable verbose output:
```bash
cargo test --tests -- --nocapture --test-threads=1
```

### Run specific failing test:
```bash
cargo test --test malformed_pdf_test test_corrupted_header -- --exact
```

### Check for memory leaks:
```bash
valgrind --leak-check=full cargo test --tests
```

### Profile test performance:
```bash
cargo test --tests --release -- --nocapture --show-output
```

## Adding New Tests

### Template for new error isolation test:
```rust
#[test]
fn test_new_error_case() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"your test content").unwrap();
    temp_file.flush().unwrap();

    let result = validate_pdf(temp_file.path(), false);
    assert!(!result, "Description of expected behavior");
}
```

### Template for new circuit breaker test:
```rust
#[test]
fn test_circuit_breaker_new_scenario() {
    let cb = CircuitBreaker::new(threshold, duration);

    // Your test logic here

    assert!(/* your assertion */, "Description");
}
```

## Known Issues

None currently. If you discover issues:
1. Check if the issue is reproducible
2. Create a minimal test case
3. Report to issue tracker with test details

## Best Practices

1. **Use `tempfile` for test fixtures** - Automatic cleanup
2. **Keep tests focused** - One aspect per test
3. **Use descriptive names** - Clear test intent
4. **Don't test external services** - Mock dependencies
5. **Make tests deterministic** - Avoid random data unless necessary
6. **Clean up resources** - Use RAII patterns
7. **Document edge cases** - Explain non-obvious tests

## Troubleshooting

### Tests fail with "too many open files":
```bash
# Increase file descriptor limit
ulimit -n 4096
```

### Tests hang:
- Check for infinite loops in malformed PDF handling
- Verify timeout mechanisms work correctly
- Run with `--test-threads=1` to isolate issues

### Flaky tests:
- Check for race conditions in parallel tests
- Verify timing assumptions in circuit breaker tests
- Use deterministic delays, not random ones

## Future Enhancements

Planned additions:
- [ ] Fuzz testing with `cargo-fuzz`
- [ ] Property-based testing with `proptest`
- [ ] Performance benchmarks with `criterion`
- [ ] Memory leak detection tests
- [ ] Large-scale stress tests (100k+ files)
- [ ] Rendering validation tests (when feature enabled)

## References

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Circuit Breaker Pattern](https://martinfowler.com/bliki/CircuitBreaker.html)
- [PDF Reference](https://www.adobe.com/content/dam/acom/en/devnet/pdf/pdfs/PDF32000_2008.pdf)
