# PRP: Project Bootstrap and Initial Setup

## Overview
Bootstrap the auto-dev-rs project with proper Rust workspace structure, development dependencies, and initial documentation. This PRP should be executed first to establish the foundation for all other work.

## Context and Background
Before implementing features, we need proper project structure, development tools, and workflows. This includes setting up Cargo workspace, adding essential dependencies, and creating development documentation.

### Research References
- Cargo workspace documentation: https://doc.rust-lang.org/cargo/reference/workspaces.html
- Rust project structure best practices: https://doc.rust-lang.org/cargo/guide/project-layout.html
- GitHub Actions for Rust: https://github.com/actions-rs/toolchain

## Requirements

### Primary Goals
1. Set up Cargo workspace structure
2. Configure development dependencies
3. Create GitHub Actions workflows
4. Set up pre-commit hooks
5. Initialize documentation structure

### Technical Constraints
- Use Rust 2021 edition (compatible with 2024)
- Support cross-platform development
- Enable incremental compilation
- Configure for optimal development experience

## Implementation Blueprint

### File Structure
```
auto-dev-rs/
├── Cargo.toml           # Workspace root
├── auto-dev/
│   ├── Cargo.toml       # Main binary crate
│   └── src/
│       └── main.rs
├── auto-dev-core/
│   ├── Cargo.toml       # Core library crate
│   └── src/
│       └── lib.rs
├── .github/
│   └── workflows/
│       ├── ci.yml       # CI pipeline
│       └── release.yml  # Release automation
├── .gitignore
├── rustfmt.toml         # Formatting config
├── .clippy.toml         # Linter config
├── CONTRIBUTING.md
├── CHANGELOG.md
└── LICENSE
```

### Workspace Configuration
```toml
# Root Cargo.toml
[workspace]
members = ["auto-dev", "auto-dev-core"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Auto Dev Team"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/auto-dev-rs"

[workspace.dependencies]
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
```

### Implementation Tasks (in order)
1. Create workspace directory structure
2. Initialize Cargo workspace configuration
3. Set up auto-dev binary crate
4. Create auto-dev-core library crate
5. Add rustfmt.toml configuration
6. Configure clippy lints
7. Create .gitignore file
8. Set up GitHub Actions CI workflow
9. Add pre-commit hooks configuration
10. Create CONTRIBUTING.md guide
11. Initialize CHANGELOG.md
12. Add MIT and Apache-2.0 licenses

## Development Configuration

### rustfmt.toml
```toml
edition = "2021"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
```

### .clippy.toml
```toml
msrv = "1.70.0"
warn-on-all-wildcard-imports = true
```

### GitHub Actions CI
```yaml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --all-features
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check
```

## Validation Gates

```bash
# Initialize git repository
git init

# Build workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Check formatting
cargo fmt --all -- --check

# Run clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Generate documentation
cargo doc --no-deps --workspace

# Verify workspace members
cargo metadata --format-version 1 | jq '.workspace_members'
```

## Success Criteria
- Workspace builds without errors
- All crates are properly linked
- CI pipeline passes
- Development tools are configured
- Documentation generates correctly

## Known Patterns and Conventions
- Use workspace inheritance for dependencies
- Separate binary from library code
- Keep Cargo.toml organized and documented
- Use conventional commit messages
- Follow Rust API guidelines

## Common Pitfalls to Avoid
- Don't mix workspace and package dependencies
- Avoid circular dependencies between crates
- Remember to update all Cargo.toml files
- Don't commit Cargo.lock for libraries
- Keep workspace members focused

## Development Scripts

### Makefile (optional)
```makefile
.PHONY: build test fmt lint clean

build:
	cargo build --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

clean:
	cargo clean
```

### Pre-commit Hook
```bash
#!/bin/sh
# .git/hooks/pre-commit

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Documentation Structure

### CONTRIBUTING.md
- Development setup instructions
- Code style guidelines
- PR process
- Testing requirements
- Documentation standards

### README Updates
- Add badges (CI status, crates.io, docs.rs)
- Installation instructions
- Quick start guide
- Development setup
- License information

## Confidence Score: 10/10
Standard Rust project setup with well-established patterns. All steps are straightforward with excellent tooling support.