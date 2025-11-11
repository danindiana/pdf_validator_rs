# Module Structure and Dependencies

```mermaid
%%{init: {'theme':'base', 'themeVariables': { 'primaryColor':'#fff0','primaryTextColor':'#333','primaryBorderColor':'#333','lineColor':'#333','secondaryColor':'#fff0','tertiaryColor':'#fff0','background':'#fff0','mainBkg':'#fff0','secondBkg':'#fff0'}}}%%
graph LR
    subgraph Binary["Binary Crate"]
        Main[main.rs<br/>CLI Entry Point]
    end

    subgraph Library["Library Crate: pdf_validator_rs"]
        Lib[lib.rs<br/>Public API]

        subgraph CoreMod["core::"]
            CoreMod_Main[mod.rs]
            Validator[validator.rs<br/>PDF Validation Logic]
        end

        subgraph ScannerMod["scanner::"]
            ScannerMod_Main[mod.rs]
            FileScanner[file_scanner.rs<br/>Directory Traversal]
            DupDetector[duplicate_detector.rs<br/>SHA-256 Hashing]
        end

        subgraph ReportingMod["reporting::"]
            ReportingMod_Main[mod.rs]
            ReportWriter[report_writer.rs<br/>Report Generation]
        end

        Prelude[prelude<br/>Convenience Re-exports]
    end

    subgraph Examples["Examples"]
        Diagnose[diagnose_discrepancies.rs<br/>Diagnostic Tool]
    end

    subgraph External["External Dependencies"]
        Clap[clap<br/>CLI Parsing]
        Rayon[rayon<br/>Parallelism]
        Lopdf[lopdf<br/>PDF Parsing]
        WalkDir[walkdir<br/>Directory Walking]
        Sha2[sha2<br/>Hashing]
        Indicatif[indicatif<br/>Progress Bars]
        Anyhow[anyhow<br/>Error Handling]
        Pdfium[pdfium-render<br/>Optional: Rendering]
    end

    %% Main connections
    Main --> Prelude
    Main --> Clap
    Main --> Rayon
    Main --> Indicatif
    Main --> Anyhow

    %% Library structure
    Lib --> CoreMod_Main
    Lib --> ScannerMod_Main
    Lib --> ReportingMod_Main
    Lib --> Prelude

    CoreMod_Main --> Validator
    ScannerMod_Main --> FileScanner
    ScannerMod_Main --> DupDetector
    ReportingMod_Main --> ReportWriter

    Prelude --> Validator
    Prelude --> FileScanner
    Prelude --> DupDetector
    Prelude --> ReportWriter

    %% Module dependencies on external crates
    Validator --> Lopdf
    Validator --> Pdfium
    Validator --> Anyhow
    FileScanner --> WalkDir
    FileScanner --> Anyhow
    DupDetector --> Sha2
    DupDetector --> Anyhow
    ReportWriter --> Anyhow

    %% Cross-module dependencies
    ReportWriter --> FileScanner
    ReportWriter --> DupDetector
    Main --> FileScanner
    Main --> Validator
    Main --> DupDetector
    Main --> ReportWriter

    %% Examples
    Diagnose --> Validator
    Diagnose --> Lopdf

    style Main fill:#ffebee
    style Library fill:#e8f5e9
    style External fill:#e3f2fd
    style Examples fill:#fff3e0
```

## Module Responsibilities

### **Binary: main.rs**
- CLI argument parsing and validation
- Thread pool configuration
- Progress bar management
- Orchestrates validation workflow
- Handles user output and reporting

### **core::validator**
**Exports:**
- `validate_pdf()` - Standard validation
- `validate_pdf_with_lopdf()` - Lopdf-based validation
- `validate_pdf_basic()` - Fallback validation
- `validate_pdf_detailed()` - Validation with error details
- `validate_pdf_lenient()` - Permissive validation
- `validate_pdf_rendering()` - Optional rendering validation

**Dependencies:**
- `lopdf` - Primary PDF parsing
- `pdfium-render` - Optional rendering validation
- `anyhow` - Error handling

### **scanner::file_scanner**
**Exports:**
- `collect_pdf_files()` - Directory scanning
- `ValidationResult` - Result struct

**Dependencies:**
- `walkdir` - Recursive directory traversal
- `anyhow` - Error handling

### **scanner::duplicate_detector**
**Exports:**
- `compute_file_hash()` - SHA-256 file hashing
- `find_duplicates()` - Duplicate detection
- `DuplicateInfo` - Duplicate group struct

**Dependencies:**
- `sha2` - SHA-256 hashing
- `anyhow` - Error handling

### **reporting::report_writer**
**Exports:**
- `write_report()` - Comprehensive report generation
- `write_simple_report()` - Legacy format report

**Dependencies:**
- `scanner::file_scanner::ValidationResult`
- `scanner::duplicate_detector::DuplicateInfo`
- `anyhow` - Error handling

### **prelude**
Convenience module that re-exports commonly used types and functions for easier imports.
