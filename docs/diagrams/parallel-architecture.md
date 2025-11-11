# Parallel Processing Architecture

```mermaid
%%{init: {'theme':'base', 'themeVariables': { 'primaryColor':'#fff0','primaryTextColor':'#333','primaryBorderColor':'#333','lineColor':'#333','secondaryColor':'#fff0','tertiaryColor':'#fff0','background':'#fff0','mainBkg':'#fff0','secondBkg':'#fff0'}}}%%
graph TB
    subgraph Main["Main Thread"]
        CLI[CLI Argument Parser<br/>clap]
        Setup[Thread Pool Setup<br/>rayon::ThreadPoolBuilder]
        Scanner[File Scanner<br/>collect_pdf_files]
        Progress[Progress Bar<br/>indicatif]
        Reporter[Report Writer]
    end

    subgraph Collection["File Collection Phase"]
        WalkDir[WalkDir Iterator]
        Filter[Filter .pdf Extensions]
        PathList[Vec&lt;PathBuf&gt;]
    end

    subgraph RayonPool["Rayon Thread Pool"]
        Splitter[Work Splitter<br/>par_iter]

        subgraph Worker1["Worker Thread 1"]
            V1[Validate PDF 1]
            V2[Validate PDF 2]
            V3[Validate PDF 3]
        end

        subgraph Worker2["Worker Thread 2"]
            V4[Validate PDF 4]
            V5[Validate PDF 5]
            V6[Validate PDF 6]
        end

        subgraph Worker3["Worker Thread N"]
            V7[Validate PDF N-2]
            V8[Validate PDF N-1]
            V9[Validate PDF N]
        end

        Collector[Result Collector<br/>Vec&lt;ValidationResult&gt;]
    end

    subgraph PostProcess["Post-Processing Phase"]
        DupDetect[Duplicate Detection<br/>SHA-256 Hashing]
        DupHash[Parallel Hashing<br/>rayon par_iter]
        DupGroup[Group by Hash<br/>HashMap]
        DeleteOps[Optional Delete Operations]
    end

    CLI --> Setup
    Setup --> Scanner
    Scanner --> WalkDir
    WalkDir --> Filter
    Filter --> PathList

    PathList --> Splitter
    Splitter --> Worker1
    Splitter --> Worker2
    Splitter --> Worker3

    Worker1 --> Collector
    Worker2 --> Collector
    Worker3 --> Collector

    V1 & V2 & V3 --> Progress
    V4 & V5 & V6 --> Progress
    V7 & V8 & V9 --> Progress

    Collector --> DupDetect
    DupDetect --> DupHash
    DupHash --> DupGroup
    DupGroup --> DeleteOps
    DeleteOps --> Reporter

    Reporter --> Output[validation_report.txt]

    style Main fill:#e3f2fd
    style RayonPool fill:#fff3e0
    style Worker1 fill:#c8e6c9
    style Worker2 fill:#c8e6c9
    style Worker3 fill:#c8e6c9
    style PostProcess fill:#f3e5f5
```

## Processing Flow Details

### 1. Initialization
- Main thread parses CLI arguments
- Configures Rayon thread pool (default: CPU count)
- Scans directory for PDF files

### 2. Parallel Validation
- File list split across N worker threads
- Each worker validates PDFs independently
- Lock-free atomic counter for progress tracking
- Results collected into single vector

### 3. Post-Processing
- Duplicate detection runs in parallel
- Each file hashed independently
- HashMap groups files by hash
- Optional deletion operations

### Performance Characteristics
- **Scalability**: Linear scaling up to I/O bottleneck
- **Memory**: Streaming validation, minimal per-file overhead
- **Synchronization**: Lock-free counters, no mutex contention
- **Load Balancing**: Rayon work-stealing for optimal distribution
