# PRP: Groq and Perplexity Provider Implementation

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 2-3 hours

## Overview
Implement Groq and Perplexity AI providers leveraging their OpenAI compatibility, enabling fast inference (Groq) and real-time search capabilities (Perplexity).

## Context and Background
Both Groq and Perplexity offer OpenAI-compatible APIs, making integration straightforward. Groq provides ultra-fast inference, while Perplexity adds real-time web search with citations.

### Research References
- Groq OpenAI compatibility: https://console.groq.com/docs/openai
- Perplexity API docs: https://docs.perplexity.ai/
- Groq models: https://console.groq.com/docs/models
- Perplexity blog: https://www.perplexity.ai/hub/blog/introducing-pplx-api

## Requirements

### Primary Goals
1. Implement Groq provider with OpenAI client
2. Implement Perplexity provider with search
3. Handle provider-specific features
4. Support streaming for both

### Technical Constraints
- Reuse OpenAI client code where possible
- Preserve provider-specific features
- Handle different rate limits
- Support citation extraction (Perplexity)

## Architectural Decisions

### Decision: Implementation Strategy
**Chosen**: Extend OpenAI provider with overrides
**Rationale**: Maximum code reuse, proven compatibility

### Decision: Citation Handling
**Chosen**: Extended response type for Perplexity
**Rationale**: Preserves unique search capabilities

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/`:
- Create `groq.rs` - Groq provider
- Create `perplexity.rs` - Perplexity provider
- Update `types.rs` - Add citation types

### Key Components
1. **GroqProvider** - Fast inference provider
2. **PerplexityProvider** - Search-enabled provider
3. **CitationResponse** - Extended response with sources
4. **ProviderAdapter** - OpenAI compatibility layer

### Implementation Tasks (in order)
1. Create base OpenAI-compatible adapter
2. Implement Groq provider with endpoint override
3. Implement Perplexity provider with search
4. Add citation extraction for Perplexity
5. Configure model mappings
6. Add provider-specific tests

## Provider Configurations

### Groq
- **Endpoint**: https://api.groq.com/openai/v1
- **Models**: llama-3.2-90b-vision-preview, mixtral-8x7b, gemma2-9b
- **Special**: Ultra-low latency, high throughput
- **Rate limits**: Higher than OpenAI

### Perplexity
- **Endpoint**: https://api.perplexity.ai/chat/completions
- **Models**: llama-3.1-sonar-large, llama-3.1-sonar-small
- **Special**: Real-time web search, citations
- **Rate limits**: Varies by plan

## Extended Response Type

For Perplexity citations:
```
CitationResponse {
  content: String,
  citations: Vec<Citation>,
  search_results: Vec<SearchResult>,
}
```

## Environment Configuration

```bash
# Groq
GROQ_API_KEY=gsk_...
GROQ_MODEL=llama-3.2-90b-vision-preview

# Perplexity
PERPLEXITY_API_KEY=pplx-...
PERPLEXITY_MODEL=llama-3.1-sonar-large
PERPLEXITY_SEARCH=true
```

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::groq
cargo test --package auto-dev-core --lib llm::perplexity

# Integration tests (require API keys)
GROQ_API_KEY=... cargo test --lib llm::groq -- --ignored
PERPLEXITY_API_KEY=... cargo test --lib llm::perplexity -- --ignored

# Test search functionality
cargo test --lib llm::perplexity::search -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Both providers work with existing code
- Groq demonstrates faster response times
- Perplexity returns citations
- Streaming works for both
- Provider-specific features accessible

## Dependencies Required
Reuse from PRP-218:
- async-openai (with endpoint override)
No new dependencies needed

## Known Patterns and Conventions
- Extend OpenAI provider pattern
- Override base_url and headers
- Preserve LLMProvider interface
- Add provider-specific options

## Common Pitfalls to Avoid
- Don't lose provider-specific features
- Remember different model names
- Handle rate limit differences
- Test citation extraction
- Verify streaming compatibility

## Provider-Specific Features

### Groq Advantages
- 10x faster inference
- Better for real-time applications
- Lower latency for code completion
- Higher rate limits

### Perplexity Advantages
- Real-time information
- Source citations
- Fact-checking capability
- Current events awareness

## Testing Approach
- Unit test configuration
- Mock API responses
- Test citation parsing
- Benchmark response times
- Compare with OpenAI baseline

## Use Case Optimization
- **Groq**: Code completion, quick iterations
- **Perplexity**: Documentation, research tasks
- **Both**: Multi-provider consensus

## Confidence Score: 9/10
OpenAI compatibility makes this straightforward. Main work is preserving provider-specific features.