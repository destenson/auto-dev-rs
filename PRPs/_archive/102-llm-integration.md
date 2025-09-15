# PRP: LLM Integration for Code Synthesis

## Overview
Integrate Large Language Models (LLMs) to transform specifications into working code. This is the core intelligence that understands requirements and generates appropriate implementations.

## Context and Background
The system needs to interface with LLMs (Claude, GPT-4, local models) to synthesize code from specifications. This involves prompt engineering, context management, token optimization, and response parsing. The integration must support multiple providers and handle failures gracefully.

### Research References
- Anthropic Claude API: https://docs.anthropic.com/claude/reference/
- OpenAI API: https://platform.openai.com/docs/api-reference
- Ollama for local models: https://github.com/ollama/ollama
- LangChain rust: https://github.com/Abraxas-365/langchain-rust

## Requirements

### Primary Goals
1. Support multiple LLM providers (Claude, OpenAI, Ollama)
2. Optimize prompts for code generation
3. Manage context windows efficiently
4. Handle streaming responses
5. Implement retry and fallback strategies

### Technical Constraints
- Respect API rate limits
- Optimize token usage for cost
- Handle context window limitations
- Support both cloud and local models
- Maintain conversation history

## Implementation Blueprint

### File Structure
```
src/
├── llm/
│   ├── mod.rs              # LLM module exports
│   ├── provider.rs         # Provider trait and implementations
│   ├── claude.rs           # Anthropic Claude integration
│   ├── openai.rs           # OpenAI GPT integration
│   ├── ollama.rs           # Local model integration
│   ├── prompt_builder.rs   # Prompt construction
│   ├── context_manager.rs  # Context window management
│   └── response_parser.rs  # Parse and validate LLM outputs
```

### Key Components
1. **LLMProvider Trait**: Common interface for all providers
2. **PromptBuilder**: Constructs optimized prompts
3. **ContextManager**: Manages conversation context
4. **ResponseParser**: Extracts code from responses
5. **RetryManager**: Handles failures and retries

### Provider Interface
```rust
#[async_trait]
trait LLMProvider {
    async fn generate_code(
        &self,
        spec: &Specification,
        context: &ProjectContext,
        options: &GenerationOptions,
    ) -> Result<GeneratedCode>;
    
    async fn explain_implementation(
        &self,
        code: &str,
        spec: &Specification,
    ) -> Result<Explanation>;
    
    async fn review_code(
        &self,
        code: &str,
        requirements: &[Requirement],
    ) -> Result<ReviewResult>;
}

struct GeneratedCode {
    files: Vec<GeneratedFile>,
    explanation: String,
    confidence: f32,
    tokens_used: usize,
}
```

### Implementation Tasks (in order)
1. Add LLM client dependencies
2. Create provider trait and types
3. Implement Claude provider with anthropic-sdk
4. Add OpenAI provider implementation
5. Build Ollama integration for local models
6. Create prompt templates for code generation
7. Implement context window management
8. Build response parsing with code extraction
9. Add retry logic with exponential backoff
10. Implement provider fallback chain
11. Create token usage tracking
12. Add response caching layer

## Prompt Engineering

### Code Generation Prompt Template
```
You are an expert software engineer implementing code based on specifications.

Project Context:
- Language: {language}
- Framework: {framework}
- Existing files: {file_list}

Specification:
{specification}

Requirements:
{requirements}

Generate implementation that:
1. Follows the specification exactly
2. Uses existing project patterns
3. Includes appropriate error handling
4. Has comprehensive documentation
5. Follows {language} best practices

Output format:
```{language}
// filepath: {path}
{code}
```
```

### Context Window Management
```rust
struct ContextManager {
    max_tokens: usize,
    reserved_for_response: usize,
    
    fn build_context(&self, spec: &Specification) -> String {
        // Priority order:
        // 1. Current specification
        // 2. Related code files
        // 3. Project structure
        // 4. Previous implementations
        // 5. Examples
        
        // Trim to fit context window
    }
}
```

## Provider Configurations

### Claude Configuration
```toml
[llm.claude]
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-3-opus-20240229"
max_tokens = 4096
temperature = 0.2
system_prompt = "You are a senior software engineer..."
```

### OpenAI Configuration
```toml
[llm.openai]
api_key_env = "OPENAI_API_KEY"
model = "gpt-4-turbo-preview"
max_tokens = 4096
temperature = 0.2
```

### Ollama Configuration
```toml
[llm.ollama]
host = "http://localhost:11434"
model = "codellama:13b"
max_tokens = 4096
temperature = 0.1
```

## Response Parsing

### Code Extraction
```rust
fn extract_code_blocks(response: &str) -> Vec<CodeBlock> {
    // Extract code blocks with file paths
    // Parse language hints
    // Validate syntax
    // Return structured code blocks
}

struct CodeBlock {
    filepath: Option<PathBuf>,
    language: String,
    content: String,
    line_start: usize,
}
```

## Error Handling and Retry

### Retry Strategy
```rust
struct RetryConfig {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    exponential_base: f32,
}

async fn with_retry<T>(
    operation: impl Fn() -> Future<Output = Result<T>>,
    config: &RetryConfig,
) -> Result<T> {
    // Implement exponential backoff
    // Handle rate limits specially
    // Fall back to alternative providers
}
```

## Validation Gates

```bash
# Test LLM integration
cargo test llm::tests

# Test with mock responses
cargo run -- generate --mock-llm test_spec.md

# Test real API (requires keys)
export ANTHROPIC_API_KEY="..."
cargo run -- generate --provider claude simple_spec.md

# Test local model
ollama run codellama
cargo run -- generate --provider ollama spec.md
```

## Success Criteria
- Successfully generates code from specs
- Handles API failures gracefully
- Manages context window effectively
- Supports multiple providers
- Maintains conversation context

## Known Patterns and Conventions
- Use async/await for API calls
- Implement circuit breaker pattern
- Cache responses to reduce API calls
- Use structured prompts
- Validate generated code syntax

## Common Pitfalls to Avoid
- Don't exceed rate limits
- Handle partial responses
- Avoid prompt injection
- Remember to sanitize inputs
- Don't leak API keys in logs

## Dependencies Required
- reqwest = { version = "0.11", features = ["json", "stream"] }
- async-trait = "0.1"
- anthropic-sdk = "0.1"  # If available
- async-openai = "0.16"
- ollama-rs = "0.1"
- tiktoken-rs = "0.5"  # Token counting
- backoff = "0.4"  # Retry logic

## Cost Optimization
- Cache responses aggressively
- Use smaller models for simple tasks
- Batch related requests
- Implement token usage budgets
- Fall back to local models when possible

## Security Considerations
- Store API keys securely
- Validate and sanitize all inputs
- Implement rate limiting
- Log API usage for auditing
- Support API key rotation

## Confidence Score: 8/10
LLM integration is well-understood with good library support. The main complexity is in prompt engineering and context management for optimal results.