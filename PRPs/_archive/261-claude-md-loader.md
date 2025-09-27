# PRP: CLAUDE.md File Loader

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 2 hours

## Overview
Implement loader for CLAUDE.md files that contain user instructions and context, parsing and validating their content for integration with auto-dev's context management system.

## Context and Background
CLAUDE.md files contain user-specific instructions and context that should guide auto-dev's behavior. These are markdown files with directives, similar to .gitignore or .editorconfig but for AI agent behavior.

### Research References
- Example CLAUDE.md structure: ~/.claude/CLAUDE.md
- Markdown parsing: https://docs.rs/pulldown-cmark/latest/pulldown_cmark/
- File reading pattern: auto-dev-core/src/llm/config.rs (lines 48-54)

## Requirements

### Primary Goals
1. Load CLAUDE.md from discovered paths
2. Parse markdown content safely
3. Merge multiple CLAUDE.md files (global + project)
4. Validate content structure
5. Handle encoding issues gracefully

### Technical Constraints
- Must handle UTF-8 and UTF-8 BOM
- Should support large files (up to 1MB)
- Must not execute any code from files
- Should preserve formatting for display

## Architectural Decisions

### Decision: Parser Choice
**Chosen**: Plain text reading (no markdown parsing needed initially)
**Rationale**: CLAUDE.md is primarily plain text instructions

### Decision: Merge Strategy
**Chosen**: Concatenate with clear separators
**Rationale**: Simple, preserves all context

## Implementation Blueprint

### File Structure
Add to `auto-dev-core/src/claude/`:
- Create `claude_md.rs` - CLAUDE.md loader
- Update `mod.rs` - Export ClaudeMdLoader

### Key Components
1. **ClaudeMdLoader** - Main loader struct
2. **ClaudeMdContent** - Parsed content holder
3. **load_and_merge** - Combine multiple files
4. **validate_content** - Basic validation

### Implementation Tasks (in order)
1. Create ClaudeMdLoader struct
2. Implement file reading with UTF-8 handling
3. Add file size validation (max 1MB)
4. Implement merge logic with separators
5. Add content validation (no scripts, reasonable size)
6. Create unit tests with sample files
7. Add error handling for missing/corrupt files

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib claude::claude_md

# Test with sample files
cargo test --package auto-dev-core --lib claude::claude_md::integration -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Loads CLAUDE.md files successfully
- Handles missing files gracefully (returns None)
- Merges multiple files with clear separation
- Validates file size limits
- Preserves original formatting

## Dependencies Required
Already in project:
- std::fs for file operations
- String/str for text handling
- anyhow for error handling

## Known Patterns and Conventions
- Use std::fs::read_to_string for simplicity
- Check file size before reading
- Use anyhow::Context for error context
- Return Option<String> for missing files

## Common Pitfalls to Avoid
- Don't parse as code or execute anything
- Handle BOM characters in UTF-8
- Check file size before loading into memory
- Validate paths are actually files
- Handle Windows line endings

## Testing Approach
- Test with sample CLAUDE.md files
- Test with empty files
- Test with large files (boundary testing)
- Test with invalid UTF-8
- Test merge scenarios

## Confidence Score: 9/10
Simple file reading task with clear requirements and no complex parsing needed.