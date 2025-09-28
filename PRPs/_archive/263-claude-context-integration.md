# PRP: Claude Context Integration

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 2-3 hours

## Overview
Integrate CLAUDE.md content into auto-dev's existing context management system as priority context that influences all LLM interactions and decision-making.

## Context and Background
The ContextManager in auto-dev already supports priority_context. We need to integrate CLAUDE.md content as high-priority context that gets included in all LLM prompts, similar to system prompts but user-configurable.

### Research References
- Context manager: auto-dev-core/src/llm/context_manager.rs (lines 41-44)
- Priority context pattern: auto-dev-core/src/llm/context_manager.rs (lines 66-73)
- Token management: auto-dev-core/src/llm/context_manager.rs (lines 36-39)

## Requirements

### Primary Goals
1. Add CLAUDE.md content to ContextManager
2. Ensure priority placement in context window
3. Handle token limits appropriately
4. Provide clear context separation
5. Support dynamic reloading

### Technical Constraints
- Must respect token limits
- Should not override system-critical context
- Must maintain context quality
- Should be readable in prompts

## Architectural Decisions

### Decision: Integration Point
**Chosen**: Extend ContextManager with claude_context field
**Rationale**: Clean separation, easy to manage

### Decision: Priority Level
**Chosen**: High priority but below system prompts
**Rationale**: User config shouldn't override safety

## Implementation Blueprint

### File Structure
Modify in `auto-dev-core/src/`:
- Update `llm/context_manager.rs` - Add Claude context support
- Create `claude/context_integration.rs` - Integration logic
- Update `claude/mod.rs` - Export integration

### Key Components
1. **ClaudeContextProvider** - Provides CLAUDE.md content
2. **with_claude_context** - ContextManager extension
3. **format_claude_context** - Format for inclusion
4. **reload_claude_context** - Dynamic reload support

### Implementation Tasks (in order)
1. Create ClaudeContextProvider trait
2. Extend ContextManager with claude_context field
3. Implement context loading from ClaudeMdLoader
4. Add formatting with clear separators
5. Integrate into build_context method
6. Add token counting for Claude content
7. Implement reload mechanism
8. Create integration tests

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::context_manager

# Integration tests
cargo test --package auto-dev-core --lib claude::context_integration -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- CLAUDE.md content appears in all LLM prompts
- Context respects token limits
- Clear separation from other context
- Can reload without restart
- Doesn't break existing context management

## Dependencies Required
Already in project:
- ContextManager structure
- Token counting (tiktoken-rs)
- Priority context support

## Known Patterns and Conventions
- Use add_priority_context method
- Format with markdown separators
- Count tokens before adding
- Use tracing for debug logging

## Common Pitfalls to Avoid
- Don't exceed token limits
- Maintain context readability
- Handle missing CLAUDE.md gracefully
- Don't duplicate context on reload
- Preserve existing priority context

## Testing Approach
- Test with various CLAUDE.md sizes
- Test token limit enforcement
- Test context ordering
- Test reload functionality
- Test with missing files

## Confidence Score: 9/10
Clear integration point exists, straightforward extension of existing functionality.