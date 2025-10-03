# Copilot Instructions for Auto-Dev-RS

This codebase is an autonomous development system that monitors specifications and automatically implements required code. Understanding the architecture and patterns is crucial for effective contributions.

## 🚧 Alpha Development Status

**This project is in pre-alpha development** - we're working toward the first alpha release with substantial work remaining:

- **59% completion rate** (30/51 PRPs implemented) - see `PRPs/README.md` for current status
- Many CLI commands return placeholder implementations ("coming soon" messages)
- Core code generation pipeline exists but needs LLM provider integration
- 327+ `unwrap()` calls need conversion to proper error handling
- Test coverage and documentation gaps throughout

**Key Challenge**: Much work has been completed architecturally but remains unintegrated or stubbed. The module system, safety gates, and LLM infrastructure exist but many components return placeholder responses or aren't wired together properly.

**Current Priority**: Focus on completing foundation PRPs and making core functionality actually work rather than adding new features. The system has excellent architecture but needs implementation depth to reach alpha quality.

## Project Architecture

### Workspace Structure
- **auto-dev-rs** - Cargo workspace with 3 main crates:
  - `auto-dev/` - CLI binary with subcommands (generate, test, deploy, docs, analyze, loop, claude)
  - `auto-dev-core/` - Core business logic with 20+ modules
  - `regex-utils/` - Shared regex utilities

### Core Modules Overview
```
auto-dev-core/src/
├── bootstrap/         # Self-development initialization
├── claude/           # Claude API and command integration  
├── context/          # Project understanding and analysis
├── dev_loop/         # Main autonomous development loop
├── docs/             # Documentation generation system
├── incremental/      # Step-by-step implementation
├── instructions/     # Instruction parsing and management
├── learning/         # Pattern learning and improvement
├── llm/              # Multi-provider LLM integration
├── mcp/              # Model Context Protocol support
├── metrics/          # Self-improvement tracking
├── modules/          # Dynamic module system with WASM
├── monitor/          # Filesystem and specification monitoring
├── parser/           # Specification and document parsing
├── safety/           # 5-layer safety validation gates
├── self_*/           # Self-development capabilities
├── synthesis/        # Code generation and templates
├── test_gen/         # Test generation frameworks
├── validation/       # Code quality and correctness checks
└── vcs/              # Version control operations
```

## Essential Patterns

### Error Handling
- Use `anyhow::Result<T>` for application boundaries
- Use `thiserror::Error` for library error types
- **AVOID** `.unwrap()` and `.expect()` - the codebase is actively removing 327+ instances
- Prefer `?` operator and proper error propagation with relevant context and inpection logging

### Async Patterns
- All I/O operations are async with `tokio`
- Use `async-trait` for async traits consistently
- CLI commands create `tokio::runtime::Runtime` and call `block_on()`
- Module interfaces are `async fn` with `Send + Sync` bounds

### Module Architecture
- All modules implement `ModuleInterface` trait with `async-trait`
- Modules support hot-reload via `get_state()` and `restore_state()`
- Sandboxing enforced through `ModuleSandbox` with capability model
- WASM modules loaded via `wasmtime` with resource limits

### Safety-First Development
- 5-layer safety gates: Static → Semantic → Security → Performance → Reversibility
- All self-modifications go through `SafetyGatekeeper::validate()`
- Resource monitoring and violation detection in sandboxed execution
- Audit logging for all security events with severity levels

## Development Workflows

### Build and Test Commands
```bash
# Standard build - works across all platforms
cargo build --workspace --all-features

# Run tests with proper async runtime
cargo test --workspace --all-features

# Check for formatting issues
cargo fmt --all -- --check

# Lint with project-specific rules
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Self-targeting mode (auto-dev analyzing itself)
cargo run -- analyze --target-self .
cargo run -- loop start --target-self
```

### CLI Integration Pattern
```rust
// In main.rs - CLI commands use this pattern:
Commands::Generate(args) => {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(cli::commands::generate::execute(args))?;
}

// In command modules - async execution:
pub async fn execute(args: GenerateArgs) -> anyhow::Result<()> {
    // Implementation uses await throughout
}
```

### PRP (Project Requirement Plan) System
- PRPs are implementation blueprints in `PRPs/` directory
- Use `.claude/commands/prp-executor.md` for systematic PRP implementation
- Each PRP is scoped to 2-4 hours of work
- 51 total PRPs with 59% completion rate tracked in `PRPs/README.md`

## Critical Integration Points

### LLM Provider Integration
- Multi-provider support: Claude, OpenAI, Ollama, OpenRouter (400+ models)
- Tiered routing system: No LLM → Tiny → Small → Medium → Large models
- Cost optimization with 87% reduction expected
- All providers implement `LLMProvider` trait

### Specification-Driven Development
- Monitors markdown files for specification changes
- Parses requirements from `SPEC.md`, `README.md`, test files
- Incremental implementation with safety validation
- File monitoring via `notify` crate with debouncing

### Self-Targeting Mode
- `--target-self` flag enables auto-dev to improve itself
- Creates `.auto-dev/self.toml` configuration
- Safety features prevent dangerous self-modifications
- Cargo metadata integration for workspace discovery

## Code Generation Patterns

### Template System
- Templates in `synthesis/templates/` for multiple languages
- Generator trait implementations for different strategies
- Context-aware generation using project analysis
- Pattern reuse reduces LLM calls by 50%

### Test Generation
- Framework-specific generators (Jest, pytest, Rust)
- Acceptance criteria validation
- Property-based testing integration
- Regression test creation

## Key Conventions

### File Organization
- Modules use `mod.rs` for public interface
- Tests in same file as implementation or `tests/` subdirectory
- Examples in `auto-dev-core/examples/` for demonstration
- Benchmarks in `auto-dev-core/benches/` for performance tracking

### Configuration Management
- TOML configuration files (Cargo workspace pattern)
- Models configuration in `auto-dev-core/models.toml`
- Claude commands in `.claude/commands/` directory
- Project configs in `.auto-dev/` directory

### Documentation Standards
- Comprehensive module-level docs with `//!`
- Public APIs documented with examples
- Architectural decisions captured in PRPs
- Changelog follows semantic versioning

## Working with This Codebase

### Before Making Changes
1. Read relevant PRPs to understand the intended architecture
2. Check `PRPs/README.md` for current status and gaps
3. Run existing tests to ensure baseline functionality
4. Use `--target-self` mode to test changes on the codebase itself

### When Adding Features
1. Follow the PRP system - create a PRP for substantial features
2. Implement safety validation for any self-modification capabilities
3. Add proper error handling (no unwrap/expect)
4. Include tests and documentation
5. Update metrics tracking if applicable

### Common Tasks
- **Adding LLM providers**: Implement `LLMProvider` trait in `llm/providers/`
- **Module development**: Use `ModuleInterface` trait and sandbox integration
- **CLI commands**: Follow async pattern with proper error handling
- **Safety features**: Extend validation gates in `safety/validators.rs`

This codebase represents a new paradigm where specifications drive implementation automatically. Focus on understanding the safety mechanisms, async patterns, and modular architecture to be immediately productive.

## Future Enhancements

### Web Frontend for Process Visibility
A web-based dashboard is planned to provide real-time visibility into auto-dev's autonomous processes:

- **Live Process Monitor**: View active development loops, file monitoring, and LLM interactions
- **Safety Gate Dashboard**: Real-time validation status and security events from the 5-layer safety system
- **Metrics Visualization**: Self-improvement progress, pattern learning, and cost optimization charts
- **Module Sandbox View**: Resource usage, capability violations, and audit logs
- **PRP Progress Tracker**: Visual representation of implementation status and dependencies
- **LLM Usage Analytics**: Provider selection, cost tracking, and response quality metrics

This would be particularly valuable for debugging autonomous behavior and understanding system decision-making in real-time.

