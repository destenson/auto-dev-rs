# PRP: Claude Non-Interactive Output Parsing

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 3 hours

## Overview
Parse and structure the output from Claude Code CLI when run in non-interactive mode (--print flag) with support for different output formats (text, json, stream-json).

## Context and Background
Claude's --print mode outputs responses in three formats: plain text (default), single JSON object, or streaming JSON. We need to parse these formats and convert them to our internal LLM response types.

### Research References
- Output format docs: https://docs.anthropic.com/en/docs/claude-code/cli-reference
- Existing LLM types in `auto-dev-core/src/llm/types.rs`
- JSON parsing patterns in `auto-dev-core/src/llm/openrouter.rs` lines 427-444

## Requirements

### Primary Goals
1. Parse text format output (default)
2. Parse JSON format output
3. Parse stream-json format (line-delimited JSON)
4. Extract code blocks from responses
5. Convert to internal LLM types

### Technical Constraints
- Must handle partial/incomplete responses
- Should detect format from output content
- Must preserve formatting in code blocks
- Handle error messages from Claude

## Architectural Decisions

### Decision: Format Detection
**Chosen**: Content-based detection with hints
**Rationale**: More robust than relying on flags alone

### Decision: Streaming Support
**Chosen**: Buffer and parse complete response for now
**Rationale**: Simpler initial implementation

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/claude_code/`:
- Create `parser.rs` - Output parsing logic
- Create `formats.rs` - Format-specific parsers
- Update `mod.rs` - Export parser types

### Key Components
1. **OutputParser** - Main parser interface
2. **TextParser** - Parse plain text output
3. **JsonParser** - Parse JSON format
4. **StreamJsonParser** - Parse stream-json
5. **CodeExtractor** - Extract code blocks

### Implementation Tasks (in order)
1. Define output format enum and detection
2. Implement plain text parser
3. Implement JSON format parser
4. Add code block extraction logic
5. Map to internal CompletionResponse type
6. Handle error responses
7. Add format auto-detection
8. Write tests for each format

## Output Formats

### Text Format (default)
Plain text response, may contain markdown code blocks:
```
Here's the solution:

\`\`\`rust
fn main() {
    println!("Hello");
}
\`\`\`
```

### JSON Format
Single JSON object with response:
```json
{
  "content": "Here's the solution...",
  "model": "claude-3-5-sonnet",
  "usage": {
    "input_tokens": 100,
    "output_tokens": 200
  }
}
```

### Stream-JSON Format
Line-delimited JSON events (for future streaming support):
```json
{"type": "content_block_start", "index": 0}
{"type": "content_block_delta", "delta": {"text": "Here's"}}
{"type": "content_block_stop", "index": 0}
```

## Code Block Extraction

Extract from markdown code blocks:
- Detect language from \`\`\`lang
- Preserve indentation
- Handle nested backticks
- Support multiple code blocks
- Map to GeneratedFile structures

## Error Handling

Parse Claude error responses:
- Rate limit errors
- Authentication errors
- Model overload errors
- Network errors
- Invalid prompt errors

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::claude_code::parser

# Test each format
cargo test --package auto-dev-core --lib llm::claude_code::parser::text
cargo test --package auto-dev-core --lib llm::claude_code::parser::json

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Parses all three output formats correctly
- Extracts code blocks with language detection
- Maps to internal types without data loss
- Handles malformed output gracefully
- Provides clear error messages

## Dependencies Required
Already in Cargo.toml:
- serde_json for JSON parsing
- regex for pattern matching
- anyhow for error handling

## Known Patterns and Conventions
- Follow code extraction from `openrouter.rs` lines 699-757
- Use serde for JSON deserialization
- Return Result types consistently
- Log parsing failures with tracing

## Common Pitfalls to Avoid
- Don't assume output is well-formed
- Handle incomplete JSON gracefully
- Preserve original formatting in code
- Test with multi-language code blocks
- Handle Unicode correctly

## Testing Approach
- Unit tests with sample outputs
- Test malformed JSON
- Test incomplete responses
- Test various code block formats
- Test error response parsing

## Response Mapping

Map to internal types:
- Claude content → CompletionResponse
- Usage data → Usage struct
- Model info → model_used field
- Timestamps → created field

## Confidence Score: 8/10
Clear requirements but needs careful handling of various output formats.