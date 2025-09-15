# PRP: Code Generation for Rust

## Overview
Implement the first code generation feature focusing on Rust, creating boilerplate for common patterns like structs, enums, traits, and implementations using the template engine.

## Context and Background
Code generation reduces boilerplate and enforces consistency. Starting with Rust allows dogfooding and provides immediate value for the project itself. This builds on the template engine foundation.

### Research References
- Rust syntax and idioms: https://doc.rust-lang.org/book/
- Common Rust patterns: https://rust-unofficial.github.io/patterns/
- Builder pattern: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
- Derive macros patterns: https://doc.rust-lang.org/rust-by-example/trait/derive.html

## Requirements

### Primary Goals
1. Generate Rust structs with builder pattern
2. Create enum definitions with methods
3. Generate trait definitions and implementations
4. Create test boilerplate
5. Support module structure generation

### Technical Constraints
- Generated code must compile without warnings
- Follow Rust idioms and conventions
- Support common derive macros
- Integrate with rustfmt for formatting
- Generate documentation comments

## Implementation Blueprint

### File Structure
```
src/
├── generator/
│   ├── mod.rs           # Generator module exports
│   ├── rust/
│   │   ├── mod.rs       # Rust generator main
│   │   ├── structs.rs   # Struct generation
│   │   ├── enums.rs     # Enum generation
│   │   ├── traits.rs    # Trait generation
│   │   ├── tests.rs     # Test generation
│   │   └── modules.rs   # Module structure

.auto-dev/
├── templates/
│   └── code/
│       └── rust/
│           ├── struct.hbs
│           ├── struct_builder.hbs
│           ├── enum.hbs
│           ├── trait.hbs
│           ├── impl.hbs
│           ├── test.hbs
│           └── module.hbs
```

### Key Components
1. **RustGenerator**: Main generator coordinator
2. **StructGenerator**: Generates struct definitions
3. **EnumGenerator**: Creates enum types
4. **TraitGenerator**: Trait definitions
5. **TestGenerator**: Test scaffolding

### Generation Patterns

#### Struct with Builder
```rust
// Input specification
{
  "name": "User",
  "fields": [
    {"name": "id", "type": "u64", "optional": false},
    {"name": "name", "type": "String", "optional": false},
    {"name": "email", "type": "Option<String>", "optional": true}
  ],
  "derives": ["Debug", "Clone", "Serialize", "Deserialize"],
  "builder": true
}

// Generated output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: Option<String>,
}

impl User {
    pub fn builder() -> UserBuilder { ... }
}

pub struct UserBuilder { ... }
impl UserBuilder { ... }
```

### Implementation Tasks (in order)
1. Create src/generator/rust module structure
2. Define generation specification types
3. Implement struct generation with basic fields
4. Add builder pattern generation
5. Create enum generation with variants
6. Implement trait definition generation
7. Add impl block generation
8. Create test boilerplate generator
9. Implement module structure generator
10. Add rustfmt integration
11. Create validation for generated code
12. Add interactive generation mode

## Template Examples

### Struct Template (struct.hbs)
```handlebars
{{#each derives}}
#[derive({{this}})]
{{/each}}
{{#if visibility}}{{visibility}} {{/if}}struct {{name}}{{#if generics}}<{{generics}}>{{/if}} {
    {{#each fields}}
    {{#if this.doc}}/// {{this.doc}}{{/if}}
    {{#if this.visibility}}{{this.visibility}} {{/if}}{{snakeCase this.name}}: {{this.field_type}},
    {{/each}}
}

{{#if builder}}
impl{{#if generics}}<{{generics}}>{{/if}} {{name}}{{#if generics}}<{{generics}}>{{/if}} {
    pub fn builder() -> {{name}}Builder{{#if generics}}<{{generics}}>{{/if}} {
        {{name}}Builder::default()
    }
}
{{/if}}
```

### Test Template (test.hbs)
```handlebars
#[cfg(test)]
mod tests {
    use super::*;

    {{#each test_cases}}
    #[test]
    fn {{snakeCase this.name}}() {
        // Arrange
        {{#if this.setup}}{{this.setup}}{{/if}}

        // Act
        {{#if this.action}}{{this.action}}{{/if}}

        // Assert
        {{#if this.assertion}}{{this.assertion}}{{else}}todo!("Implement test");{{/if}}
    }
    {{/each}}
}
```

## CLI Interface

```bash
# Generate a struct
auto-dev generate rust struct User \
  --field id:u64 \
  --field name:String \
  --field "email:Option<String>" \
  --derive Debug,Clone \
  --builder

# Generate from specification file
auto-dev generate rust --spec user.yaml

# Interactive mode
auto-dev generate rust --interactive
```

## Validation Gates

```bash
# Build and test
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Test generation
cargo test generator::rust::tests

# Generate sample code
cargo run -- generate rust struct TestStruct --field name:String

# Verify generated code compiles
echo "use serde::{Serialize, Deserialize};" > test_gen.rs
cargo run -- generate rust struct User --derive Serialize,Deserialize >> test_gen.rs
rustc --edition 2021 test_gen.rs
```

## Success Criteria
- Generated code compiles without errors
- Follows Rust idioms and style
- Builder pattern works correctly
- Documentation is included
- Templates are customizable

## Known Patterns and Conventions
- Use snake_case for field names
- PascalCase for type names
- Include derive macros appropriately
- Generate documentation comments
- Follow official Rust style guide

## Common Pitfalls to Avoid
- Don't generate invalid syntax
- Handle generic types carefully
- Remember lifetime annotations
- Avoid name collisions
- Don't forget visibility modifiers

## Dependencies Required
- syn = "2.0"  # For parsing Rust code
- quote = "1.0"  # For generating Rust code
- proc-macro2 = "1.0"
- prettyplease = "0.2"  # For formatting

## Future Enhancements
- Macro generation
- Async trait support
- Generic constraints
- Custom derive generation
- Integration with rust-analyzer

## Confidence Score: 8/10
Clear patterns with good library support. Rust code generation is well-understood with established tools and patterns.