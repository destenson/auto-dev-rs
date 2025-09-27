# PRP: LLM Prompt Templates for Code Generation

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 2-3 hours

## Overview
Create structured prompt templates that guide LLMs to generate high-quality code, ensuring consistent output format that integrates with the template system.

## Context and Background
LLMs need carefully crafted prompts to generate usable code. This PRP creates a library of prompt templates for different programming tasks, ensuring LLMs return code in a format our template system can process.

### Research References
- OpenAI prompt engineering: https://platform.openai.com/docs/guides/prompt-engineering
- Claude prompt guide: https://docs.claude.com/en/docs/prompt-engineering
- Few-shot prompting: https://www.promptingguide.ai/techniques/fewshot

## Requirements

### Primary Goals
1. Create prompt templates for common tasks
2. Ensure consistent JSON output format
3. Include language-specific examples
4. Support different complexity levels

### Technical Constraints
- Prompts must work with both GPT and Claude
- Output must be parseable JSON
- Should include error handling guidance
- Must specify import requirements

## Architectural Decisions

### Decision: Prompt Storage
**Chosen**: YAML files with template variables
**Rationale**: Human-readable, version-controllable, easy to edit

### Decision: Output Format
**Chosen**: Structured JSON with defined schema
**Rationale**: Reliable parsing, clear structure

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/synthesis/`:
- Create `prompts/` directory
- Create `prompt_manager.rs` - Prompt loading and formatting
- Add prompt templates as YAML files

Prompt structure:
```
prompts/
  common/
    system.yaml
    output_format.yaml
  rust/
    function.yaml
    struct.yaml
    trait_impl.yaml
  python/
    function.yaml
    class.yaml
    async_function.yaml
  javascript/
    function.yaml
    class.yaml
    react_component.yaml
```

### Key Components
1. **PromptManager** - Loads and formats prompts
2. **PromptTemplate** - Represents a single prompt
3. **PromptContext** - Variables for prompt
4. **OutputSchema** - Expected JSON structure

### Implementation Tasks (in order)
1. Create PromptManager to load YAML prompts
2. Define output JSON schema
3. Create system prompt for code generation
4. Add language-specific prompts
5. Implement prompt variable substitution
6. Add prompt validation
7. Create tests for prompt formatting

## Prompt Structure

Each prompt should include:
- System message (role, capabilities)
- Task description
- Input format specification
- Output format (JSON schema)
- Examples (few-shot learning)
- Constraints and requirements

## Expected Output Schema

Standard JSON output from LLM:
```
{
  "code": "main generated code",
  "imports": ["required", "imports"],
  "language": "rust|python|javascript",
  "function_name": "name",
  "parameters": [...],
  "return_type": "type",
  "documentation": "doc string",
  "tests": "optional test code",
  "dependencies": ["external", "deps"]
}
```

## Validation Gates

```bash
# Build and test first
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib synthesis::prompt_manager
cargo test --package auto-dev-core --lib synthesis::prompts

# Verify prompt files exist
find auto-dev-core/src/synthesis/prompts -name "*.yaml" | wc -l

# Then format and lint
cargo fmt --check --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Prompts load from YAML successfully
- Variable substitution works
- Generated prompts are valid
- Output schema is consistent
- Works with both GPT and Claude

## Dependencies Required
Already available: serde_yaml for YAML parsing

## Known Patterns and Conventions
- Use clear, specific instructions
- Include examples for complex tasks
- Specify edge case handling
- Request structured output
- Use consistent terminology

## Common Pitfalls to Avoid
- Don't make prompts too long
- Avoid ambiguous instructions
- Don't forget error handling guidance
- Include type information
- Test with both LLM providers

## Prompt Guidelines
- Be specific about requirements
- Use consistent formatting
- Include "think step by step" for complex logic
- Specify import requirements explicitly
- Request documentation/comments

## Testing Approach
- Unit test prompt loading
- Test variable substitution
- Validate output schema
- Mock LLM responses for testing
- End-to-end test with real LLMs

## Example Prompt Elements
- Role: "You are an expert <language> programmer"
- Task: "Generate a <type> that <description>"
- Format: "Return JSON with the following structure"
- Constraints: "Follow <language> best practices"
- Examples: Provide 1-2 examples

## Confidence Score: 9/10
Well-defined requirements, straightforward implementation. Success depends on prompt quality.