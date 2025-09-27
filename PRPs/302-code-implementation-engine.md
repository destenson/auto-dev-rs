# PRP: Code Implementation Engine

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 3-4 hours

## Overview
Build the core engine that takes parsed instructions and generates initial code files using a combination of rule-based generation and optional local model assistance. Works primarily through deterministic patterns with local LLM as enhancement.

## Context and Background
After project initialization and instruction parsing, this engine creates the actual code files. It uses pattern matching, templates, and optionally the local Qwen model for enhanced generation. Must work offline without external APIs.

### Research References
- Existing code synthesis in `auto-dev-core/src/synthesis/`
- Local model at `models/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf`
- Template patterns: https://handlebarsjs.com/guide/

## Requirements

### Primary Goals
1. Generate code files from parsed instructions
2. Use deterministic patterns as primary approach
3. Optionally enhance with local Qwen model
4. Create sensible project structure

### Technical Constraints
- Must work without network access
- Prioritize deterministic generation
- Local model optional enhancement only
- Use existing synthesis module patterns

## Architectural Decisions

### Decision: Generation Strategy
**Chosen**: Rule-based with optional LLM enhancement
**Rationale**: Predictable, works offline, debuggable

### Decision: File Structure Generation
**Chosen**: Convention-based directory layout
**Rationale**: Follows ecosystem standards, no magic

## Implementation Blueprint

### File Structure
Enhance in `auto-dev-core/src/synthesis/`:
- Update `generator.rs` - Main generation logic
- Create `patterns.rs` - Rule-based patterns
- Create `structure.rs` - Project structure builder
- Update `local_model.rs` - Qwen integration

### Key Components
1. **ImplementationEngine** - Orchestrates generation
2. **PatternMatcher** - Maps instructions to code patterns
3. **StructureBuilder** - Creates directory/file layout
4. **LocalEnhancer** - Optional Qwen model integration

### Implementation Tasks (in order)
1. Define common code patterns for each language
2. Build pattern matcher for instruction keywords
3. Create structure builder for project layout
4. Implement basic file generation from patterns
5. Add local model enhancement for complex parts
6. Integrate with instruction parser output

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core synthesis

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Test generation
auto-dev generate --from instructions.md --output ./test-project
auto-dev generate "create a fibonacci function in rust"

# Verify generated code compiles
cd test-project && cargo build
```

## Success Criteria
- Generates compilable code for basic instructions
- Creates appropriate project structure
- Works without local model present
- Enhanced quality with model available
- Handles multiple file generation
- Respects language conventions

## Dependencies Required
- handlebars (for template generation) - optional
- Existing llm module for local model
- No new external dependencies

## Known Patterns and Conventions
- Follow existing synthesis module structure
- Use async for model calls
- Return Result<Vec<GeneratedFile>>
- Integrate with existing generator trait

## Common Pitfalls to Avoid
- Don't rely solely on LLM generation
- Keep patterns language-idiomatic
- Don't generate overcomplicated code
- Handle model unavailability gracefully
- Avoid generating broken imports

## Testing Approach
- Test pattern matching logic
- Test structure generation
- Mock local model for testing
- Verify generated code compiles
- Test multiple language targets

## Confidence Score: 8/10
Leverages existing synthesis infrastructure, clear fallback strategy without LLM.