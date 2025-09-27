# PRP: CLI Generate Command Integration

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 2 hours

## Overview
Wire together the complete `auto-dev generate` command that accepts instructions (string or file), initializes projects, generates code, and runs the build-fix loop. This is the user-facing integration point.

## Context and Background
This PRP connects all the previous components into a cohesive CLI command. It updates the existing placeholder generate command to provide full project creation and implementation functionality.

### Research References
- Existing generate command: `auto-dev/src/cli/commands/generate.rs`
- CLI structure: `auto-dev/src/cli/app.rs`
- Similar to parse command implementation pattern

## Requirements

### Primary Goals
1. Accept instruction string or file path
2. Create project with appropriate tooling
3. Generate initial implementation
4. Run build-fix iterations
5. Provide progress feedback

### Technical Constraints
- Integrate with existing CLI structure
- Use tokio for async operations
- Provide clear progress indicators
- Support dry-run mode

## Architectural Decisions

### Decision: Command Interface
**Chosen**: Flexible positional argument with options
**Rationale**: Natural CLI UX, similar to other tools

### Decision: Progress Feedback
**Chosen**: indicatif progress bars
**Rationale**: Already in dependencies, good UX

## Implementation Blueprint

### File Structure
Update in `auto-dev/src/cli/commands/`:
- Rewrite `generate.rs` - Full implementation
- Create `generate/handler.rs` - Orchestration logic
- Create `generate/progress.rs` - Progress reporting

### Key Components
1. **GenerateCommand** - CLI argument handling
2. **GenerateHandler** - Orchestrates full pipeline
3. **ProgressReporter** - User feedback
4. **GenerateResult** - Success/failure reporting

### Implementation Tasks (in order)
1. Update CLI arguments in app.rs
2. Parse instruction from string or file
3. Initialize project with detected type
4. Generate code implementation
5. Run build-fix loop with progress
6. Report results to user

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev
cargo test --package auto-dev commands::generate

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# CLI tests
auto-dev generate "create a hello world in rust"
auto-dev generate instructions.md --output my-project
auto-dev generate spec.yaml --dry-run
auto-dev generate --help
```

## Success Criteria
- Accepts instruction strings directly
- Reads instruction files (md, yaml, txt)
- Creates working projects
- Shows progress during generation
- Handles errors gracefully
- Supports common CLI patterns

## Dependencies Required
- indicatif (already available)
- All components from previous PRPs
- No new external dependencies

## Known Patterns and Conventions
- Follow existing command structure
- Use anyhow::Result for errors
- Implement async execution pattern
- Match parse command conventions

## Common Pitfalls to Avoid
- Blocking the runtime
- Poor error messages
- Missing progress feedback
- Not handling Ctrl+C
- Forgetting dry-run mode

## Testing Approach
- Unit test argument parsing
- Mock component integration
- End-to-end CLI tests
- Test various input formats
- Verify error handling

## Confidence Score: 9/10
Clear integration path, follows existing patterns, well-defined scope.