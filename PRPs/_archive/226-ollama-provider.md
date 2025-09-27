# PRP: Ollama Provider for Local and Remote Models

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 3-4 hours

## Overview
Implement Ollama provider using ollama-rs crate, enabling both local model execution and remote Ollama server connections for privacy-conscious and cost-effective AI operations.

## Context and Background
Ollama enables running LLMs locally with privacy and no per-request costs. It also supports remote deployment for shared team resources. This PRP adds Ollama as a first-class provider.

### Research References
- ollama-rs documentation: https://github.com/pepperoni21/ollama-rs
- Ollama API docs: https://github.com/ollama/ollama/blob/main/docs/api.md
- Model library: https://ollama.com/library
- OpenAI compatibility: https://markaicode.com/ollama-openai-compatibility-setup-guide/

## Requirements

### Primary Goals
1. Support local Ollama instances
2. Support remote Ollama servers
3. Enable model management (pull, list, delete)
4. Implement streaming responses
5. Support embeddings generation

### Technical Constraints
- Must implement LLMProvider trait from PRP-216
- Should auto-detect local Ollama installation
- Must handle both HTTP and HTTPS endpoints
- Should support model switching at runtime

## Architectural Decisions

### Decision: Connection Management
**Chosen**: Auto-detect with override
**Rationale**: Best UX - works locally by default, configurable for remote

### Decision: Model Management
**Chosen**: Lazy pull on first use
**Rationale**: Reduces setup friction, handles missing models gracefully

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/`:
- Create `ollama.rs` - Ollama provider implementation
- Create `ollama_models.rs` - Model management
- Update `mod.rs` - Add Ollama exports

### Key Components
1. **OllamaProvider** - Implements LLMProvider trait
2. **OllamaClient** - Wrapper around ollama-rs
3. **ModelManager** - Pull/list/delete models
4. **ConnectionDetector** - Find local/remote instances

### Implementation Tasks (in order)
1. Add ollama-rs to Cargo.toml dependencies
2. Create OllamaProvider with LLMProvider trait
3. Implement connection detection logic
4. Add streaming chat completions
5. Implement model management commands
6. Add embeddings support
7. Create integration tests

## Model Support

Priority models to test:
- **Code**: codellama, deepseek-coder, qwen2.5-coder
- **General**: llama3.2, mistral, mixtral
- **Small/Fast**: phi3, gemma2, tinyllama
- **Embeddings**: nomic-embed-text, all-minilm

## Configuration

Environment variables:
```bash
OLLAMA_HOST=http://localhost:11434  # Default
OLLAMA_MODEL=llama3.2:latest
OLLAMA_TIMEOUT=120  # seconds
```

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::ollama

# Test local connection
ollama list  # Ensure Ollama is running
cargo test --package auto-dev-core --lib llm::ollama::local -- --ignored

# Test model management
cargo test --package auto-dev-core --lib llm::ollama::models -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Connects to local Ollama automatically
- Can connect to remote servers
- Streams responses properly
- Handles model downloading
- Supports multiple concurrent requests

## Dependencies Required
To add to Cargo.toml:
```toml
ollama-rs = { git = "https://github.com/pepperoni21/ollama-rs.git", branch = "master" }
```

## Known Patterns and Conventions
- Follow provider pattern from PRP-216
- Use existing streaming patterns
- Implement timeout handling
- Cache model list

## Common Pitfalls to Avoid
- Don't assume Ollama is installed
- Handle model download time
- Remember different model formats
- Test with various model sizes
- Handle connection failures gracefully

## Testing Approach
- Mock for unit tests
- Integration tests with local Ollama
- Test model switching
- Test streaming interruption
- Load test with concurrent requests

## Unique Ollama Features
- Model management (pull/delete)
- Custom model creation (Modelfile)
- Local execution privacy
- No API keys required
- Quantization options

## Confidence Score: 8/10
Well-documented API with good Rust SDK. Main complexity in model management and connection detection.