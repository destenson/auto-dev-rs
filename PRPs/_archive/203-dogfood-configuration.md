# PRP: Dogfood Configuration Profile

## Overview
Create a configuration profile that allows auto-dev-rs to target itself as a development project, using its existing monitoring, parsing, and synthesis capabilities on its own codebase.

## Context and Background
Rather than creating special self-development modules, auto-dev-rs should dogfood its own capabilities by treating itself as just another project. This requires a configuration profile that points the existing infrastructure at its own source tree with appropriate safety constraints.

### Research References
- Dogfooding practices: https://en.wikipedia.org/wiki/Eating_your_own_dog_food
- Configuration profiles: https://doc.rust-lang.org/cargo/reference/profiles.html
- Recursive development: https://www.gnu.org/software/automake/manual/html_node/Bootstrapping.html
- Safety constraints: https://docs.rs/sandbox/latest/sandbox/

## Requirements

### Primary Goals
1. Create configuration profile for self-development
2. Point existing monitors at own source tree
3. Configure synthesis engine for self-modification
4. Set up appropriate safety boundaries
5. Enable/disable self-development mode

### Technical Constraints
- Must use existing infrastructure unchanged
- Cannot modify core monitoring/synthesis logic
- Must be toggleable via CLI flag
- Should support dry-run mode
- Must preserve normal operation mode

## Architectural Decisions

### Decision: Configuration Approach
**Chosen**: Additional TOML profile with CLI override
**Alternatives Considered**:
- Hardcoded self mode: Too inflexible
- Environment variables only: Hard to manage
- Separate binary: Unnecessary duplication
**Rationale**: TOML profile provides persistence while CLI allows quick toggles

### Decision: Safety Strategy
**Chosen**: Layered constraints in configuration
**Alternatives Considered**:
- Code-level safety checks: Violates principle of treating self as normal project
- External supervisor: Too complex
- No safety: Too dangerous
**Rationale**: Configuration-based safety maintains principle while protecting system

## Implementation Blueprint

### File Structure
Create configuration files and CLI additions:
- auto-dev.dogfood.toml - Dogfood configuration profile
- src/cli/commands/dogfood.rs - CLI command for self-development
- .auto-dev/dogfood/config.toml - Runtime configuration

### Key Components
1. **DogfoodConfig** - Configuration structure
2. **DogfoodCommand** - CLI interface
3. Configuration validation
4. Safety boundary definitions
5. Dry-run mode support

### Implementation Tasks (in order)
1. Create dogfood configuration schema
2. Add auto-dev.dogfood.toml with self-targeting paths
3. Implement CLI command: `auto-dev dogfood`
4. Add configuration validation logic
5. Define safety boundaries (exclude critical files)
6. Implement dry-run mode for testing
7. Add rollback configuration
8. Create bootstrap script
9. Document configuration options
10. Add integration tests

## Validation Gates

```bash
# Validate configuration
cargo run -- dogfood validate

# Test dry-run mode
cargo run -- dogfood --dry-run "Add new CLI command"

# Check safety boundaries
cargo run -- dogfood check-safety

# Run in monitoring mode only
cargo run -- dogfood monitor
```

## Success Criteria
- Can monitor own source tree using existing monitor
- Generates valid specifications from own TODOs
- Synthesis engine accepts self as target
- Safety boundaries prevent critical file modification
- Dry-run mode shows planned changes without applying

## Known Patterns and Conventions
- Follow existing TOML configuration structure
- Use same path resolution as normal mode
- Reuse existing CLI command patterns
- Match configuration loading from Config struct

## Common Pitfalls to Avoid
- Don't hardcode paths - use relative from config
- Remember to exclude target/ and .git/
- Don't allow modification of Cargo.lock during builds
- Prevent modifications to running binary
- Consider file locks during self-modification

## Dependencies Required
- Already available: all existing infrastructure
- No new dependencies needed

## Configuration Example
Example auto-dev.dogfood.toml structure to be created:
- project_name = "auto-dev-rs"
- mode = "dogfood"
- monitoring.watch_patterns = ["*.rs", "*.toml", "*.md"]
- monitoring.exclude = ["target/", ".git/", "*.lock"]
- synthesis.target_dir = "."
- synthesis.safety_mode = "strict"
- synthesis.allow_paths = ["src/", "tests/", "docs/"]
- synthesis.deny_paths = ["src/main.rs", "Cargo.toml"]

## Confidence Score: 9/10
This approach leverages all existing infrastructure without modification. The main work is configuration and safety boundaries, which are well-understood problems.