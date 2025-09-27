# PRP: Code Generator Integration Pipeline

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 3-4 hours

## Overview
Connect all code generation components (LLM providers, prompts, templates) into a working pipeline that generates real code instead of TODO placeholders.

## Context and Background
With LLM providers, prompt templates, and code templates in place, this PRP integrates them into the existing synthesis pipeline, replacing the TODO placeholder with actual code generation.

### Research References
- Review existing pipeline: `auto-dev-core/src/synthesis/pipeline/generator.rs`
- Integration patterns from other codebases
- Pipeline architectures

## Requirements

### Primary Goals
1. Wire up LLM providers to generator
2. Connect prompt and template systems
3. Implement proper error handling
4. Add retry logic for failures

### Technical Constraints
- Must maintain existing Pipeline trait interface
- Should support multiple LLM providers
- Must handle partial failures gracefully
- Should cache results when appropriate

## Architectural Decisions

### Decision: Provider Selection
**Chosen**: Strategy pattern with runtime selection
**Rationale**: Allows switching providers based on task, cost, or availability

### Decision: Pipeline Flow
**Chosen**: Sequential with fallbacks
**Rationale**: Simple, debuggable, allows provider fallback

## Implementation Blueprint

### File Structure
Update in `auto-dev-core/src/synthesis/pipeline/`:
- Update `generator.rs` - Replace TODO implementation
- Create `llm_integration.rs` - LLM coordination
- Update `mod.rs` - New exports

### Key Components
1. **CodeGenerator** - Main generation orchestrator
2. **LLMCoordinator** - Manages LLM providers
3. **GenerationPipeline** - Full generation flow
4. **ResultCache** - Cache successful generations

### Implementation Tasks (in order)
1. Create LLMCoordinator to manage providers
2. Update generate_code in generator.rs
3. Implement prompt selection based on task
4. Add LLM calling with retry logic
5. Parse LLM response to structured format
6. Apply code templates to structure
7. Add result caching
8. Implement comprehensive error handling

## Pipeline Flow

1. Receive generation request (spec)
2. Select appropriate prompt template
3. Format prompt with context
4. Call LLM provider (with fallback)
5. Parse JSON response
6. Apply code template
7. Format and validate result
8. Return generated code

## Error Handling Strategy

- LLM failures: Retry with exponential backoff
- Parse failures: Try simpler prompt
- Template failures: Return raw LLM output
- Network failures: Use cached results if available

## Validation Gates

```bash
# Build and test first
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib synthesis::pipeline

# Integration test
cargo test --package auto-dev-core --test code_generation

# End-to-end test (requires LLM keys)
OPENAI_API_KEY=... cargo run -- generate "create a fibonacci function"

# Then format and lint
cargo fmt --check --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Generates actual code, not TODOs
- Works with at least one LLM provider
- Handles errors gracefully
- Generated code compiles/runs
- Performance under 5 seconds for simple tasks

## Dependencies Required
All dependencies from previous PRPs:
- LLM providers (PRP-217, 218)
- Template system (PRP-219)
- Prompt templates (PRP-220)

## Known Patterns and Conventions
- Use existing PipelineStage trait
- Follow error handling patterns
- Use tracing for debugging
- Cache expensive operations

## Common Pitfalls to Avoid
- Don't block on LLM calls indefinitely
- Handle malformed LLM responses
- Don't cache failed results
- Remember timeout handling
- Test with various input types

## Integration Points

Connect with:
- `LLMProvider` trait (PRP-216)
- `TemplateEngine` (PRP-219)
- `PromptManager` (PRP-220)
- Existing `PipelineStage` trait

## Testing Approach
- Unit test each component
- Mock LLM responses
- Test error scenarios
- Integration test full pipeline
- End-to-end with real LLMs

## Performance Considerations
- Cache LLM responses (expensive)
- Reuse template engine
- Parallel LLM calls if multiple
- Timeout long-running operations

## Configuration
```toml
[synthesis.generator]
primary_provider = "claude"
fallback_provider = "openai"
timeout_seconds = 30
cache_ttl_seconds = 3600
max_retries = 3
```

## Confidence Score: 7/10
Integration work with multiple components. Complexity in error handling and fallback logic.