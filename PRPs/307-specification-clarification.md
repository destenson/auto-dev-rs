# PRP: Specification Clarification System

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 2-3 hours

## Overview
Implement a system that detects underspecified instructions and generates a clarification file with questions, decisions, and options for the user to answer before proceeding with implementation. This prevents incorrect assumptions and ensures generated code meets user expectations.

## Context and Background
Instructions like "create a REST API in Rust" are ambiguous - what framework? What endpoints? Database? Authentication? Rather than guessing, the system should generate a structured questionnaire and wait for user input.

### Research References
- Interactive CLI patterns: https://docs.rs/dialoguer/latest/dialoguer/
- Config file formats: YAML/TOML for questionnaires
- Similar to npm init's interactive mode

## Requirements

### Primary Goals
1. Detect when specifications lack critical details
2. Generate context-appropriate clarification questions
3. Create structured questionnaire file
4. Parse user responses and update specifications
5. Support both interactive and file-based responses

### Technical Constraints
- No LLM for question generation (use templates)
- Must work in non-interactive mode
- Questions should be language/framework specific
- Support partial answers (iterative clarification)

## Architectural Decisions

### Decision: Question Storage Format
**Chosen**: YAML with defaults and options
**Rationale**: Human-readable, supports complex structures, easy to edit

### Decision: Detection Strategy
**Chosen**: Rule-based completeness checker
**Rationale**: Predictable, debuggable, no ML needed

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/clarification/`:
- `mod.rs` - Public interface
- `detector.rs` - Incompleteness detection
- `questionnaire.rs` - Question generation
- `templates.rs` - Question templates per project type
- `parser.rs` - Response parsing

### Key Components
1. **SpecificationAnalyzer** - Detects missing information
2. **QuestionnaireGenerator** - Creates clarification file
3. **TemplateLibrary** - Pre-defined questions per project type
4. **ResponseParser** - Processes user answers
5. **SpecificationMerger** - Combines original + clarifications

### Implementation Tasks (in order)
1. Define question templates for common project types
2. Build specification completeness analyzer
3. Create questionnaire file generator
4. Implement response parser
5. Add specification merger
6. Integrate with generate command flow

## Question Template Examples

### REST API in Rust
```yaml
project_clarification:
  framework:
    question: "Which web framework would you like to use?"
    options: ["actix-web", "axum", "rocket", "warp"]
    default: "axum"
    
  database:
    question: "Will this API use a database?"
    type: boolean
    default: false
    follow_up:
      if_true:
        db_type:
          question: "Which database?"
          options: ["PostgreSQL", "SQLite", "MongoDB", "None"]
          
  endpoints:
    question: "Describe the main endpoints (or press enter for CRUD example):"
    type: multiline
    example: |
      GET /users - List users
      POST /users - Create user
      GET /users/:id - Get user
      
  authentication:
    question: "Include authentication?"
    options: ["JWT", "Basic", "OAuth2", "None"]
    default: "None"
```

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core clarification

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Test clarification flow
auto-dev generate "create a REST API" --dry-run
# Should create .auto-dev/clarification.yaml

# Test with answers
auto-dev generate --resume .auto-dev/clarification.yaml
```

## Success Criteria
- Detects underspecified instructions correctly
- Generates relevant questions based on project type
- Creates readable YAML questionnaire
- Parses user responses accurately
- Merges clarifications into specification
- Supports non-interactive mode

## Dependencies Required
- serde_yaml (already in use)
- Optional: dialoguer for interactive mode
- No new major dependencies

## Known Patterns and Conventions
- Follow existing instruction parser patterns
- Use Result for error handling
- Store in .auto-dev directory
- Support --yes flag for defaults

## Common Pitfalls to Avoid
- Asking too many questions
- Not providing sensible defaults
- Blocking on user input indefinitely
- Losing original instruction context
- Over-complicating question trees

## Testing Approach
- Test completeness detection rules
- Test question template selection
- Mock user responses
- Test merge logic
- Verify file generation

## Confidence Score: 9/10
Clear problem domain, template-based approach is proven, integrates well with existing flow.