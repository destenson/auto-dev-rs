# PRP: End-to-End Project Generation Orchestration

**Status**: NOT STARTED  
**Priority**: Medium (P2)  
**Estimated Time**: 2-3 hours

## Overview
Create the orchestration layer that ties together project initialization, code generation, and iterative improvement into a smooth end-to-end flow. This ensures all components work together seamlessly.

## Context and Background
While individual components handle specific tasks, this orchestrator manages the complete flow from instruction to working project. It handles state management, error recovery, and pipeline coordination.

### Research References
- Existing loop_control patterns: `auto-dev/src/cli/commands/loop_control.rs`
- Incremental executor: `auto-dev-core/src/incremental/executor.rs`
- Pipeline patterns in synthesis module

## Requirements

### Primary Goals
1. Coordinate multi-step generation pipeline
2. Manage state between phases
3. Handle partial failures gracefully
4. Support resumable generation

### Technical Constraints
- Must maintain generation context
- Support checkpointing for long runs
- Clean up on failure
- Work with async operations

## Architectural Decisions

### Decision: State Management
**Chosen**: File-based checkpointing
**Rationale**: Resumable, debuggable, simple

### Decision: Pipeline Architecture
**Chosen**: Sequential phases with rollback
**Rationale**: Clear flow, easier error handling

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/orchestration/`:
- `mod.rs` - Public interface
- `pipeline.rs` - Main orchestration logic
- `state.rs` - State management
- `checkpoint.rs` - Progress checkpointing

### Key Components
1. **GenerationPipeline** - Main orchestrator
2. **PipelineState** - Tracks progress
3. **CheckpointManager** - Saves/restores state
4. **PhaseExecutor** - Runs pipeline phases

### Implementation Tasks (in order)
1. Define pipeline phases and transitions
2. Create state management structure
3. Implement checkpoint save/restore
4. Build phase execution with rollback
5. Add resume capability
6. Integrate with CLI generate command

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core orchestration

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Test full pipeline
auto-dev generate "complex project" --verbose
# Interrupt and resume
auto-dev generate --resume .auto-dev/checkpoint.json

# Test rollback on failure
auto-dev test-orchestration --fail-at-phase 3
```

## Success Criteria
- Completes simple projects end-to-end
- Resumes from checkpoints correctly
- Rolls back failed phases
- Provides detailed progress
- Handles interruptions gracefully
- Maintains generation context

## Dependencies Required
- serde_json (for checkpointing)
- Already available dependencies
- No new external requirements

## Known Patterns and Conventions
- Similar to incremental executor patterns
- Use existing state storage approaches
- Follow async execution patterns
- Match loop_control architecture

## Common Pitfalls to Avoid
- Losing state on crashes
- Not cleaning up temp files
- Poor phase boundaries
- Missing rollback logic
- Overly complex state

## Testing Approach
- Test each phase in isolation
- Test phase transitions
- Simulate failures at each phase
- Test checkpoint/resume
- Verify cleanup logic

## Confidence Score: 8/10
Follows established patterns, clear phase boundaries, testable components.