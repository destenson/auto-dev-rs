# PRP: Iterative Build and Fix Loop

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 3-4 hours

## Overview
Implement an automated loop that attempts to build generated code, captures errors, and iteratively fixes them using pattern matching and optional local model assistance. This provides self-healing code generation.

## Context and Background
Generated code often has minor issues - missing imports, type mismatches, syntax errors. This engine runs build commands, parses error output, and applies fixes automatically until the code compiles.

### Research References
- Rust compiler error formats: https://doc.rust-lang.org/rustc/json.html
- Error correction patterns: https://github.com/rust-lang/rust-analyzer
- Similar to existing validation module patterns

## Requirements

### Primary Goals
1. Execute build commands for different languages
2. Parse compiler/interpreter errors
3. Apply automated fixes based on error patterns
4. Iterate until success or max attempts

### Technical Constraints
- Must handle multiple language toolchains
- Parse errors without external tools
- Limit iterations to prevent infinite loops
- Work without LLM for basic fixes

## Architectural Decisions

### Decision: Error Parsing Strategy
**Chosen**: Regex patterns per language/tool
**Rationale**: Reliable, fast, no dependencies

### Decision: Fix Application
**Chosen**: Pattern-based fixes with optional LLM
**Rationale**: Deterministic for common errors, smart for complex

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/iteration/`:
- `mod.rs` - Public interface
- `builder.rs` - Build execution logic
- `error_parser.rs` - Error extraction
- `fixer.rs` - Fix application engine
- `patterns.rs` - Common error patterns

### Key Components
1. **BuildRunner** - Executes build commands
2. **ErrorParser** - Extracts actionable errors
3. **FixEngine** - Applies corrections
4. **IterationController** - Manages retry loop

### Implementation Tasks (in order)
1. Create build runner for cargo/python/node
2. Implement error parsers for each toolchain
3. Define common error fix patterns
4. Build fix application engine
5. Add iteration controller with limits
6. Integrate with code generation pipeline

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core iteration

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Test iteration with broken code
echo "fn main() { println!(x) }" > test.rs
auto-dev fix test.rs --max-iterations 3

# Test with various error types
auto-dev test-iteration examples/broken/
```

## Success Criteria
- Fixes missing semicolons automatically
- Adds missing imports for common types
- Resolves basic type mismatches
- Stops after max iterations
- Preserves working code
- Handles multiple files

## Dependencies Required
- regex (already in use)
- Similar to existing incremental module
- No new major dependencies

## Known Patterns and Conventions
- Use existing executor patterns from incremental module
- Follow validation module error handling
- Implement async execution for builds
- Store iteration history for debugging

## Common Pitfalls to Avoid
- Infinite fix loops
- Corrupting working code
- Not preserving user intent
- Over-aggressive fixing
- Language-specific assumptions

## Testing Approach
- Test each error parser individually
- Test fix patterns with examples
- Mock build commands for speed
- Test iteration limits
- Verify idempotent fixes

## Confidence Score: 8/10
Similar patterns exist in codebase, clear error formats to parse.