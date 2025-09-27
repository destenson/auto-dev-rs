# PRP: Claude Configuration Discovery System

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 2-3 hours

## Overview
Implement discovery mechanism to locate and identify .claude directories in both project-local and user home locations, establishing the foundation for Claude configuration awareness.

## Context and Background
Auto-dev needs to discover Claude configuration directories following standard dotfile conventions. The system should check both project-specific (./.claude) and global (~/.claude) locations, similar to how git finds .git directories or how MCP discovery works.

### Research References
- MCP discovery pattern: auto-dev-core/src/mcp/discovery.rs (lines 171-201)
- dirs crate documentation: https://docs.rs/dirs/latest/dirs/
- Configuration loading pattern: auto-dev-core/src/llm/config.rs (lines 48-64)

## Requirements

### Primary Goals
1. Discover .claude directory in project root
2. Discover ~/.claude in user home directory  
3. Determine which directories exist and are readable
4. Establish priority order (project overrides global)
5. Cache discovery results for performance

### Technical Constraints
- Must work cross-platform (Windows, Mac, Linux)
- Should handle permission errors gracefully
- Must not fail if directories don't exist
- Should follow symlinks appropriately

## Architectural Decisions

### Decision: Discovery Order
**Chosen**: Project-first with fallback to global
**Rationale**: Follows established convention (git, npm, cargo)

### Decision: Caching Strategy  
**Chosen**: Lazy discovery with TTL cache
**Rationale**: Balances performance with freshness

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/claude/`:
- Create `mod.rs` - Module exports
- Create `discovery.rs` - Discovery logic
- Update `auto-dev-core/src/lib.rs` - Add claude module

### Key Components
1. **ClaudeConfigDiscovery** - Main discovery struct
2. **ClaudeConfigLocation** - Enum for project/global/both
3. **ClaudeConfigPaths** - Struct holding discovered paths
4. **discovery_cache** - Simple TTL cache

### Implementation Tasks (in order)
1. Create claude module structure
2. Implement path discovery using dirs crate
3. Add existence and permission checks
4. Implement caching with 5-minute TTL
5. Add priority resolution logic
6. Create unit tests for discovery
7. Add cross-platform tests

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib claude::discovery

# Verify cross-platform
cargo test --package auto-dev-core --lib claude::discovery::windows -- --ignored
cargo test --package auto-dev-core --lib claude::discovery::unix -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Correctly discovers .claude in project root
- Correctly discovers ~/.claude in home directory
- Returns empty result when directories don't exist
- Handles permission errors without panicking
- Cache improves performance by 10x on repeated calls

## Dependencies Required
Already in project:
- dirs crate (via existing dependencies)
- anyhow for error handling
- tracing for logging

## Known Patterns and Conventions
- Follow MCP discovery pattern for consistency
- Use dirs::home_dir() for cross-platform home
- Use PathBuf for all path operations
- Return Option or Result, never panic

## Common Pitfalls to Avoid
- Don't assume directories exist
- Handle Windows path separators
- Check permissions before reading
- Don't follow infinite symlink loops
- Cache invalidation on directory changes

## Testing Approach
- Mock home directory for tests
- Test with missing directories
- Test with permission denied
- Test with symlinks
- Benchmark cache performance

## Confidence Score: 9/10
Clear requirements, existing patterns to follow, minimal external dependencies.