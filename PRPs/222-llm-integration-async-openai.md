# PRP: LLM Integration using async-openai

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 2-3 hours

## Overview
Integrate async-openai crate as the primary OpenAI provider, implementing streaming, function calling, and proper token management for production use.

## Context and Background
Building on PRP-218's basic OpenAI client, this PRP adds production features: streaming responses for better UX, function calling for tool use, and token management to prevent limit violations.

### Research References
- async-openai streaming: https://github.com/64bit/async-openai/tree/main/examples/stream
- Function calling guide: https://github.com/64bit/async-openai/blob/main/examples/function_calling
- Token counting: https://docs.rs/tiktoken-rs/latest/tiktoken_rs/

## Requirements

### Primary Goals
1. Implement streaming chat completions
2. Add function calling capability
3. Implement token counting and management
4. Add conversation context management

### Technical Constraints
- Must not exceed model token limits
- Should handle stream interruptions
- Must parse function call responses
- Should manage conversation history

## Architectural Decisions

### Decision: Streaming Implementation
**Chosen**: Server-Sent Events with tokio::sync::mpsc
**Rationale**: Reliable, supports backpressure, integrates with async

### Decision: Token Management
**Chosen**: Pre-count with tiktoken-rs
**Rationale**: Prevents API errors, allows context trimming

## Implementation Blueprint

### File Structure
Enhance in `auto-dev-core/src/llm/`:
- Update `openai.rs` - Add streaming support
- Create `token_manager.rs` - Token counting
- Create `function_caller.rs` - Function execution
- Update `conversation.rs` - Context management

### Key Components
1. **StreamingClient** - Handles SSE streams
2. **TokenManager** - Counts and manages tokens
3. **FunctionCaller** - Executes function calls
4. **ConversationManager** - Maintains context

### Implementation Tasks (in order)
1. Implement streaming response handler
2. Add token counting with tiktoken-rs
3. Create conversation context manager
4. Implement function call parser
5. Add context trimming for token limits
6. Create streaming integration tests

## Validation Gates

```bash
# Build and test first
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::

# Then format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Integration test with streaming
OPENAI_API_KEY=... cargo test --package auto-dev-core --lib llm::streaming -- --ignored
```

## Success Criteria
- Streaming responses arrive in real-time
- Token limits never exceeded
- Function calls execute correctly
- Context managed efficiently
- No stream interruption crashes

## Dependencies Required
Already in project:
- async-openai (from PRP-218)
- tiktoken-rs (already in Cargo.toml)
- tokio with sync features

## Known Patterns and Conventions
- Use channels for streaming data
- Implement Drop for cleanup
- Use Arc<Mutex> for shared state
- Follow existing async patterns

## Common Pitfalls to Avoid
- Don't buffer entire stream
- Handle partial JSON in streams
- Clean up on stream errors
- Don't leak memory with history
- Test with slow connections

## Testing Approach
- Mock SSE responses
- Test token counting accuracy
- Test context trimming
- Test function parsing
- Stress test streaming

## Confidence Score: 8/10
Clear implementation path with good examples available.