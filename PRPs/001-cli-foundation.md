# PRP: CLI Foundation with Clap

## Overview
Implement the foundational CLI structure for auto-dev-rs using the clap crate with derive API. This establishes the command-line interface that all future features will build upon.

## Context and Background
The project currently has only a basic "Hello, world!" main.rs. We need a robust CLI foundation that supports subcommands, arguments, and extensibility for the 13 planned features outlined in the README.

### Research References
- Clap documentation: https://docs.rs/clap/latest/clap/
- Clap derive tutorial: https://github.com/clap-rs/clap/tree/master/clap_derive
- Best practices guide: https://www.shuttle.dev/blog/2023/12/08/clap-rust
- Example implementations: https://github.com/clap-rs/clap/tree/master/examples

## Requirements

### Primary Goals
1. Set up clap with derive features in Cargo.toml
2. Create modular CLI structure with subcommands
3. Implement version and help information
4. Add global flags for verbosity and configuration path
5. Create placeholder subcommands for major features

### Technical Constraints
- Use clap 4.5+ with derive feature
- Follow Rust 2024 edition idioms
- Maintain single source of truth for CLI definitions
- Support future plugin architecture integration

## Implementation Blueprint

### File Structure
```
src/
├── main.rs         # Entry point, calls CLI handler
├── cli/
│   ├── mod.rs      # CLI module exports
│   ├── app.rs      # Main CLI struct and parser
│   └── commands/   # Subcommand implementations
│       └── mod.rs
```

### Key Components
1. **Main CLI Parser**: Define root `Cli` struct with clap derive macros
2. **Subcommand Enum**: Create enum for all major feature commands
3. **Global Arguments**: Implement verbosity levels and config path override
4. **Command Dispatch**: Route subcommands to appropriate handlers

### Implementation Tasks (in order)
1. Add clap dependency with derive feature to Cargo.toml
2. Create src/cli directory structure
3. Define root Cli struct with Parser derive
4. Create Commands enum with Subcommand derive
5. Add placeholder subcommands: generate, manage, test, deploy, docs
6. Implement basic command dispatch in main.rs
7. Add global verbosity flag with multiple levels
8. Add config path override option
9. Test all commands respond with placeholder messages

## Validation Gates

```bash
# Build and format check
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Verify binary runs
cargo run -- --help
cargo run -- --version

# Test each subcommand exists
cargo run -- generate --help
cargo run -- manage --help
cargo run -- test --help

# Test global flags
cargo run -- -v generate --help
cargo run -- --config custom.toml generate --help
```

## Success Criteria
- All validation gates pass
- Help text displays for all commands
- Version information shows from Cargo.toml
- Subcommands are recognized and dispatch correctly
- Global flags are parsed and accessible

## Known Patterns and Conventions
- Use `#[derive(Parser)]` for main CLI struct
- Use `#[derive(Subcommand)]` for command enum
- Use `#[command(version, about, long_about = None)]` for metadata
- Keep all CLI types in dedicated src/cli module
- Use `#[arg(short, long)]` for flag definitions

## Common Pitfalls to Avoid
- Don't use Builder API when Derive API suffices
- Avoid string-based command matching
- Don't hardcode version strings
- Remember to propagate version to subcommands
- Don't mix CLI logic with business logic

## Dependencies Required
- clap = { version = "4.5", features = ["derive"] }

## Confidence Score: 9/10
This PRP has clear requirements, comprehensive validation gates, and references to established patterns. The scope is focused and achievable in 2-3 hours.