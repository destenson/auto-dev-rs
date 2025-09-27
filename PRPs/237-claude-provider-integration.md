# PRP: Claude Code LLM Provider Implementation

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 4 hours

## Overview
Integrate the Claude Code CLI components into a complete LLMProvider implementation that can be used throughout the auto-dev-rs system.

## Context and Background
This PRP ties together the detection, execution, parsing, and session components into a cohesive LLMProvider that implements the standard trait and integrates with the router.

### Research References
- LLMProvider trait in `auto-dev-core/src/llm/provider.rs`
- Existing CLI provider in `auto-dev-core/src/llm/cli_tools.rs` lines 59-120
- Router integration in `auto-dev-core/src/llm/router.rs` lines 96-112

## Requirements

### Primary Goals
1. Implement full LLMProvider trait
2. Integrate with detection and execution
3. Support all provider methods
4. Auto-register with router when available
5. Provide proper tier classification

### Technical Constraints
- Must implement async trait methods
- Should cache provider availability
- Must handle all error cases
- Support configuration via environment

## Architectural Decisions

### Decision: Provider Tier
**Chosen**: ModelTier::Large
**Rationale**: Claude Code uses full Claude models

### Decision: Auto-registration
**Chosen**: Register if binary detected
**Rationale**: Zero-config when Claude installed

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/claude_code/`:
- Create `provider.rs` - LLMProvider implementation
- Create `config.rs` - Configuration types
- Update `mod.rs` - Export provider and types
- Update parent `llm/mod.rs` - Add claude_code module

### Key Components
1. **ClaudeCodeProvider** - Main provider struct
2. **ClaudeCodeConfig** - Configuration
3. Integration with detector, executor, parser
4. Session management integration

### Implementation Tasks (in order)
1. Create ClaudeCodeProvider struct
2. Implement LLMProvider trait methods
3. Integrate detector for availability check
4. Wire up executor and parser
5. Add session management support
6. Implement all trait methods
7. Add router auto-registration
8. Write integration tests

## Provider Methods Implementation

### Core Methods
- `name()` → "claude-code"
- `tier()` → ModelTier::Large
- `is_available()` → Check detector
- `cost_per_1k_tokens()` → 0.0 (subscription-based)

### Generation Methods
- `generate_code()` → Build prompt, execute, parse
- `explain_implementation()` → Format as question
- `review_code()` → Structured review prompt
- `answer_question()` → Direct prompt execution

### Complex Methods
- `classify_content()` → Parse classification
- `assess_complexity()` → Analyze response
- Stream variants → Not supported initially

## Configuration

Environment variables:
```
CLAUDE_BINARY_PATH=/custom/path/to/claude
CLAUDE_MODEL=claude-3-5-sonnet
CLAUDE_TIMEOUT=60
CLAUDE_SESSION_DIR=~/.auto-dev/sessions
CLAUDE_OUTPUT_FORMAT=json
```

Config structure with defaults and overrides.

## Router Integration

Auto-register in router.rs:
```rust
// After OpenRouter registration
if ClaudeCodeProvider::is_available_static().await {
    let provider = ClaudeCodeProvider::new(Default::default())?;
    self.register_provider(Arc::new(provider));
    info!("Registered Claude Code provider");
}
```

## Error Handling

Comprehensive error types:
- BinaryNotFound → With installation instructions
- ExecutionFailed → With command details
- ParsingError → With raw output
- TimeoutError → With duration
- SessionError → With session ID

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::claude_code

# Test provider trait implementation
cargo test --package auto-dev-core --lib llm::claude_code::provider

# Integration test with real Claude
cargo test --package auto-dev-core --lib llm::claude_code::integration -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Fully implements LLMProvider trait
- Auto-registers when Claude available
- All methods return valid responses
- Proper error handling throughout
- Session continuity works

## Dependencies Required
No new dependencies, uses:
- async-trait (already included)
- Components from PRPs 233-236
- Existing LLM infrastructure

## Known Patterns and Conventions
- Follow ClaudeCLIProvider pattern
- Use Arc for shared state
- Implement Default for config
- Log with tracing macros

## Common Pitfalls to Avoid
- Don't block async runtime
- Cache availability check
- Handle missing binary gracefully
- Test all trait methods
- Validate prompt escaping

## Testing Approach
- Unit tests with mocked components
- Test each trait method
- Integration test with real Claude
- Test error scenarios
- Benchmark performance

## Method-Specific Notes

### generate_code
- Format spec clearly
- Include language context
- Parse code blocks from response

### review_code
- Structure requirements clearly
- Parse feedback into issues
- Extract suggestions

### classify_content
- Use specific prompt format
- Parse JSON or structured response
- Provide confidence scores

## Confidence Score: 9/10
Clear integration path with well-defined interfaces and patterns.