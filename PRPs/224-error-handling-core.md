# PRP: Error Handling Standardization - Core Modules

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 3-4 hours

## Overview
Replace all unwrap() and expect() calls in core modules with proper error handling using thiserror for library code and anyhow for application boundaries.

## Context and Background
The codebase has 327 unwrap/expect calls that can cause panics. This PRP focuses on core modules (synthesis, validation, monitoring) to establish proper error handling patterns.

### Research References
- thiserror documentation: https://docs.rs/thiserror
- Error handling best practices: https://github.com/rust-lang/project-error-handling
- Existing error types in codebase

## Requirements

### Primary Goals
1. Eliminate unwrap() in synthesis module
2. Eliminate unwrap() in validation module  
3. Eliminate unwrap() in monitoring module
4. Create proper error types with thiserror

### Technical Constraints
- Maintain existing public APIs
- Preserve error context
- No performance degradation
- Must be backward compatible

## Architectural Decisions

### Decision: Error Strategy
**Chosen**: thiserror for types, ? operator for propagation
**Rationale**: Idiomatic Rust, good ergonomics

### Decision: Error Granularity  
**Chosen**: Module-specific error enums
**Rationale**: Clear error sources, better handling

## Implementation Blueprint

### File Structure
Create/update in each module:
- `synthesis/error.rs` - Synthesis errors
- `validation/error.rs` - Validation errors
- `monitoring/error.rs` - Monitoring errors

### Key Components
1. **SynthesisError** - Code generation errors
2. **ValidationError** - Validation failures
3. **MonitoringError** - File watching errors
4. **ErrorContext** - Additional error info

### Implementation Tasks (in order)
1. Create error types with thiserror
2. Replace unwrap() in synthesis module
3. Replace unwrap() in validation module
4. Replace unwrap() in monitoring module
5. Add error context where helpful
6. Update tests for new error handling

## Error Type Examples

Define errors like:
- `SynthesisError::TemplateNotFound`
- `SynthesisError::LLMFailure`
- `ValidationError::SpecInvalid`
- `MonitoringError::PathNotFound`

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core

# Then check for unwrap
! grep -r "unwrap()" auto-dev-core/src/synthesis/
! grep -r "unwrap()" auto-dev-core/src/validation/
! grep -r "unwrap()" auto-dev-core/src/monitor/

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- No unwrap() in targeted modules
- All tests still pass
- Clear error messages
- No panics in normal operation
- Error context preserved

## Dependencies Required
Already available:
- thiserror (in workspace)
- anyhow (in workspace)

## Known Patterns and Conventions
- Use `#[derive(thiserror::Error)]`
- Include source errors with `#[from]`
- Add context with `#[error("...")]`
- Convert to anyhow at boundaries

## Common Pitfalls to Avoid
- Don't lose error context
- Avoid generic error messages
- Don't hide underlying causes
- Test error paths
- Document error conditions

## Migration Strategy
1. Start with leaf functions
2. Work up to public APIs
3. Update tests as you go
4. Keep commits focused
5. Run tests frequently

## Testing Approach
- Test each error variant
- Test error propagation
- Test error display messages
- Ensure no panics
- Test error recovery

## Example Transformations
Replace patterns like:
- `x.unwrap()` → `x?` or `x.context("...")?`
- `x.expect("msg")` → `x.ok_or_else(|| Error::X)?`
- `panic!("...")` → `return Err(Error::X)`

## Confidence Score: 8/10
Mechanical refactoring with clear patterns. Time consuming but straightforward.