# PRP: Claude Command File Parser

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 3 hours

## Overview
Implement parser for command files in .claude/commands/ directory, extracting command metadata, arguments, and execution instructions from markdown files.

## Context and Background
Claude command files are markdown documents that define reusable commands. Each file represents a command with its name, description, arguments, and execution instructions. Similar to GitHub Actions or npm scripts but in markdown format.

### Research References
- Command examples: ~/.claude/commands/*.md
- Markdown frontmatter parsing: https://docs.rs/gray_matter/latest/gray_matter/
- Command pattern in CLI tools: auto-dev/src/cli/commands/

## Requirements

### Primary Goals
1. Parse command markdown files
2. Extract command name from filename
3. Parse command description and usage
4. Extract argument definitions
5. Store command instructions for execution

### Technical Constraints
- Must handle various markdown formats
- Should extract structured data from unstructured text
- Must validate command names are valid
- Should handle malformed files gracefully

## Architectural Decisions

### Decision: Parser Strategy
**Chosen**: Simple line-based parsing with patterns
**Rationale**: Commands follow consistent structure

### Decision: Command Storage
**Chosen**: HashMap with command name as key
**Rationale**: Fast lookup, simple implementation

## Implementation Blueprint

### File Structure
Add to `auto-dev-core/src/claude/`:
- Create `command_parser.rs` - Command parsing logic
- Create `command_types.rs` - Command data structures
- Update `mod.rs` - Export command parser

### Key Components
1. **CommandParser** - Main parser struct
2. **ClaudeCommand** - Parsed command representation
3. **CommandArgument** - Argument definition
4. **parse_command_file** - File parsing function
5. **CommandRegistry** - Collection of commands

### Implementation Tasks (in order)
1. Define ClaudeCommand and CommandArgument structs
2. Implement filename to command name extraction
3. Create markdown section parser (## Usage, ## Arguments, etc.)
4. Extract command description from first paragraph
5. Parse argument definitions from content
6. Build CommandRegistry for all commands
7. Add validation for command names and structure
8. Create comprehensive tests

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib claude::command_parser

# Test with real command files
cargo test --package auto-dev-core --lib claude::command_parser::real_files -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Parses all command files in directory
- Extracts command name from filename correctly
- Captures usage instructions
- Identifies argument patterns
- Handles malformed files without crashing

## Dependencies Required
Already in project:
- std::fs for directory operations
- regex for pattern matching (if not present, can use string methods)
- serde for serialization

## Known Patterns and Conventions
- Command name from filename (remove .md extension)
- First paragraph is description
- ## sections denote different parts
- Arguments often in Usage section

## Common Pitfalls to Avoid
- Don't assume file structure is perfect
- Handle missing sections gracefully
- Validate command names (no spaces, special chars)
- Don't execute any code from files
- Handle empty or corrupt files

## Testing Approach
- Test with sample command files
- Test with minimal command files
- Test with malformed markdown
- Test command name extraction
- Test argument parsing

## Confidence Score: 8/10
Well-defined task but requires careful parsing of semi-structured data.