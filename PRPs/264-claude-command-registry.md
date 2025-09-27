# PRP: Claude Command Registry System

**Status**: NOT STARTED  
**Priority**: Medium (P2)  
**Estimated Time**: 3 hours

## Overview
Implement a registry system that maintains all discovered Claude commands, handles registration, lookup, and provides a unified interface for command execution preparation.

## Context and Background
After parsing command files, we need a central registry that manages all available commands, handles name conflicts, provides search capabilities, and prepares commands for execution. Similar to a command palette or package registry.

### Research References
- Registry pattern: auto-dev-core/src/modules/registry.rs
- Command pattern: auto-dev/src/cli/commands/
- HashMap usage: auto-dev-core/src/learning/pattern_library.rs

## Requirements

### Primary Goals
1. Register parsed commands from both locations
2. Handle name conflicts (project overrides global)
3. Provide command lookup by name
4. Support command search/filtering
5. Maintain command metadata and stats

### Technical Constraints
- Must handle duplicate command names
- Should be thread-safe for concurrent access
- Must validate command availability
- Should track command usage

## Architectural Decisions

### Decision: Storage Structure
**Chosen**: Arc<RwLock<HashMap>> for thread safety
**Rationale**: Concurrent read access, safe updates

### Decision: Conflict Resolution
**Chosen**: Project commands override global
**Rationale**: Local customization priority

## Implementation Blueprint

### File Structure
Add to `auto-dev-core/src/claude/`:
- Create `command_registry.rs` - Registry implementation
- Update `command_types.rs` - Add registry types
- Update `mod.rs` - Export registry

### Key Components
1. **CommandRegistry** - Central registry struct
2. **RegisteredCommand** - Command with metadata
3. **CommandSource** - Enum for project/global
4. **CommandLookup** - Search functionality
5. **CommandStats** - Usage tracking

### Implementation Tasks (in order)
1. Create CommandRegistry with Arc<RwLock<HashMap>>
2. Implement register_command with source tracking
3. Add conflict resolution logic
4. Create get_command lookup method
5. Implement list_commands with filtering
6. Add command search by partial name
7. Track registration timestamp and usage count
8. Create comprehensive tests for registry

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib claude::command_registry

# Concurrency tests
cargo test --package auto-dev-core --lib claude::command_registry::concurrent -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Can register commands from multiple sources
- Project commands override global ones
- Fast lookup by exact name
- Search returns relevant results
- Thread-safe concurrent access

## Dependencies Required
Already in project:
- std::sync::{Arc, RwLock}
- std::collections::HashMap
- chrono for timestamps

## Known Patterns and Conventions
- Use Arc<RwLock<>> for shared state
- Include source in command metadata
- Use Option for lookup results
- Log conflicts at debug level

## Common Pitfalls to Avoid
- Don't hold write locks too long
- Handle poisoned locks gracefully
- Validate command names before registration
- Don't lose global commands on override
- Consider case-sensitive names

## Testing Approach
- Test registration from both sources
- Test conflict resolution
- Test concurrent access
- Test search functionality
- Benchmark lookup performance

## Confidence Score: 8/10
Standard registry pattern with established concurrency patterns in Rust.