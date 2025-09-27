# PRP: CLI Generate Command Implementation

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 2-3 hours

## Overview
Implement the CLI `generate` command that connects user input to the code synthesis pipeline, providing the primary interface for code generation.

## Context and Background
The generate command currently returns a placeholder. This PRP implements the full command, parsing user specifications and invoking the synthesis pipeline.

### Research References
- Clap documentation: https://docs.rs/clap/latest/clap/
- Existing CLI structure: `auto-dev/src/cli/commands/generate.rs`

## Requirements

### Primary Goals
1. Parse generation specifications from CLI
2. Invoke synthesis pipeline
3. Handle output formatting and file writing
4. Provide progress feedback

### Technical Constraints
- Must work with existing CLI structure
- Should support multiple output formats
- Must handle long-running operations
- Should provide meaningful error messages

## Architectural Decisions

### Decision: Input Format
**Chosen**: Natural language with optional flags
**Rationale**: User-friendly, flexible

### Decision: Progress Feedback
**Chosen**: indicatif progress bars
**Rationale**: Already in dependencies, good UX

## Implementation Blueprint

### File Structure
Update in `auto-dev/src/cli/commands/`:
- Update `generate.rs` - Implement command
- Create `generate_handler.rs` - Business logic
- Update progress reporting

### Key Components
1. **GenerateCommand** - CLI argument parsing
2. **GenerateHandler** - Orchestrates generation
3. **ProgressReporter** - User feedback
4. **OutputFormatter** - Formats results

### Implementation Tasks (in order)
1. Define CLI arguments with clap
2. Create handler to process requests
3. Connect to synthesis pipeline
4. Add progress reporting with indicatif
5. Implement file output options
6. Add error handling and recovery

## CLI Interface

```bash
# Basic usage
auto-dev generate "create a REST API client"

# With options
auto-dev generate "fibonacci function" --language rust --output fib.rs

# From specification file  
auto-dev generate --spec api.yaml --output src/
```

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev
cargo test --package auto-dev --bin auto-dev

# Then format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# CLI integration test
cargo run -- generate "test function" --dry-run
```

## Success Criteria
- Command parses arguments correctly
- Invokes synthesis pipeline
- Shows progress during generation
- Outputs generated code
- Handles errors gracefully

## Dependencies Required
Already available:
- clap (CLI parsing)
- indicatif (progress bars)
- tokio (async runtime)

## Known Patterns and Conventions
- Use clap derive for argument parsing
- Follow existing command structure
- Use Result for error handling
- Provide helpful error messages

## Common Pitfalls to Avoid
- Don't block the async runtime
- Handle Ctrl+C gracefully
- Validate paths before writing
- Don't overwrite without confirmation
- Provide clear error messages

## Testing Approach
- Unit test argument parsing
- Test handler logic
- Integration test with pipeline
- Test file output
- Test error scenarios

## Confidence Score: 9/10
Clear requirements, straightforward implementation using existing patterns.