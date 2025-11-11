# Test Fixtures

This directory contains test fixture files for integration testing.

## Purpose

Test fixtures are pre-created files used by the integration test suite to verify:
- Error isolation with various malformed PDFs
- Circuit breaker behavior with toxic files
- Thread safety in parallel processing
- Handling of edge cases and corrupted data

## Fixture Generation

Most test fixtures are generated dynamically by the test code using `tempfile` crate.
This approach ensures:
- Tests are self-contained
- No need to commit binary test files to git
- Easy to modify test cases
- Cross-platform compatibility

## Static Fixtures (Optional)

If you need to add static test fixtures:

1. **valid_minimal.pdf** - Smallest valid PDF for baseline testing
2. **valid_multipage.pdf** - Valid PDF with multiple pages
3. **corrupted_header.pdf** - Invalid PDF header
4. **missing_eof.pdf** - PDF without EOF marker
5. **circular_ref.pdf** - PDF with circular object references
6. **deeply_nested.pdf** - PDF with deeply nested structures

These can be generated with:
```bash
# Minimal valid PDF (7 lines)
echo "%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R/Resources<<>>>>endobj
xref
0 4
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
trailer<</Size 4/Root 1 0 R>>
startxref
212
%%EOF" > valid_minimal.pdf
```

## Usage in Tests

Tests should prefer dynamically generated fixtures:

```rust
use tempfile::NamedTempFile;
use std::io::Write;

let mut temp_file = NamedTempFile::new().unwrap();
temp_file.write_all(b"%PDF-1.7\nContent\n%%EOF").unwrap();
temp_file.flush().unwrap();

let result = validate_pdf(temp_file.path(), false);
```

## Cleanup

Temporary files are automatically cleaned up by the `tempfile` crate when they go out of scope.
