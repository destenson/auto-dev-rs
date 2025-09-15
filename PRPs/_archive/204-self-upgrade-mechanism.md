# PRP: Self-Upgrade and Restart Mechanism

## Overview
Implement a safe self-upgrade mechanism that allows auto-dev-rs to compile, test, and replace itself with a new version while maintaining the ability to rollback if issues occur.

## Context and Background
When auto-dev-rs modifies its own code, it needs to compile the new version, verify it works, and seamlessly transition to running the new binary. This requires careful orchestration to avoid leaving the system in a broken state.

### Research References
- Self-updating binaries: https://github.com/jaemk/self_update
- Hot reloading in Rust: https://robert.kra.hn/posts/hot-reloading-rust/
- Binary replacement strategies: https://docs.microsoft.com/en-us/windows/win32/debug/pe-format
- Unix exec system call: https://man7.org/linux/man-pages/man3/exec.3.html

## Requirements

### Primary Goals
1. Compile modified version of itself
2. Run verification tests on new version
3. Safely replace running binary
4. Maintain rollback capability
5. Preserve state across upgrade

### Technical Constraints
- Must handle platform differences (Windows/Unix)
- Cannot lose in-progress work
- Must verify new version before switching
- Should support gradual rollout
- Must handle compilation failures gracefully

## Architectural Decisions

### Decision: Upgrade Strategy
**Chosen**: Build-verify-stage-swap approach
**Alternatives Considered**:
- Direct binary replacement: Too risky
- Blue-green deployment: Overkill for single binary
- Containerized versions: Adds complexity
**Rationale**: Staged approach allows verification before commitment

### Decision: Platform Handling
**Chosen**: Platform-specific implementations with common interface
**Alternatives Considered**:
- Unix-only: Limits usability
- Lowest common denominator: Misses platform capabilities
- External upgrader: Adds dependency
**Rationale**: Platform-specific code maximizes reliability on each OS

## Implementation Blueprint

### File Structure
Create module in auto-dev-core/src/self_upgrade/
- mod.rs - Common interface
- upgrader.rs - Upgrade orchestration
- verifier.rs - New version verification  
- state_preserver.rs - State persistence
- rollback.rs - Rollback mechanism
- platform/
  - unix.rs - Unix-specific implementation
  - windows.rs - Windows-specific implementation

### Key Components
1. **SelfUpgrader** - Main upgrade orchestrator
2. **VersionVerifier** - Tests new version
3. **StatePreserver** - Saves/restores state
4. **RollbackManager** - Handles rollback
5. **BinarySwapper** - Platform-specific binary replacement

### Implementation Tasks (in order)
1. Create self_upgrade module structure
2. Implement state preservation mechanism
3. Build compilation orchestrator
4. Create verification test suite
5. Implement Unix binary replacement using exec
6. Implement Windows binary replacement
7. Add rollback mechanism with version history
8. Create upgrade transaction log
9. Implement gradual verification
10. Add CLI commands for upgrade control
11. Write platform-specific tests
12. Add recovery mechanism for failed upgrades

## Validation Gates

```bash
# Test compilation of modified version
cargo build --release

# Run verification suite
cargo test --release

# Test upgrade mechanism (dry-run)
cargo run -- self-upgrade --dry-run

# Verify rollback capability
cargo run -- self-upgrade rollback --test

# Check state preservation
cargo run -- self-upgrade --verify-state
```

## Success Criteria
- Successfully compiles modified version
- Passes verification tests before upgrade
- Seamlessly transitions to new version
- Preserves all state and in-progress work
- Can rollback within 10 seconds
- Handles compilation failures gracefully

## Known Patterns and Conventions
- Use Command pattern for upgrade steps
- Follow transaction pattern for atomicity
- Reuse existing state persistence from synthesis module
- Match platform detection from std::env::consts::OS

## Common Pitfalls to Avoid
- Don't assume binary can be replaced while running (Windows)
- Remember to preserve file permissions on Unix
- Handle anti-virus interference on Windows
- Don't lose environment variables across restart
- Consider open file handles during replacement

## Dependencies Required
- tempfile = "3.0" - For staging area
- which = "4.0" - For finding binary path
- Platform-specific: libc (Unix), winapi (Windows)

## Platform-Specific Considerations

### Unix/Linux/macOS
- Use fork/exec for seamless transition
- Preserve signal handlers
- Handle stdout/stderr redirection
- Maintain process group

### Windows
- Use MoveFileEx with MOVEFILE_DELAY_UNTIL_REBOOT flag
- Handle file locking issues
- Create wrapper exe for replacement
- Consider Windows Defender interference

## Confidence Score: 6/10
This is complex due to platform differences and the critical nature of self-replacement. The Unix implementation is straightforward, but Windows binary replacement while running requires careful handling.