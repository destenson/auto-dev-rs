# PRP: Claude API Client Implementation

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 3-4 hours

## Overview
Implement Claude API client using anthropic-sdk or similar crate, providing actual LLM functionality for auto-dev-rs.

## Context and Background
The Claude provider currently returns placeholder responses. This PRP implements a real Claude client that can generate code, answer questions, and power auto-dev-rs's AI features.

### Research References
- anthropic-sdk docs: https://crates.io/crates/anthropic-sdk
- Claude API documentation: https://docs.claude.com/en/api/
- Message API guide: https://docs.claude.com/en/api/messages
- Streaming responses: https://docs.claude.com/en/api/streaming

## Requirements

### Primary Goals
1. Implement Claude provider using anthropic-sdk
2. Support messages API with system prompts
3. Enable streaming responses for real-time output
4. Handle rate limiting and retries

### Technical Constraints
- Must implement LLMProvider trait from PRP-216
- API key from environment variable CLAUDE_API_KEY
- Support Claude 3.7 Sonnet and Opus models
- Respect rate limits (requests per minute)

## Architectural Decisions

### Decision: SDK Choice
**Chosen**: anthropic-sdk crate
**Rationale**: Most mature, supports streaming, actively maintained

### Decision: Configuration
**Chosen**: Environment variables with optional config file
**Rationale**: Standard practice, secure, flexible

## Implementation Blueprint

### File Structure
Work in `auto-dev-core/src/llm/`:
- Update `claude.rs` - Replace stub with real implementation
- Update `config.rs` - Add Claude configuration
- Add tests in same file

### Key Components
1. **ClaudeProvider** - Implements LLMProvider trait
2. **ClaudeConfig** - Configuration for API key, model, etc.
3. **ClaudeClient** - Wrapper around anthropic-sdk
4. **RateLimiter** - Handle API rate limits

### Implementation Tasks (in order)
1. Add anthropic-sdk to Cargo.toml dependencies
2. Create ClaudeConfig with model selection
3. Implement ClaudeProvider with LLMProvider trait
4. Add streaming support using tokio streams
5. Implement retry logic with exponential backoff
6. Add integration tests (marked as ignored by default)

## Validation Gates

```bash
# Build and run tests first
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::claude

# Run integration tests (requires API key)
CLAUDE_API_KEY=your_key cargo test --package auto-dev-core --lib llm::claude -- --ignored

# Check for sensitive data
grep -r "sk-" auto-dev-core/src/llm/ && echo "Found API key!" || echo "No keys found"

# Then format and lint
cargo fmt --check --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Can complete basic prompts to Claude API
- Streaming responses work correctly
- Rate limiting prevents 429 errors
- API key never logged or exposed
- All tests pass (unit and integration)

## Dependencies Required
To add to Cargo.toml:
```toml
anthropic-sdk = "0.2"  # or latest version
```
Already available: tokio, serde, reqwest

## Known Patterns and Conventions
- Follow provider pattern from modules/
- Use tracing for logging, not println!
- Configuration through environment variables
- Never commit API keys

## Common Pitfalls to Avoid
- Don't hardcode API keys
- Remember to handle network errors
- Avoid blocking the async runtime
- Don't forget to implement timeout handling
- Test with actual API sparingly (costs money)

## Testing Approach
- Mock responses for unit tests
- Integration tests with real API (ignored by default)
- Test rate limiting behavior
- Test error cases (network, auth, rate limits)

## Environment Setup
Set environment variable:
```bash
export CLAUDE_API_KEY=your_api_key_here
```

## Model Support
Start with:
- claude-3-sonnet-20240229
- claude-3-opus-20240229
- claude-3-haiku-20240307

## Confidence Score: 8/10
Clear requirements and good SDK available. Main complexity is in proper error handling and rate limiting.