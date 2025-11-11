# Build Guide - PDF Validator v1.0.0

## Table of Contents
- [Prerequisites](#prerequisites)
- [Build from Source](#build-from-source)
- [Build Profiles](#build-profiles)
- [Optimization Options](#optimization-options)
- [Feature Flags](#feature-flags)
- [Cross-Compilation](#cross-compilation)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software

#### Rust Toolchain
```bash
# Install Rust via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version  # Should be 1.70 or higher
cargo --version
```

#### System Dependencies

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config
```

**macOS:**
```bash
# Install Xcode Command Line Tools
xcode-select --install
```

**Windows:**
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
- Or use [mingw-w64](https://www.mingw-w64.org/)

### Minimum Requirements

- **Rust**: 1.70 or higher
- **RAM**: 2GB minimum (4GB recommended for compilation)
- **Disk Space**: 500MB for build artifacts
- **OS**: Linux, macOS, Windows 10+

## Build from Source

### Step 1: Clone Repository

```bash
git clone https://github.com/danindiana/pdf_validator_rs.git
cd pdf_validator_rs
```

### Step 2: Build Debug Version

```bash
# Fast compilation, includes debug symbols
cargo build

# Binary location
./target/debug/pdf_validator_rs --help
```

### Step 3: Build Release Version

```bash
# Optimized build (recommended for production)
cargo build --release

# Binary location
./target/release/pdf_validator_rs --help
```

### Step 4: Run Tests

```bash
# Run all tests
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_validate_pdf_basic
```

## Build Profiles

### Debug Profile (Default)

**Characteristics:**
- Fast compilation
- Large binary size (~10-15 MB)
- No optimizations
- Includes debug symbols
- Good for development

**Build Command:**
```bash
cargo build
```

### Release Profile

**Characteristics:**
- Slow compilation (2-5 minutes)
- Small binary size (~3-5 MB after strip)
- Maximum optimizations (opt-level = 3)
- LTO enabled
- Single codegen unit
- Debug symbols stripped

**Build Command:**
```bash
cargo build --release
```

**Configuration in Cargo.toml:**
```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = true           # Link-Time Optimization
codegen-units = 1    # Better optimization, slower compile
strip = true         # Strip debug symbols
```

## Optimization Options

### Standard Release Build
```bash
cargo build --release
```

### Release with Debug Info
```bash
# Useful for profiling
RUSTFLAGS="-C debuginfo=2" cargo build --release
```

### Maximum Performance Build
```bash
# Use native CPU features
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Minimum Binary Size
```bash
# Already configured in Cargo.toml, but can be enhanced
cargo build --release
strip target/release/pdf_validator_rs  # Further strip if needed
```

## Feature Flags

### Default Features

```bash
# Build with default features (no rendering)
cargo build --release
```

### Rendering Feature (Future)

```bash
# Build with PDF rendering validation support
cargo build --release --features rendering
```

**Note:** Rendering feature requires additional dependencies (pdfium).

### No Default Features

```bash
# Minimal build
cargo build --release --no-default-features
```

## Cross-Compilation

### Linux to Windows

```bash
# Add target
rustup target add x86_64-pc-windows-gnu

# Install cross-compiler
sudo apt install mingw-w64

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

### macOS to Linux

```bash
# Add target
rustup target add x86_64-unknown-linux-musl

# Install cross-compilation tools
brew install filosottile/musl-cross/musl-cross

# Build
cargo build --release --target x86_64-unknown-linux-musl
```

### Using Cross Tool

```bash
# Install cross
cargo install cross

# Build for different platforms
cross build --release --target x86_64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target x86_64-pc-windows-gnu
```

## Build Time Optimization

### Parallel Compilation

```bash
# Use all CPU cores (default)
cargo build --release -j $(nproc)

# Limit to 4 cores
cargo build --release -j 4
```

### Incremental Compilation

Already enabled by default in debug builds. For release:

```bash
# Enable incremental compilation in release mode
CARGO_INCREMENTAL=1 cargo build --release
```

### Using sccache

```bash
# Install sccache
cargo install sccache

# Configure
export RUSTC_WRAPPER=sccache

# Build
cargo build --release
```

## Build Verification

### Check Binary

```bash
# Verify binary exists and is executable
ls -lh target/release/pdf_validator_rs

# Check dependencies (Linux)
ldd target/release/pdf_validator_rs

# Check binary size
du -h target/release/pdf_validator_rs

# Verify it runs
target/release/pdf_validator_rs --help
```

### Run Integration Tests

```bash
# Create test directory
mkdir -p /tmp/pdf_test
# (Add some test PDFs)

# Run validator
target/release/pdf_validator_rs /tmp/pdf_test --recursive
```

## Troubleshooting

### Common Issues

#### 1. Linker Errors

**Problem:** `error: linking with 'cc' failed`

**Solution:**
```bash
# Ubuntu/Debian
sudo apt install build-essential

# macOS
xcode-select --install
```

#### 2. Out of Memory

**Problem:** Compilation runs out of memory

**Solution:**
```bash
# Reduce parallel jobs
cargo build --release -j 2

# Or increase swap space
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

#### 3. Outdated Rust Version

**Problem:** `error: package requires rustc 1.70`

**Solution:**
```bash
rustup update stable
```

#### 4. Dependency Resolution Failures

**Problem:** Cargo fails to resolve dependencies

**Solution:**
```bash
# Update Cargo.lock
cargo update

# Or clean and rebuild
cargo clean
cargo build --release
```

### Build Logs

Save build logs for debugging:

```bash
# Capture build output
cargo build --release 2>&1 | tee build.log

# Verbose output
cargo build --release --verbose 2>&1 | tee build-verbose.log
```

## Installation After Build

### System-Wide Installation

```bash
# Copy to system path
sudo cp target/release/pdf_validator_rs /usr/local/bin/

# Verify
which pdf_validator_rs
pdf_validator_rs --version
```

### User Installation

```bash
# Copy to user bin
mkdir -p ~/.local/bin
cp target/release/pdf_validator_rs ~/.local/bin/

# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"
```

### Using Cargo Install

```bash
# Install from local source
cargo install --path .

# Install from git
cargo install --git https://github.com/danindiana/pdf_validator_rs
```

## Build Information Reference

**Current Build Environment:**
- **Rust Version**: 1.90.0
- **Cargo Version**: 1.90.0
- **OS**: Ubuntu 22.04.5 LTS
- **Kernel**: 6.8.0-87-generic
- **Build Date**: Mon Nov 10 23:22:26 CST 2025
- **Unix Timestamp**: 1762838546

## Additional Resources

- [Rust Book - Building and Running](https://doc.rust-lang.org/book/ch01-03-hello-cargo.html)
- [Cargo Book - Build Configuration](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)

---

**Last Updated**: v1.0.0 (November 10, 2025)
