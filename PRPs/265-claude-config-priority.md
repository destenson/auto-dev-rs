# PRP: Claude Configuration Priority System

**Status**: NOT STARTED  
**Priority**: Medium (P2)  
**Estimated Time**: 2-3 hours

## Overview
Implement a priority and override system that manages configuration precedence between project-local and global Claude settings, ensuring correct layering and merge behavior.

## Context and Background
Configuration systems typically follow a precedence order where more specific settings override general ones. We need a clear priority system: project .claude > user ~/.claude > defaults, with proper merge strategies for different configuration types.

### Research References
- Config precedence in git: https://git-scm.com/docs/git-config#_configuration_file
- Cargo config layering: https://doc.rust-lang.org/cargo/reference/config.html
- VS Code settings priority: https://code.visualstudio.com/docs/getstarted/settings

## Requirements

### Primary Goals
1. Define clear priority levels
2. Implement merge strategies per config type
3. Handle partial overrides correctly
4. Provide visibility into active config
5. Support explicit priority override

### Technical Constraints
- Must maintain predictability
- Should log override decisions
- Must handle missing configs gracefully
- Should support future config sources

## Architectural Decisions

### Decision: Priority Order
**Chosen**: Project > User > System > Defaults
**Rationale**: Standard convention across tools

### Decision: Merge Strategy
**Chosen**: Type-specific (replace vs merge)
**Rationale**: Different configs need different strategies

## Implementation Blueprint

### File Structure
Add to `auto-dev-core/src/claude/`:
- Create `config_priority.rs` - Priority system
- Create `config_merger.rs` - Merge strategies
- Update `mod.rs` - Export priority system

### Key Components
1. **ConfigPriority** - Enum for priority levels
2. **ConfigMerger** - Merge strategy implementation
3. **MergeStrategy** - Enum (Replace/Merge/Append)
4. **ConfigLayer** - Single configuration layer
5. **ResolvedConfig** - Final merged configuration

### Implementation Tasks (in order)
1. Define ConfigPriority enum with ordering
2. Create ConfigLayer to wrap configs with priority
3. Implement merge strategies for different types
4. Build layered configuration resolver
5. Add logging for override decisions
6. Create configuration inspector for debugging
7. Implement priority override mechanism
8. Add comprehensive tests

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib claude::config_priority

# Test merge scenarios
cargo test --package auto-dev-core --lib claude::config_merger -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Project config overrides global correctly
- CLAUDE.md files merge with proper separators
- Commands follow override rules
- Can inspect active configuration
- Predictable behavior across scenarios

## Dependencies Required
Already in project:
- std::cmp::Ordering for priority
- tracing for logging decisions
- serde for configuration

## Known Patterns and Conventions
- Use Ord trait for priority comparison
- Log at debug level for overrides
- Return merged result, not mutate
- Document merge behavior clearly

## Common Pitfalls to Avoid
- Don't lose information during merge
- Make priority order configurable
- Handle circular dependencies
- Consider future config sources
- Test edge cases thoroughly

## Testing Approach
- Test each priority level
- Test partial overrides
- Test merge strategies
- Test missing configurations
- Test inspector functionality

## Confidence Score: 8/10
Well-established pattern with clear requirements and precedents in other tools.