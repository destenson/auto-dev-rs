# PRP: Local Model Integration for Code Generation

**Status**: NOT STARTED  
**Priority**: Medium (P2)  
**Estimated Time**: 2-3 hours

## Overview
Integrate the local Qwen 2.5 Coder model (0.5B parameters) for enhanced code generation. This provides intelligent code completion and generation without requiring external API calls.

## Context and Background
The project includes a quantized Qwen model at `models/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf`. This PRP integrates it for code generation using the candle framework or llama.cpp bindings.

### Research References
- Candle framework: https://github.com/huggingface/candle
- llama.cpp rust bindings: https://docs.rs/llama-cpp-rs/latest/
- GGUF format: https://github.com/ggerganov/ggml/blob/master/docs/gguf.md
- Qwen models: https://github.com/QwenLM/Qwen2.5-Coder

## Requirements

### Primary Goals
1. Load and run local GGUF model
2. Generate code completions from prompts
3. Provide streaming generation support
4. Work entirely offline

### Technical Constraints
- Model must load from local file
- Memory efficient (0.5B model is small)
- No external API dependencies
- Graceful fallback if model unavailable

## Architectural Decisions

### Decision: Inference Backend
**Chosen**: llama-cpp-rs for GGUF support
**Rationale**: Direct GGUF support, proven performance, active maintenance

### Decision: Integration Pattern
**Chosen**: Provider trait implementation
**Rationale**: Fits existing LLM provider architecture

## Implementation Blueprint

### File Structure
Update in `auto-dev-core/src/llm/`:
- Create `local_qwen.rs` - Qwen model implementation
- Update `provider.rs` - Add local provider variant
- Create `gguf_loader.rs` - GGUF model loading
- Update `mod.rs` - Export new provider

### Key Components
1. **QwenLocalProvider** - LLM provider implementation
2. **GgufModelLoader** - Handles GGUF loading
3. **GenerationConfig** - Temperature, max_tokens, etc.
4. **TokenStreamer** - Streaming generation support

### Implementation Tasks (in order)
1. Add llama-cpp-rs dependency to Cargo.toml
2. Create GGUF model loader
3. Implement LLM provider trait for Qwen
4. Add generation configuration options
5. Test model loading and generation
6. Add streaming token support

## Validation Gates

```bash
# Build with model support
cargo build --package auto-dev-core --features local-model

# Run tests
cargo test --package auto-dev-core llm::local_qwen

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Test model loading
auto-dev generate "write hello world" --use-local-model

# Verify model inference
cargo run --example test-local-model
```

## Success Criteria
- Model loads from GGUF file successfully
- Generates coherent code completions
- Supports streaming generation
- Falls back gracefully if model missing
- Memory usage under 2GB for inference
- Generation speed > 10 tokens/sec on CPU

## Dependencies Required
```toml
llama-cpp-rs = "0.1"  # or latest version
# OR
candle-core = "0.3"
candle-transformers = "0.3"
```

## Known Patterns and Conventions
- Implement existing LlmProvider trait
- Use Result for error handling
- Follow async pattern for generation
- Cache loaded model globally

## Common Pitfalls to Avoid
- Model file path hardcoding
- Not checking model file exists
- Loading model multiple times
- Blocking on generation
- Not handling OOM errors

## Testing Approach
- Mock model for unit tests
- Integration test with real model
- Test streaming generation
- Benchmark token generation speed
- Test memory usage

## Confidence Score: 7/10
GGUF loading complexity uncertain, but fallback patterns clear.