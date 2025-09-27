# PRP: OpenAI API Client Implementation

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 3-4 hours

## Overview
Implement OpenAI API client using async-openai crate, enabling GPT-4 and GPT-3.5 support for code generation.

## Context and Background
The OpenAI provider is currently a stub. This PRP implements a real OpenAI client supporting chat completions, function calling, and streaming responses.

### Research References
- async-openai documentation: https://docs.rs/async-openai/latest/async_openai/
- OpenAI API reference: https://platform.openai.com/docs/api-reference
- Chat completions guide: https://platform.openai.com/docs/guides/chat
- Function calling: https://platform.openai.com/docs/guides/function-calling

## Requirements

### Primary Goals
1. Implement OpenAI provider using async-openai
2. Support GPT-4 and GPT-3.5-turbo models
3. Enable function calling for tool use
4. Implement streaming for real-time responses

### Technical Constraints
- Must implement LLMProvider trait from PRP-216
- API key from OPENAI_API_KEY environment variable
- Handle rate limiting and token limits
- Support both Azure OpenAI and OpenAI endpoints

## Architectural Decisions

### Decision: SDK Choice
**Chosen**: async-openai crate
**Rationale**: Most popular, well-maintained, supports all OpenAI features

### Decision: Model Selection
**Chosen**: Dynamic model selection with fallback
**Rationale**: Allows using GPT-4 when needed, GPT-3.5 for speed/cost

## Implementation Blueprint

### File Structure
Work in `auto-dev-core/src/llm/`:
- Update `openai.rs` - Replace stub implementation
- Create `openai_config.rs` - Configuration types
- Update `mod.rs` - Export new types

### Key Components
1. **OpenAIProvider** - Implements LLMProvider trait
2. **OpenAIConfig** - API key, model, temperature settings
3. **FunctionRegistry** - Manage available functions
4. **TokenCounter** - Track token usage using tiktoken-rs

### Implementation Tasks (in order)
1. Add async-openai to Cargo.toml dependencies
2. Create OpenAIConfig with model and parameter settings
3. Implement OpenAIProvider with LLMProvider trait
4. Add streaming support using Server-Sent Events
5. Implement function calling capabilities
6. Add token counting and limit checking
7. Create integration tests

## Validation Gates

```bash
# Build and run tests first
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::openai

# Verify token counting
cargo test --package auto-dev-core --lib llm::openai::token_counter

# Integration tests (requires API key)
OPENAI_API_KEY=your_key cargo test --package auto-dev-core --lib llm::openai -- --ignored

# Then format and lint
cargo fmt --check --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Successfully calls OpenAI API
- Streaming responses work
- Function calling returns valid JSON
- Token limits respected
- Handles rate limiting gracefully

## Dependencies Required
To add to Cargo.toml:
```toml
async-openai = "0.24"  # or latest version
```
Already available: tiktoken-rs (for token counting)

## Known Patterns and Conventions
- Use builder pattern for request construction
- Follow existing error handling patterns
- Log API calls with tracing
- Never log full API keys

## Common Pitfalls to Avoid
- Don't exceed token limits (check before sending)
- Remember different limits for different models
- Handle incomplete streaming responses
- Don't forget Azure OpenAI compatibility
- Cache tokenizer for performance

## Testing Approach
- Mock API responses for unit tests
- Real API tests marked as ignored
- Test token counting accuracy
- Test streaming parsing
- Test error recovery

## Configuration Example
Environment variables:
```bash
export OPENAI_API_KEY=sk-...
export OPENAI_MODEL=gpt-4-turbo-preview
export OPENAI_MAX_TOKENS=4000
```

## Model Support Priority
1. gpt-4-turbo-preview (best for code)
2. gpt-4 (fallback)
3. gpt-3.5-turbo (fast/cheap option)

## Token Limits
- GPT-4: 8,192 tokens (older), 128,000 (turbo)
- GPT-3.5: 4,096 tokens (older), 16,385 (latest)

## Confidence Score: 8/10
Well-documented API with excellent SDK. Main complexity in token management and streaming.