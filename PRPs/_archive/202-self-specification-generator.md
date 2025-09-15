# PRP: TODO and Documentation Specification Parsing

## Overview
Configure the existing specification parser to extract requirements from auto-dev-rs's own TODO comments, documentation, and markdown files, treating them as specification sources like any other project.

## Context and Background
Auto-dev-rs already has a SpecParser that can extract requirements from various formats. We need to extend it to recognize TODO comments and documentation patterns as specification sources, allowing it to generate actionable specs from its own codebase.

### Research References
- TODO comment conventions: https://stackoverflow.com/questions/16767921/what-is-the-format-for-todo-comments
- Markdown parsing: https://commonmark.org/
- Rust doc comments: https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html
- GitHub issue references: https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/autolinked-references-and-urls

## Requirements

### Primary Goals
1. Extend existing parser to recognize TODO patterns
2. Parse requirements from markdown documentation
3. Extract specs from doc comments
4. Use existing synthesis format
5. Prioritize based on markers (FIXME, TODO, HACK)

### Technical Constraints
- Must use existing SpecParser infrastructure
- Output must match existing Specification model
- No special handling for self
- Same parsing rules for all projects
- Should be configurable

## Architectural Decisions

### Decision: Parser Extension Strategy
**Chosen**: Add TODO extractor to existing parser
**Alternatives Considered**:
- Separate TODO parser: Duplicates logic
- External preprocessor: Adds complexity
- Manual extraction: Not scalable
**Rationale**: Extending existing parser maintains consistency

### Decision: Priority Mapping
**Chosen**: Map TODO markers to existing Priority enum
**Alternatives Considered**:
- New priority system: Breaks compatibility
- Ignore priorities: Loses information
- Custom weights: Too complex
**Rationale**: Reusing Priority enum maintains compatibility

## Implementation Blueprint

### File Structure
Extend existing parser in auto-dev-core/src/parser/
- Add todo_extractor.rs - TODO pattern extraction
- Extend markdown.rs - Enhanced markdown parsing
- Update extractor.rs - Include TODO sources

### Key Components (extend existing)
1. **SpecParser** - Already exists, add TODO support
2. **MarkdownParser** - Already exists, enhance patterns
3. **RequirementExtractor** - Already exists, add TODO extraction
4. Use existing **Specification** model
5. Use existing **Priority** enum

### Implementation Tasks (in order)
1. Add TODO regex patterns to extractor
2. Map TODO markers to Priority enum (FIXME=High, TODO=Medium, HACK=Low)
3. Extend markdown parser for requirement patterns
4. Add doc comment requirement extraction
5. Configure file patterns for TODO search
6. Integrate with existing parse command
7. Add tests for TODO extraction
8. Update existing examples
9. Document new patterns
10. Test on auto-dev-rs itself

## TODO Pattern Recognition
Recognize standard patterns:
```rust
// TODO: Implement feature X
// FIXME: Critical bug in Y
// HACK: Temporary workaround for Z
// TODO(username): Assigned task
// TODO: [High Priority] Important feature
```

Map to existing structures:
- Extract description
- Infer priority from marker
- Generate unique ID
- Set category based on context

## Validation Gates

```bash
# Parse TODOs using existing command
cargo run -- parse --include-todos src/

# Parse documentation as specs
cargo run -- parse PRPs/*.md

# Verify extracted specifications
cargo run -- parse --target-self --validate

# Test priority mapping
cargo run -- parse --show-priorities
```

## Success Criteria
- Existing parse command handles TODOs
- Output format unchanged
- TODO priorities map correctly
- Works on any Rust project
- No special logic for auto-dev-rs

## Known Patterns and Conventions
- Use existing Specification model
- Follow existing parser patterns
- Reuse Priority enum
- Match existing command interface

## Common Pitfalls to Avoid
- Don't create new specification format
- Remember commented-out code isn't TODO
- Avoid parsing TODOs in dependencies
- Don't change existing parser behavior
- Consider multi-line TODO comments

## Dependencies Required
- Already available: regex, pulldown-cmark
- No new dependencies needed

## Configuration Addition
Add to existing parser config:
```toml
[parser]
include_todos = true
todo_patterns = ["TODO", "FIXME", "HACK", "XXX"]
todo_file_types = ["*.rs", "*.md", "*.toml"]

[parser.priority_mapping]
FIXME = "High"
TODO = "Medium"  
HACK = "Low"
XXX = "Medium"
```

## Confidence Score: 8/10
This extends existing functionality rather than creating new systems. The main work is pattern recognition and mapping, which is straightforward with existing infrastructure.