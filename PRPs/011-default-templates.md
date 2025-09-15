# PRP: Default Templates and Prompts Creation

## Overview
Create a comprehensive set of default templates, prompts, and procedures that ship with auto-dev, providing immediate value and serving as examples for customization.

## Context and Background
Users need working templates out of the box. These defaults demonstrate best practices and provide patterns for creating custom templates. They should cover common development tasks across multiple languages.

### Research References
- GitHub template repositories for patterns
- Yeoman generators: https://yeoman.io/generators/
- Create-react-app templates: https://github.com/facebook/create-react-app
- Cargo generate templates: https://github.com/cargo-generate/cargo-generate

## Requirements

### Primary Goals
1. Create code generation templates for 3+ languages
2. Develop project initialization templates
3. Build documentation templates
4. Create system prompts for AI interactions
5. Develop workflow procedures

### Technical Constraints
- Templates must be immediately usable
- Support variable substitution
- Include documentation in templates
- Follow language-specific conventions
- Maintain consistency across languages

## Implementation Blueprint

### File Structure
```
.auto-dev/
├── templates/
│   ├── init/              # Project initialization
│   │   ├── rust-cli/
│   │   ├── rust-lib/
│   │   ├── python-app/
│   │   └── node-app/
│   ├── code/              # Code generation
│   │   ├── rust/
│   │   ├── python/
│   │   └── javascript/
│   ├── docs/              # Documentation
│   │   ├── README.hbs
│   │   ├── API.hbs
│   │   └── CONTRIBUTING.hbs
│   └── tests/             # Test templates
│       ├── unit-test.hbs
│       └── integration-test.hbs

├── prompts/
│   ├── system/
│   │   ├── code-review.md
│   │   ├── bug-fix.md
│   │   ├── refactor.md
│   │   └── explain.md
│   └── procedures/
│       ├── feature-dev.md
│       ├── debugging.md
│       └── deployment.md
```

### Template Categories

#### Project Initialization Templates
- Rust CLI application with clap
- Rust library with examples
- Python application with poetry
- Node.js app with TypeScript
- Generic Makefile project

#### Code Generation Templates
- CRUD operations
- REST API endpoints
- Database models
- Configuration structures
- Error handling patterns

#### Documentation Templates
- README with badges
- API documentation
- Contributing guidelines
- Changelog format
- License files

### Implementation Tasks (in order)
1. Create directory structure for templates
2. Develop Rust project templates (CLI and library)
3. Create Python project templates
4. Build JavaScript/Node templates
5. Implement code generation templates per language
6. Create documentation templates
7. Develop test templates
8. Write system prompts for AI tasks
9. Create workflow procedures
10. Add template metadata files
11. Implement template installation command
12. Create template documentation

## Sample Templates

### Rust CLI Project (init/rust-cli/Cargo.toml.hbs)
```handlebars
[package]
name = "{{kebabCase projectName}}"
version = "0.1.0"
edition = "2021"
authors = ["{{author}} <{{email}}>"]
description = "{{description}}"
license = "{{license}}"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
{{#each dependencies}}
{{this.name}} = "{{this.version}}"
{{/each}}

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
```

### Python Class Template (code/python/class.hbs)
```handlebars
"""{{description}}"""

from typing import Optional, List, Dict, Any
{{#each imports}}
{{this}}
{{/each}}


class {{pascalCase className}}:
    """{{classDoc}}"""

    def __init__(self{{#each attributes}}, {{snakeCase this.name}}: {{this.type}}{{#if this.default}} = {{this.default}}{{/if}}{{/each}}):
        """Initialize {{pascalCase className}}.
        
        Args:
            {{#each attributes}}
            {{snakeCase this.name}}: {{this.description}}
            {{/each}}
        """
        {{#each attributes}}
        self.{{snakeCase this.name}} = {{snakeCase this.name}}
        {{/each}}

    {{#each methods}}
    def {{snakeCase this.name}}(self{{#each this.params}}, {{this.name}}: {{this.type}}{{/each}}) -> {{this.returnType}}:
        """{{this.description}}"""
        {{#if this.implementation}}
        {{this.implementation}}
        {{else}}
        raise NotImplementedError("Method {{this.name}} not yet implemented")
        {{/if}}
    {{/each}}
```

### System Prompt (prompts/system/code-review.md)
```markdown
# Code Review Prompt

You are reviewing code for quality, security, and best practices.

## Review Criteria
1. **Correctness**: Does the code do what it claims?
2. **Security**: Are there any security vulnerabilities?
3. **Performance**: Are there obvious performance issues?
4. **Maintainability**: Is the code clean and maintainable?
5. **Testing**: Is the code properly tested?

## Context
- Project: {{projectName}}
- Language: {{language}}
- Component: {{component}}
- Author: {{author}}

## Code to Review
{{code}}

## Focus Areas
{{#each focusAreas}}
- {{this}}
{{/each}}

Provide specific, actionable feedback with examples.
```

### Workflow Procedure (prompts/procedures/feature-dev.md)
```markdown
# Feature Development Procedure

## Phase 1: Planning
1. Review requirements and acceptance criteria
2. Identify affected components
3. Design solution approach
4. Estimate effort and risks

## Phase 2: Implementation
1. Create feature branch from {{baseBranch}}
2. Write failing tests for new functionality
3. Implement feature incrementally
4. Refactor for clarity and performance

## Phase 3: Testing
1. Run unit tests locally
2. Add integration tests
3. Manual testing of edge cases
4. Performance testing if applicable

## Phase 4: Documentation
1. Update API documentation
2. Add inline code comments
3. Update README if needed
4. Create usage examples

## Phase 5: Review
1. Self-review changes
2. Run linters and formatters
3. Create pull request
4. Address review feedback

## Variables
- Feature Name: {{featureName}}
- Target Branch: {{targetBranch}}
- Reviewer: {{reviewer}}
```

## Validation Gates

```bash
# Verify template structure
cargo run -- template validate --all

# Test template rendering
cargo run -- template test

# Create sample project
cargo run -- init rust-cli my-test-project
cd my-test-project && cargo build

# Test code generation
cargo run -- generate rust struct User
```

## Success Criteria
- All templates render without errors
- Generated projects compile/run
- Templates follow best practices
- Documentation is comprehensive
- Examples work out of the box

## Known Patterns and Conventions
- Use language-specific naming conventions
- Include helpful comments in templates
- Provide sensible defaults
- Make templates composable
- Version templates appropriately

## Common Pitfalls to Avoid
- Don't hardcode user-specific values
- Avoid complex logic in templates
- Don't assume environment setup
- Remember cross-platform compatibility
- Keep templates focused

## Dependencies Required
None additional - uses existing template engine

## Maintenance Considerations
- Regular updates for dependency versions
- Track language evolution
- Community feedback integration
- Security update monitoring
- Template testing automation

## Confidence Score: 9/10
Clear requirements with many reference implementations. Template creation is straightforward with established patterns.