# PRP: Self-Targeting Configuration

## Overview
Configure auto-dev-rs to use its existing monitoring, parsing, and analysis capabilities on its own codebase, treating itself as just another Rust project to analyze and improve.

## Context and Background
Auto-dev-rs already has all the capabilities needed to analyze and understand codebases. Rather than building special self-awareness modules, we simply need to configure it to target its own source directory using existing infrastructure.

### Research References
- Dogfooding practices: https://en.wikipedia.org/wiki/Eating_your_own_dog_food
- Rust project analysis: https://github.com/rust-lang/rust-analyzer
- Cargo workspace configuration: https://doc.rust-lang.org/cargo/reference/workspaces.html
- Project introspection: https://docs.rs/cargo_metadata/latest/cargo_metadata/

## Requirements

### Primary Goals
1. Configure existing analyzer to parse own codebase
2. Point existing monitor at own source directory
3. Use existing spec parser on own documentation
4. Apply existing synthesis to own modules
5. Enable existing metrics for self

### Technical Constraints
- Must use only existing capabilities
- No special code paths for self
- Same analysis as any Rust project
- Configuration-only changes preferred
- Must be toggleable via CLI

## Architectural Decisions

### Decision: Configuration Approach
**Chosen**: Additional configuration profile with CLI flag
**Alternatives Considered**:
- Hardcoded self-analysis: Violates principle
- New analysis modules: Unnecessary
- External tool: Adds complexity
**Rationale**: Configuration profile uses existing code as-is

### Decision: Project Detection
**Chosen**: Use cargo metadata for project info
**Alternatives Considered**:
- Manual configuration: Error-prone
- Custom detection: Reinventing wheel
- Hardcoded paths: Not portable
**Rationale**: Cargo metadata provides standard project information

## Implementation Blueprint

### File Structure
Configuration and CLI changes only:
- Create .auto-dev/self.toml configuration
- Add --target-self flag to existing commands
- Update configuration loader to handle self-targeting

### Key Components (all existing)
1. **FileAnalyzer** - Analyzes Rust files
2. **SpecParser** - Parses documentation
3. **Monitor** - Watches file changes
4. **SynthesisEngine** - Generates code
5. **MetricsCollector** - Tracks metrics

### Implementation Tasks (in order)
1. Add --target-self CLI flag to existing commands
2. Create configuration loader enhancement
3. Use cargo metadata to find workspace root
4. Configure monitor to watch src/ and PRPs/
5. Set analyzer to parse Rust files in workspace
6. Configure spec parser for .md files
7. Set synthesis target to current project
8. Enable metrics for self operations
9. Add validation that prevents dangerous operations
10. Test all existing commands with --target-self

## Configuration Example
When --target-self is used, automatically configure:
```toml
[project]
path = "." # Current directory
name = "auto-dev-rs"

[monitor]
watch = ["src/**/*.rs", "PRPs/*.md", "*.toml"]
exclude = ["target/", ".git/"]

[analyzer]
language = "rust"
workspace = true

[synthesis]
target = "."
safety_mode = "strict"
```

## Validation Gates

```bash
# Analyze self using existing command
cargo run -- analyze --target-self

# Monitor self using existing command
cargo run -- monitor --target-self

# Parse own specifications
cargo run -- parse --target-self PRPs/

# Generate metrics for self
cargo run -- metrics --target-self
```

## Success Criteria
- All existing commands work with --target-self
- No special code paths for self-analysis
- Same output format as external projects
- Performance comparable to analyzing similar-sized projects
- Safety validations prevent dangerous operations

## Known Patterns and Conventions
- Reuse all existing analyzers unchanged
- Follow existing CLI patterns
- Use same configuration structure
- Match existing command outputs

## Common Pitfalls to Avoid
- Don't create special analysis logic
- Remember existing code should work as-is
- Avoid self-specific optimizations
- Don't bypass safety checks
- Consider cargo workspace structure

## Dependencies Required
- cargo_metadata = "0.18" - Read project info
- All other dependencies already present

## Confidence Score: 9/10
This approach maximizes code reuse by treating self as any other project. The only new code is configuration handling, making this low-risk and maintainable.