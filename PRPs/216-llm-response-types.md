# PRP: LLM Response Types and Base Infrastructure

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 2-3 hours

## Overview
Implement base response types and infrastructure for LLM providers, creating a unified interface that all providers (OpenAI, Claude, local models) will use.

## Context and Background
Currently, auto-dev-rs has placeholder LLM implementations that return stub responses. This PRP establishes the foundation types and traits that all LLM providers will implement.

### Research References
- async-openai response types: https://docs.rs/async-openai/latest/async_openai/types/
- anthropic-rs types: https://docs.rs/anthropic/latest/anthropic/
- OpenAI API reference: https://platform.openai.com/docs/api-reference
- Claude API reference: https://docs.claude.com/en/api/

## Requirements

### Primary Goals
1. Define common response types for all LLM operations
2. Create provider trait with standard interface
3. Implement streaming and non-streaming responses
4. Set up proper error types using thiserror

### Technical Constraints
- Must support both streaming and non-streaming
- Should handle rate limiting and retries
- Must work with existing async runtime (tokio)
- Should integrate with existing error handling (anyhow)

## Architectural Decisions

### Decision: Response Type Design
**Chosen**: Unified enum-based response types
**Rationale**: Allows different providers to return consistent types while handling provider-specific features

### Decision: Error Handling
**Chosen**: thiserror for typed errors, converting to anyhow at boundaries
**Rationale**: Following Rust best practices - typed errors in libraries, anyhow in applications

## Implementation Blueprint

### File Structure
Work in `auto-dev-core/src/llm/`:
- Update `mod.rs` - Add new modules and exports
- Create `types.rs` - Common LLM types
- Create `traits.rs` - Provider traits
- Update `errors.rs` - LLM-specific errors

### Key Components
1. **LLMResponse** - Unified response type
2. **LLMProvider** - Base trait all providers implement
3. **StreamingResponse** - For streaming completions
4. **LLMError** - Error types using thiserror

### Implementation Tasks (in order)
1. Define base types in `types.rs` (messages, roles, functions)
2. Create provider trait in `traits.rs`
3. Implement error types using thiserror in `errors.rs`
4. Add streaming response handler
5. Create mock provider for testing
6. Write unit tests

## Validation Gates

```bash
# Build and run tests first
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::

# Verify no unwrap/panic
grep -r "unwrap()" auto-dev-core/src/llm/ && echo "Found unwrap!" || echo "No unwrap found"

# Then check formatting and lints
cargo fmt --check --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Provider trait compiles and is usable
- Mock provider passes all tests
- Streaming responses work correctly
- Error handling uses Result everywhere
- No unwrap() or panic!() calls

## Dependencies Required
Already available: tokio, serde, serde_json, anyhow, thiserror
To add: None needed for this PRP

## Known Patterns and Conventions
- Follow existing module structure in auto-dev-core
- Use async-trait for async traits
- Match error handling patterns from modules/ directory
- Use builder pattern for complex types

## Common Pitfalls to Avoid
- Don't make types too OpenAI-specific
- Remember to handle rate limiting
- Avoid blocking operations in async code
- Don't forget Display implementations for errors

## Testing Approach
- Unit tests for each type
- Mock provider implementation
- Test both streaming and non-streaming
- Error case testing

## Confidence Score: 9/10
This is foundational work with clear requirements and no external dependencies. The patterns are well-established in the Rust ecosystem.