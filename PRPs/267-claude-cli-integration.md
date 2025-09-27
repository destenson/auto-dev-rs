# PRP: Claude Commands CLI Integration

**Status**: NOT STARTED  
**Priority**: Medium (P2)  
**Estimated Time**: 3-4 hours

## Overview
Integrate Claude commands into auto-dev's CLI system, making them available as subcommands and enabling execution of user-defined commands from .claude/commands/.

## Context and Background
Auto-dev has an existing CLI structure using clap. Claude commands should be dynamically added as subcommands, allowing users to run custom commands defined in their .claude/commands/ directory directly from the auto-dev CLI.

### Research References
- CLI structure: auto-dev/src/cli/commands/
- Command pattern: auto-dev/src/cli/mod.rs
- Clap dynamic commands: https://docs.rs/clap/latest/clap/builder/struct.Command.html#method.subcommand

## Requirements

### Primary Goals
1. Dynamically register Claude commands in CLI
2. Route command execution to Claude handlers
3. Pass arguments correctly to commands
4. Display command help from markdown
5. Handle command errors gracefully

### Technical Constraints
- Must work with existing clap structure
- Should not slow down CLI startup
- Must validate command availability
- Should provide helpful error messages

## Architectural Decisions

### Decision: Registration Strategy
**Chosen**: Dynamic subcommand registration
**Rationale**: Flexible, allows user customization

### Decision: Execution Model
**Chosen**: Delegate to command executor
**Rationale**: Separation of concerns

## Implementation Blueprint

### File Structure
Create/Modify:
- Create `auto-dev/src/cli/claude_commands.rs` - Claude command handling
- Update `auto-dev/src/cli/mod.rs` - Add Claude command integration
- Create `auto-dev-core/src/claude/command_executor.rs` - Execution logic

### Key Components
1. **ClaudeCommandHandler** - CLI handler for Claude commands
2. **DynamicCommandBuilder** - Builds clap commands dynamically
3. **CommandExecutor** - Executes Claude commands
4. **ArgumentParser** - Parses CLI args for commands
5. **HelpFormatter** - Formats command help from markdown

### Implementation Tasks (in order)
1. Create ClaudeCommandHandler in CLI module
2. Implement dynamic command discovery at startup
3. Build clap subcommands from ClaudeCommands
4. Create argument parsing from command definitions
5. Implement help text extraction from markdown
6. Add command routing to executor
7. Handle execution errors with helpful messages
8. Add shell completion support for commands

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev
cargo test --package auto-dev --bin auto-dev

# Test command registration
cargo run --package auto-dev -- --help

# Test command execution
cargo run --package auto-dev -- <claude-command> --help

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Claude commands appear in CLI help
- Commands execute when invoked
- Arguments pass through correctly
- Help text displays from markdown
- Error messages are helpful

## Dependencies Required
Already in project:
- clap for CLI parsing
- Command infrastructure
- Error handling

## Known Patterns and Conventions
- Use clap's Command builder
- Follow existing command patterns
- Use anyhow for error handling
- Provide detailed help text

## Common Pitfalls to Avoid
- Don't slow down CLI startup
- Handle missing commands gracefully
- Validate command names for CLI
- Consider shell completion
- Test with various shells

## Testing Approach
- Test command discovery
- Test argument passing
- Test help text generation
- Test error scenarios
- Integration tests with CLI

## Confidence Score: 7/10
Dynamic command registration adds complexity to CLI structure.