# PRP: Claude Command Execution Wrapper

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 3 hours

## Overview
Implement a robust command execution wrapper for Claude Code CLI in non-interactive mode (--print flag), handling process spawning, timeout management, and error handling.

## Context and Background
This PRP focuses exclusively on non-interactive execution using the --print flag, which is essential for programmatic use. Interactive mode support is out of scope.

### Research References
- Process handling pattern in `auto-dev-core/src/mcp/transport.rs` lines 26-35
- Existing basic implementation in `auto-dev-core/src/llm/cli_tools.rs` lines 40-56
- Tokio process documentation: https://docs.rs/tokio/latest/tokio/process/

## Requirements

### Primary Goals
1. Execute Claude with proper argument handling
2. Capture stdout, stderr, and exit codes
3. Implement timeout mechanism
4. Handle large outputs efficiently
5. Support both sync and async execution

### Technical Constraints
- Must use --print flag for non-interactive mode
- Should handle outputs up to 10MB
- Default timeout of 30 seconds, configurable
- Must properly escape shell arguments

## Architectural Decisions

### Decision: Async vs Sync
**Chosen**: Async by default with sync wrapper
**Rationale**: Aligns with tokio usage in codebase

### Decision: Output Handling
**Chosen**: Streaming with buffering
**Rationale**: Handles large outputs without memory issues

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/claude_code/`:
- Create `executor.rs` - Command execution logic
- Create `command_builder.rs` - Build Claude commands
- Update `mod.rs` - Export executor types

### Key Components
1. **ClaudeExecutor** - Main execution struct
2. **CommandBuilder** - Builds command with arguments
3. **ExecutionResult** - Captures output and metadata
4. **ExecutionOptions** - Timeout, env vars, etc.

### Implementation Tasks (in order)
1. Create CommandBuilder for argument construction
2. Implement basic process spawning with tokio
3. Add timeout handling with tokio::time::timeout
4. Implement output streaming and buffering
5. Add proper error handling and types
6. Create sync wrapper for non-async contexts
7. Add retry logic for transient failures
8. Write comprehensive tests

## Command Building

Base command structure:
- Binary path from detector
- --print flag for non-interactive
- Optional: --output-format, --model, --fallback-model
- Optional: --session-id for conversation tracking
- Prompt as final argument

## Error Handling

Error types to handle:
- Binary not found (use detector first)
- Process spawn failure
- Timeout exceeded
- Non-zero exit codes
- Output parsing errors
- Permission denied

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::claude_code::executor

# Test timeout handling
cargo test --package auto-dev-core --lib llm::claude_code::executor::timeout -- --ignored

# Test with mock binary
cargo test --package auto-dev-core --lib llm::claude_code::executor::mock

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Executes Claude commands reliably
- Handles timeouts gracefully
- Captures all output types
- Provides detailed error context
- Works on all platforms

## Dependencies Required
Already in Cargo.toml:
- tokio with process feature
- anyhow for error handling
- tracing for logging

## Known Patterns and Conventions
- Use tokio::process::Command
- Follow error handling from `mcp/transport.rs`
- Use tracing for debug output
- Implement Drop for cleanup

## Common Pitfalls to Avoid
- Don't block the tokio runtime
- Handle zombie processes
- Escape shell special characters
- Test with Unicode in prompts
- Clean up on cancellation

## Testing Approach
- Unit tests with mock processes
- Test various exit codes
- Test timeout scenarios
- Test large output handling
- Integration test with real Claude

## Output Size Limits
- Stdout: 10MB max
- Stderr: 1MB max
- Truncate with warning if exceeded
- Consider streaming for larger outputs

## Confidence Score: 9/10
Well-understood problem with clear patterns in existing codebase.