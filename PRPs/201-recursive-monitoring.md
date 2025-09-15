# PRP: Recursive Self-Monitoring

## Overview
Enable auto-dev-rs to monitor its own source directory for changes, creating a recursive monitoring capability where the system watches and responds to modifications in its own codebase.

## Context and Background
Building on the existing filesystem monitoring capability (PRP 100), this module specifically configures monitoring for the auto-dev-rs source tree itself, with special handling to prevent infinite loops and dangerous modifications.

### Research References
- Watchdog patterns: https://github.com/gorakhargosh/watchdog
- File monitoring in Rust: https://docs.rs/notify/latest/notify/
- Recursive monitoring pitfalls: https://stackoverflow.com/questions/tagged/file-watcher
- Self-modifying code safety: https://en.wikipedia.org/wiki/Self-modifying_code

## Requirements

### Primary Goals
1. Monitor own source directory for changes
2. Distinguish between safe and unsafe self-modifications
3. Prevent infinite modification loops
4. Track modification history
5. Integrate with existing monitoring infrastructure

### Technical Constraints
- Must prevent circular modifications
- Cannot monitor build artifacts
- Must ignore temporary files
- Should debounce rapid changes
- Must maintain modification audit trail

## Architectural Decisions

### Decision: Loop Prevention Strategy
**Chosen**: Modification tagging with cooldown periods
**Alternatives Considered**:
- Lock files: Can cause deadlocks
- Process isolation: Too complex
- Disable during modification: Might miss important changes
**Rationale**: Tagging allows tracking modification source while cooldown prevents loops

### Decision: Safety Boundaries
**Chosen**: Whitelist of modifiable paths
**Alternatives Considered**:
- Blacklist critical files: Too risky
- No restrictions: Dangerous
- Read-only mode: Too limiting
**Rationale**: Whitelist ensures only safe paths can be modified

## Implementation Blueprint

### File Structure
Extend existing monitor module in auto-dev-core/src/monitor/
- self_monitor.rs - Self-monitoring logic
- modification_guard.rs - Safety checks
- loop_detector.rs - Detect modification loops
- audit_trail.rs - Track all self-modifications

### Key Components
1. **SelfMonitor** - Specialized monitor for own codebase
2. **ModificationGuard** - Validates safe modifications
3. **LoopDetector** - Prevents infinite loops
4. **AuditTrail** - Records all self-modifications

### Implementation Tasks (in order)
1. Create self_monitor submodule
2. Configure notify watcher for project root
3. Implement path filtering for source files only
4. Add modification source tagging
5. Build loop detection algorithm
6. Create safety validation rules
7. Implement cooldown mechanism
8. Add audit trail logging
9. Integrate with existing monitor module
10. Write safety tests

## Validation Gates

```bash
# Test self-monitoring
cargo test monitor::self_monitor

# Verify loop prevention
cargo run -- self-monitor --test-loop-prevention

# Check safety boundaries
cargo run -- self-monitor --verify-boundaries

# Test with actual file changes
echo "// test" >> src/test_file.rs && cargo run -- self-monitor --detect
```

## Success Criteria
- Detects all changes in src/ directory
- Prevents modification loops within 3 iterations
- Ignores target/ and other build artifacts
- Responds to changes within 500ms
- Maintains complete audit trail

## Known Patterns and Conventions
- Reuse existing FileWatcher from monitor module
- Follow debouncing patterns from PRP 100
- Use existing event queue infrastructure
- Match ignore patterns from .gitignore

## Common Pitfalls to Avoid
- Don't monitor .git directory
- Avoid watching files being written
- Remember to handle file renames
- Don't trigger on your own modifications
- Consider filesystem events during builds

## Dependencies Required
- Already available: notify, ignore
- No new dependencies needed

## Confidence Score: 9/10
This builds directly on existing monitoring infrastructure with clear safety boundaries. Main complexity is loop prevention which is well-understood problem.