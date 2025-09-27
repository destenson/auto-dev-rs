# PRP: Claude Configuration File Watcher

**Status**: NOT STARTED  
**Priority**: Medium (P3)  
**Estimated Time**: 3 hours

## Overview
Implement file watching for .claude directories to detect changes in CLAUDE.md and command files, enabling hot-reload of configuration without restarting auto-dev.

## Context and Background
Auto-dev already has file watching infrastructure through the notify crate. We need to extend this to monitor .claude directories for changes and trigger appropriate reload actions when files are modified.

### Research References
- Existing watcher: auto-dev-core/src/monitor/watcher.rs (lines 40-100)
- Notify crate usage: auto-dev-core/src/monitor/watcher.rs (lines 64-77)
- Event handling: auto-dev-core/src/monitor/watcher.rs (lines 66-76)

## Requirements

### Primary Goals
1. Watch .claude directories for changes
2. Detect CLAUDE.md modifications
3. Monitor commands/ directory changes
4. Trigger reload on relevant changes
5. Debounce rapid changes

### Technical Constraints
- Must reuse existing watcher infrastructure
- Should minimize resource usage
- Must handle directory creation/deletion
- Should debounce rapid edits

## Architectural Decisions

### Decision: Integration Approach
**Chosen**: Extend existing FileWatcher
**Rationale**: Reuse proven infrastructure

### Decision: Reload Strategy
**Chosen**: Selective reload based on change type
**Rationale**: Minimize disruption

## Implementation Blueprint

### File Structure
Add to `auto-dev-core/src/claude/`:
- Create `config_watcher.rs` - Watch implementation
- Create `reload_handler.rs` - Reload logic
- Update `mod.rs` - Export watcher

### Key Components
1. **ClaudeConfigWatcher** - Specialized watcher
2. **ClaudeFileChange** - Change event type
3. **ReloadHandler** - Processes changes
4. **DebounceBuffer** - Prevents rapid reloads
5. **WatcherIntegration** - Connects to FileWatcher

### Implementation Tasks (in order)
1. Create ClaudeConfigWatcher wrapping FileWatcher
2. Configure paths for .claude directories
3. Implement event filtering for relevant files
4. Add debounce logic (500ms window)
5. Create reload handler for different file types
6. Integrate with CommandRegistry for command updates
7. Update ContextManager on CLAUDE.md changes
8. Add tests with mock file system events

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib claude::config_watcher

# Integration tests with file system
cargo test --package auto-dev-core --lib claude::config_watcher::fs -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Detects file changes within 1 second
- Reloads configuration without restart
- Handles rapid edits with debouncing
- Watches both project and home directories
- Minimal performance impact

## Dependencies Required
Already in project:
- notify crate for file watching
- tokio for async handling
- mpsc channels for events

## Known Patterns and Conventions
- Use existing FileWatcher patterns
- Send events through channels
- Debounce with time windows
- Log reload actions at info level

## Common Pitfalls to Avoid
- Don't watch too many files
- Handle directory recreation
- Avoid infinite reload loops
- Consider editor swap files
- Test with various editors

## Testing Approach
- Test with file modifications
- Test with file creation/deletion
- Test debounce behavior
- Test directory watching
- Mock file system events

## Confidence Score: 7/10
Complexity in handling various editor behaviors and file system events.