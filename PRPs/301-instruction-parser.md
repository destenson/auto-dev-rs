# PRP: Instruction File Parser and Loader

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 2 hours

## Overview
Create a robust instruction parser that reads project specifications from files (markdown, yaml, json, text) or command-line strings. Extracts structured requirements without requiring LLMs.

## Context and Background
Instructions can come from various sources - CLI arguments, markdown files, YAML specs, or plain text. The parser needs to extract actionable information while preserving the original intent for later implementation phases.

### Research References
- YAML parsing: https://docs.rs/serde_yaml/latest/serde_yaml/
- Markdown parsing: https://docs.rs/pulldown-cmark/latest/pulldown_cmark/
- Similar to existing ParseArgs in `auto-dev/src/cli/app.rs`

## Requirements

### Primary Goals
1. Parse instruction files in multiple formats
2. Extract project metadata (name, type hints, dependencies)
3. Preserve full instruction text for implementation phase
4. Support both file paths and inline strings

### Technical Constraints
- No LLM parsing - use structured formats and heuristics
- Must handle malformed input gracefully
- Integrate with existing parse command structure

## Architectural Decisions

### Decision: Parser Architecture
**Chosen**: Format-specific parsers with common interface
**Rationale**: Clean separation, easy to extend

### Decision: Data Extraction
**Chosen**: Regex patterns and structured format parsing
**Rationale**: Predictable, debuggable, no ML needed

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/instructions/`:
- `mod.rs` - Public interface
- `parser.rs` - Main parser logic
- `formats.rs` - Format-specific parsers
- `extractor.rs` - Metadata extraction

### Key Components
1. **InstructionParser** - Main entry point
2. **FormatDetector** - Identifies file format
3. **MetadataExtractor** - Pulls out structured data
4. **InstructionDocument** - Parsed result structure

### Implementation Tasks (in order)
1. Define InstructionDocument struct with metadata
2. Implement format detection from extension/content
3. Create markdown parser for structured specs
4. Add YAML/JSON parser for formal specs
5. Build metadata extractor with regex patterns
6. Add plain text fallback parser

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core instructions

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Test parsing
echo "Build a REST API in Rust" | auto-dev parse -
auto-dev parse examples/project.md
auto-dev parse specifications/api.yaml
```

## Success Criteria
- Parses markdown with code blocks and headers
- Extracts YAML/JSON specifications
- Handles plain text instructions
- Identifies project name, type, dependencies
- Preserves complete original text
- Returns structured InstructionDocument

## Dependencies Required
- serde_yaml (for YAML parsing)
- pulldown-cmark (for markdown parsing)
- regex (for pattern extraction)
- All already in use or minimal additions

## Known Patterns and Conventions
- Use existing parse command as integration point
- Follow Result<T> error handling pattern
- Implement Display trait for output
- Use serde for serialization

## Common Pitfalls to Avoid
- Don't try to parse natural language deeply
- Keep extraction rules simple and visible
- Handle encoding issues in text files
- Don't lose original formatting
- Test with minimal/malformed inputs

## Testing Approach
- Unit test each format parser
- Test metadata extraction patterns
- Test format detection logic
- Integration test with real files
- Test error cases and recovery

## Confidence Score: 9/10
Well-defined scope, leverages existing parsing libraries, clear integration path.