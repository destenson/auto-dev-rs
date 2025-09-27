# PRP: OpenRouter Gateway Integration

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 3-4 hours

## Overview
Implement OpenRouter as a unified gateway to 400+ LLMs from 60+ providers through a single API, providing automatic failover, cost optimization, and simplified multi-model access.

## Context and Background
OpenRouter is a game-changing service that aggregates all major AI providers (OpenAI, Anthropic, Google, Meta, etc.) through one API. It handles failover, rate limiting, and billing aggregation while maintaining provider-native pricing.

### Research References
- OpenRouter docs: https://openrouter.ai/docs
- API reference: https://openrouter.ai/docs/api-reference
- openrouter-rs crate: https://crates.io/crates/openrouter-rs
- openrouter_api crate: https://lib.rs/crates/openrouter_api

## Requirements

### Primary Goals
1. Integrate OpenRouter as primary provider gateway
2. Access 400+ models through single API
3. Leverage automatic failover capabilities
4. Implement cost tracking and optimization
5. Support both BYOK and credit modes

### Technical Constraints
- OpenAI-compatible API format
- Must handle provider-specific features
- Should track costs per provider
- Must support streaming responses

## Architectural Decisions

### Decision: Integration Strategy
**Chosen**: Primary gateway with direct fallback
**Rationale**: Maximizes model access while maintaining direct provider option

### Decision: SDK Selection
**Chosen**: openrouter-rs for type safety
**Rationale**: Better documentation, recent updates, reasoning token support

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/`:
- Create `openrouter.rs` - OpenRouter provider
- Create `openrouter_models.rs` - Model catalog
- Create `cost_tracker.rs` - Usage and cost tracking
- Update `provider_factory.rs` - Add OpenRouter

### Key Components
1. **OpenRouterProvider** - Main provider implementation
2. **ModelCatalog** - 400+ model registry
3. **CostOptimizer** - Select cheapest suitable model
4. **FailoverManager** - Handle automatic failover
5. **UsageTracker** - Track costs across providers

### Implementation Tasks (in order)
1. Add openrouter-rs to Cargo.toml
2. Create OpenRouterProvider with LLMProvider trait
3. Implement model catalog and search
4. Add cost optimization logic
5. Implement usage tracking
6. Create provider-specific routing
7. Add comprehensive tests
8. Document model selection strategy

## Model Access

Categories available:
- **OpenAI**: GPT-4, GPT-3.5, o1-preview
- **Anthropic**: Claude 3 Opus/Sonnet/Haiku
- **Google**: Gemini Pro, PaLM
- **Meta**: Llama 3.2, CodeLlama
- **Mistral**: Mistral, Mixtral
- **Free Tier**: Zephyr, Toppy, DeepSeek R1
- **Specialized**: Command-R, Qwen, Yi

## Cost Optimization

Strategies:
```toml
[openrouter.optimization]
mode = "balanced"  # cheapest|balanced|quality
fallback_chain = ["anthropic/claude-3-haiku", "meta/llama-3.2", "mistral/mistral-7b"]
max_cost_per_request = 0.10
prefer_free_tier = false
```

## Configuration

```toml
[openrouter]
api_key = "${OPENROUTER_API_KEY}"
base_url = "https://openrouter.ai/api/v1"
mode = "credits"  # credits|byok
default_model = "anthropic/claude-3-sonnet"
enable_fallback = true
track_usage = true

[openrouter.routing]
code_models = ["anthropic/claude-3-opus", "openai/gpt-4"]
chat_models = ["anthropic/claude-3-haiku", "meta/llama-3.2"]
reasoning_models = ["openai/o1-preview", "deepseek/deepseek-r1"]
```

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::openrouter

# Test model catalog
cargo test --lib llm::openrouter::models

# Integration test with real API
OPENROUTER_API_KEY=... cargo test openrouter_integration -- --ignored

# Test failover behavior
cargo test --lib llm::openrouter::failover -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Access to 400+ models confirmed
- Automatic failover works
- Cost tracking accurate
- Streaming responses function
- Provider-specific features preserved

## Dependencies Required
To add to Cargo.toml:
```toml
openrouter-rs = "0.2"  # or latest version
# Alternative: openrouter_api = "0.1"
```

## Known Patterns and Conventions
- Use OpenAI-compatible format
- Implement cost tracking hooks
- Cache model catalog
- Log provider selection
- Track failover events

## Common Pitfalls to Avoid
- Don't hardcode model names
- Handle provider outages gracefully
- Track BYOK vs credits usage
- Monitor rate limits per provider
- Test with various model types

## Unique OpenRouter Features

1. **Automatic Failover**: Falls back when providers fail
2. **Cost Aggregation**: Single billing for all providers
3. **Provider Health**: Real-time provider status
4. **Rate Limit Pooling**: Combines limits across providers
5. **Model Discovery**: New models appear automatically
6. **BYOK Support**: Use your own provider keys

## Model Selection Logic

Choose models based on:
- Task type (code, chat, reasoning)
- Cost constraints
- Speed requirements
- Quality needs
- Availability status

## Cost Tracking

Track per request:
- Model used
- Token counts
- Cost in credits/USD
- Provider selected
- Failover events

## Testing Approach
- Unit test provider adapter
- Test model catalog updates
- Mock failover scenarios
- Test cost calculations
- Integration with real API

## Migration Benefits

Moving to OpenRouter provides:
- **Simplified Integration**: One API for all
- **Better Uptime**: Automatic failover
- **Cost Control**: Unified billing
- **Model Access**: 400+ models instantly
- **Future Proof**: New models automatically

## Performance Considerations
- ~40ms added latency (edge locations)
- Parallel model queries possible
- Caching reduces catalog lookups
- Streaming maintains responsiveness

## Confidence Score: 9/10
Clear value proposition with mature SDKs available. OpenRouter significantly simplifies multi-provider management.