# PRP: Code Synthesis Template System

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 3-4 hours

## Overview
Implement a template-based code generation system using Tera or Handlebars that transforms LLM responses into properly formatted code for different languages.

## Context and Background
Currently, the code generator returns `"// TODO: Implement {task}"`. This PRP creates the template system that will format LLM-generated code into proper file structures with imports, classes, functions, and tests.

### Research References
- Tera documentation: https://keats.github.io/tera/docs/
- Handlebars-rust: https://github.com/sunng87/handlebars-rust
- Template benchmarks: https://github.com/rosetta-rs/template-benchmarks-rs

## Requirements

### Primary Goals
1. Create template system for code generation
2. Support Rust, Python, JavaScript/TypeScript
3. Handle imports, formatting, documentation
4. Generate complete, runnable files

### Technical Constraints
- Templates must be maintainable and extensible
- Should integrate with language-specific formatters
- Must handle different code styles/conventions
- Templates loaded at runtime for flexibility

## Architectural Decisions

### Decision: Template Engine
**Chosen**: Tera
**Rationale**: Better for complex logic, runtime loading, good performance

### Decision: Template Organization
**Chosen**: Language-specific template directories
**Rationale**: Clear organization, easy to extend

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/synthesis/`:
- Create `templates/` directory structure
- Update `pipeline/generator.rs` - Use templates
- Create `template_engine.rs` - Template management
- Add templates for each language

Template structure:
```
templates/
  rust/
    function.tera
    struct.tera
    module.tera
    test.tera
  python/
    function.tera
    class.tera
    module.tera
    test.tera
  javascript/
    function.tera
    class.tera
    module.tera
    test.tera
```

### Key Components
1. **TemplateEngine** - Manages Tera instance and templates
2. **CodeTemplate** - Represents a single template
3. **TemplateContext** - Data passed to templates
4. **LanguageTemplates** - Per-language template sets

### Implementation Tasks (in order)
1. Add tera to Cargo.toml dependencies
2. Create TemplateEngine wrapper around Tera
3. Create base templates for each language
4. Update generator.rs to use templates
5. Add template validation on load
6. Implement template caching
7. Write tests for each language template

## Template Examples (Structure, not code)

Each template should handle:
- File header (comments, license)
- Import statements
- Main code structure
- Documentation comments
- Error handling patterns
- Test structure (if applicable)

## Validation Gates

```bash
# Build and test first
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib synthesis::template_engine
cargo test --package auto-dev-core --lib synthesis::pipeline::generator

# Verify templates exist
ls -la auto-dev-core/src/synthesis/templates/*/*.tera

# Then format and lint
cargo fmt --check --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Templates load successfully
- Generated code is syntactically valid
- Each language has basic templates
- Template errors are handled gracefully
- Generated code follows conventions

## Dependencies Required
To add to Cargo.toml:
```toml
tera = "1.20"  # or latest version
```

## Known Patterns and Conventions
- Store templates as separate files, not strings
- Use template inheritance for common patterns
- Follow each language's style guide
- Include helpful comments in generated code

## Common Pitfalls to Avoid
- Don't hardcode paths to templates
- Remember to escape special characters
- Handle missing template variables
- Don't over-complicate templates
- Test with various input types

## Template Variables
Standard context for all templates:
- `function_name` - Name of function/class
- `parameters` - List of parameters
- `return_type` - Return type (if applicable)
- `body` - Main code body
- `imports` - Required imports
- `documentation` - Doc comments
- `metadata` - Additional language-specific data

## Testing Approach
- Unit test each template type
- Test with minimal and complex inputs
- Verify output compiles/runs
- Test error cases (missing variables)
- Integration test with LLM output

## File Organization
Keep templates in `auto-dev-core/src/synthesis/templates/` as `.tera` files for easy editing and version control.

## Confidence Score: 9/10
Clear requirements, good template engines available. Main work is creating quality templates.