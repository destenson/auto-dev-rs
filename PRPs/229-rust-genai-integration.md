# PRP: rust-genai Multi-Provider Integration

**Status**: NOT STARTED  
**Priority**: Medium (P3)  
**Estimated Time**: 3-4 hours

## Overview
Integrate rust-genai crate as an alternative multi-provider backend, providing unified access to OpenAI, Anthropic, Gemini, Ollama, Groq, DeepSeek, and more through a single API.

## Context and Background
rust-genai provides a comprehensive multi-provider interface that could replace or complement our individual provider implementations. It supports advanced features like image analysis and DeepSeekR1 reasoning.

### Research References
- rust-genai repo: https://github.com/jeremychone/rust-genai
- Documentation: https://docs.rs/genai/latest/genai/
- Examples: https://github.com/jeremychone/rust-genai/tree/main/examples

## Requirements

### Primary Goals
1. Evaluate rust-genai as alternative backend
2. Implement adapter to LLMProvider trait
3. Support all rust-genai providers
4. Enable provider-specific features
5. Compare with direct implementations

### Technical Constraints
- Must adapt to our LLMProvider interface
- Should preserve provider-specific features
- Must handle version compatibility
- Should support configuration migration

## Architectural Decisions

### Decision: Integration Strategy
**Chosen**: Adapter pattern with fallback
**Rationale**: Allows gradual migration and comparison

### Decision: Provider Selection
**Chosen**: Runtime configuration
**Rationale**: Flexibility to choose implementation

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/`:
- Create `genai_adapter.rs` - rust-genai adapter
- Create `genai_providers.rs` - Provider configurations
- Update `provider_factory.rs` - Add genai option

### Key Components
1. **GenAIAdapter** - Adapts genai to LLMProvider
2. **GenAIConfig** - Configuration management
3. **ProviderMapper** - Maps our providers to genai
4. **FeatureBridge** - Advanced features adapter

### Implementation Tasks (in order)
1. Add rust-genai to Cargo.toml
2. Create GenAIAdapter implementing LLMProvider
3. Map configuration from our format
4. Implement streaming adapter
5. Add provider-specific features
6. Create comparison benchmarks
7. Document migration path

## Provider Mapping

Our Provider -> rust-genai:
- OpenAI -> OpenAI
- Claude -> Anthropic  
- Ollama -> Ollama
- Groq -> Groq
- (NEW) Gemini -> Gemini
- (NEW) DeepSeek -> DeepSeek
- (NEW) Cohere -> Cohere

## Advanced Features

rust-genai extras:
- **Image Analysis**: OpenAI, Gemini, Anthropic
- **DeepSeekR1**: Reasoning content with stream
- **Unified Auth**: Single config format
- **Built-in Retry**: Automatic retry logic
- **Model Aliases**: Simplified model names

## Configuration Migration

From our format to genai:
```toml
[genai]
default_provider = "openai"
api_keys.openai = "${OPENAI_API_KEY}"
api_keys.anthropic = "${CLAUDE_API_KEY}"
api_keys.groq = "${GROQ_API_KEY}"

[genai.providers.openai]
model = "gpt-4"
temperature = 0.7
```

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::genai_adapter

# Test each provider through genai
cargo test --lib llm::genai::providers -- --ignored

# Benchmark against direct implementations
cargo bench --bench llm_comparison

# Feature compatibility tests
cargo test --lib llm::genai::features -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- All providers work through genai
- Performance comparable to direct
- Advanced features accessible
- Configuration migration works
- Can switch between implementations

## Dependencies Required
To add to Cargo.toml:
```toml
genai = "0.4.0-alpha.4"  # or latest stable when available
```

## Known Patterns and Conventions
- Use adapter pattern
- Preserve our abstractions
- Map errors appropriately
- Cache client instances
- Document differences

## Common Pitfalls to Avoid
- Version compatibility issues
- Feature gaps between versions
- Different error types
- Configuration complexity
- Performance overhead

## Comparison Strategy

Benchmark metrics:
- Response time
- Streaming latency
- Memory usage
- Error handling
- Feature completeness

## Migration Path

1. Add as optional backend
2. A/B test implementations
3. Migrate configuration
4. Switch providers gradually
5. Deprecate redundant code

## Advantages of rust-genai
- Single dependency for all providers
- Consistent API across providers
- Active maintenance
- Advanced features built-in
- Simpler configuration

## Disadvantages
- Additional abstraction layer
- Potential version lag
- Less control over specifics
- Dependency on external crate
- May not expose all features

## Testing Approach
- Adapter pattern tests
- Provider compatibility tests
- Feature parity tests
- Performance benchmarks
- Configuration migration tests

## Confidence Score: 7/10
Well-designed library but adds complexity. Value depends on maintenance burden vs flexibility tradeoff.