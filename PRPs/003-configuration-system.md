# PRP: Configuration System with Multi-Platform Support

## Overview
Implement a flexible configuration system using serde with TOML/JSON support, including ability to read and integrate configurations from other code generation platforms (.cursor/*, .claude/*, etc.).

## Context and Background
The system needs hierarchical configuration loading: defaults → global config → project config → CLI arguments. Additionally, it should recognize and utilize existing configurations from platforms like Cursor, Claude, and others for seamless interoperability.

### Research References
- Serde documentation: https://serde.rs/
- TOML crate: https://docs.rs/toml/latest/toml/
- Config crate for layered configuration: https://docs.rs/config/latest/config/
- Persistent config pattern: https://docs.rs/persistent_config/latest/persistent_config/

## Requirements

### Primary Goals
1. Create configuration struct hierarchy with serde derives
2. Implement layered configuration loading
3. Support TOML and JSON formats
4. Integrate .cursor/*, .claude/*, and other platform configs
5. Provide configuration validation and defaults

### Technical Constraints
- Use serde with derive features
- Support both TOML and JSON formats
- Maintain backward compatibility
- Respect existing platform configuration files
- Follow XDG base directory specification on Unix

## Implementation Blueprint

### File Structure
```
src/
├── config/
│   ├── mod.rs           # Config module exports
│   ├── types.rs         # Configuration structs
│   ├── loader.rs        # Loading and merging logic
│   ├── platforms/       # Platform-specific config readers
│   │   ├── mod.rs
│   │   ├── cursor.rs    # Cursor config integration
│   │   ├── claude.rs    # Claude config integration
│   │   └── vscode.rs    # VS Code settings integration
│   └── defaults.rs      # Default configuration values
```

### Key Components
1. **Config Struct**: Main configuration with nested sections
2. **Platform Readers**: Parsers for .cursor/*, .claude/* files
3. **Config Loader**: Merges configs from multiple sources
4. **Validation**: Schema validation for loaded configs
5. **Config Paths**: Standard locations for config files

### Implementation Tasks (in order)
1. Add serde, serde_json, toml, and config to Cargo.toml
2. Create src/config module structure
3. Define Config struct with all settings
4. Implement Default trait with sensible defaults
5. Create platform config detection functions
6. Implement cursor config reader (.cursor/settings.json)
7. Implement claude config reader (.claude/config.toml)
8. Create config merger with priority ordering
9. Add environment variable override support
10. Implement config validation and error reporting
11. Create config initialization for new projects
12. Add config migration for version updates

## Configuration Priority Order
1. CLI arguments (highest priority)
2. Environment variables (AUTO_DEV_*)
3. Project-local config (.auto-dev/config.toml)
4. Platform-specific configs (.cursor/*, .claude/*)
5. User global config (~/.config/auto-dev/config.toml)
6. System defaults (lowest priority)

## Platform Integration Details

### Cursor Platform (.cursor/*)
- Read .cursor/settings.json for project preferences
- Extract relevant AI model configurations
- Import task definitions and workflows

### Claude Platform (.claude/*)
- Parse .claude/config.toml for project settings
- Import CLAUDE.md instructions if present
- Utilize prompt templates and procedures

### VS Code (.vscode/*)
- Read .vscode/settings.json for relevant configs
- Import task.json for task definitions
- Respect workspace configurations

## Validation Gates

```bash
# Build and format
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Test configuration loading
cargo test config::tests

# Verify platform config detection
mkdir -p .cursor .claude .vscode
echo '{"test": true}' > .cursor/settings.json
cargo run -- config validate

# Test config merging
cargo run -- config show
```

## Success Criteria
- Configuration loads from multiple sources correctly
- Platform configs are detected and integrated
- Merge order respects priority
- Invalid configs produce clear errors
- Default config works out of the box

## Known Patterns and Conventions
- Use #[derive(Serialize, Deserialize, Debug, Clone)]
- Use #[serde(default)] for optional fields
- Use #[serde(rename_all = "kebab-case")] for TOML
- Implement Default trait for all config structs
- Use Option<T> for truly optional settings

## Common Pitfalls to Avoid
- Don't hardcode paths - use platform-appropriate locations
- Handle missing config files gracefully
- Don't lose type safety with excessive use of Value
- Remember to validate after merging configs
- Avoid circular dependencies in config references

## Dependencies Required
- serde = { version = "1.0", features = ["derive"] }
- serde_json = "1.0"
- toml = "0.8"
- config = "0.14"
- directories = "5.0"  # For XDG paths

## Example Configuration Structure
```toml
[general]
project-name = "my-project"
verbosity = "info"

[plugins]
enabled = ["code-generator", "test-runner"]
path = "~/.auto-dev/plugins"

[templates]
path = "~/.auto-dev/templates"
default-language = "rust"

[integrations]
use-cursor-config = true
use-claude-config = true
```

## Confidence Score: 8/10
Well-defined scope with clear integration points. Platform config formats may vary, requiring adaptive parsing.