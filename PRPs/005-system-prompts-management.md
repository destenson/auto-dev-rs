# PRP: System Prompts and Procedures Management

## Overview
Create a filesystem-based system for managing prompts, task descriptions, and procedures with user-configurable overrides and a rich set of defaults. This enables customization of AI interactions and standardized workflows.

## Context and Background
Users need to customize system behavior through prompts and procedures while having sensible defaults. The system should support prompt templates for different contexts (code generation, testing, documentation) and allow users to define their own.

### Research References
- Handlebars templating: https://docs.rs/handlebars/latest/handlebars/
- Prompt engineering best practices: https://platform.openai.com/docs/guides/prompt-engineering
- Template organization patterns from existing tools

## Requirements

### Primary Goals
1. Create prompt template system with variables
2. Implement procedure definitions for workflows
3. Provide rich set of default prompts
4. Enable user customization and overrides
5. Support context-aware prompt selection

### Technical Constraints
- Templates must be filesystem-based for easy editing
- Support variable substitution
- Allow inheritance and composition
- Maintain version compatibility
- Keep prompts human-readable

## Implementation Blueprint

### File Structure
```
src/
├── prompts/
│   ├── mod.rs           # Prompts module exports
│   ├── manager.rs       # PromptManager implementation
│   ├── loader.rs        # Template loading and parsing
│   ├── context.rs       # Context building for templates
│   └── defaults.rs      # Default prompt initialization

.auto-dev/
├── prompts/
│   ├── system/          # System-level prompts
│   │   ├── code-generation.md
│   │   ├── testing.md
│   │   └── documentation.md
│   ├── procedures/      # Workflow procedures
│   │   ├── feature-implementation.md
│   │   ├── bug-fix.md
│   │   └── refactoring.md
│   ├── custom/          # User-defined prompts
│   └── templates.toml   # Template configuration
```

### Key Components
1. **PromptManager**: Loads and manages prompt templates
2. **Template Engine**: Variable substitution and rendering
3. **ProcedureRunner**: Executes multi-step procedures
4. **ContextBuilder**: Gathers context for template variables
5. **PromptSelector**: Chooses appropriate prompt based on context

### Implementation Tasks (in order)
1. Add handlebars or tera to Cargo.toml
2. Create src/prompts module structure
3. Define PromptTemplate struct with metadata
4. Implement template loading from filesystem
5. Create default prompt templates
6. Build variable substitution system
7. Implement procedure parsing and validation
8. Add context detection for automatic prompt selection
9. Create prompt override mechanism
10. Implement prompt versioning and migration
11. Add prompt testing and validation commands
12. Create prompt documentation generator

## Default Prompt Categories

### System Prompts
```markdown
# code-generation.md
Generate {{language}} code for {{feature}}.
Requirements:
- Follow {{style_guide}} conventions
- Include error handling
- Add appropriate tests
Context: {{project_context}}
```

### Procedures
```markdown
# feature-implementation.md
1. Analyze requirements
2. Design solution architecture
3. Implement core functionality
4. Add unit tests
5. Update documentation
6. Run integration tests
```

### Task Descriptions
```markdown
# task-descriptions.toml
[tasks.refactor]
description = "Refactor code for better maintainability"
steps = ["identify-code-smells", "plan-refactoring", "implement-changes"]
```

## Variable System

### Common Variables
- `{{project_name}}` - Current project name
- `{{language}}` - Primary programming language
- `{{feature}}` - Feature being implemented
- `{{context}}` - Gathered project context
- `{{user_instructions}}` - Custom user requirements

### Context Sources
1. Project configuration
2. Git repository information
3. File system structure
4. Previous task history
5. User preferences

## Validation Gates

```bash
# Build and lint
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Test prompt loading
cargo test prompts::tests

# Verify default prompts exist
cargo run -- prompts validate

# Test variable substitution
cargo run -- prompts render code-generation --var language=rust
```

## Success Criteria
- All default prompts load successfully
- Variable substitution works correctly
- User overrides take precedence
- Procedures execute in order
- Invalid templates produce clear errors

## Known Patterns and Conventions
- Use Markdown for prompt files
- TOML for configuration and metadata
- Handlebars {{variable}} syntax
- Semantic versioning for prompts
- Keep prompts focused and modular

## Common Pitfalls to Avoid
- Don't hardcode prompt paths
- Handle missing variables gracefully
- Avoid complex logic in templates
- Remember to escape special characters
- Don't mix concerns in single prompts

## Dependencies Required
- handlebars = "5.0"
- walkdir = "2.0"
- regex = "1.0"
- once_cell = "1.0"

## Extensibility Considerations
- Plugin system can add custom prompts
- Support for remote prompt repositories
- Prompt sharing and marketplace
- Version control integration
- A/B testing for prompt effectiveness

## Example Usage
```rust
// Load and render a prompt
let prompt = prompt_manager.load("code-generation")?;
let rendered = prompt.render(context! {
    language: "rust",
    feature: "user authentication",
    style_guide: "project-conventions.md"
})?;

// Execute a procedure
let procedure = prompt_manager.load_procedure("feature-implementation")?;
procedure.execute(context)?;
```

## Confidence Score: 9/10
Well-defined scope with clear patterns from existing tools. Template systems are mature and well-understood.