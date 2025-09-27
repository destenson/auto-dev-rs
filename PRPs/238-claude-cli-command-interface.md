# PRP: Claude-Compatible CLI Command Interface

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 3 hours

## Overview
Add a Claude-compatible command-line interface to auto-dev that mimics essential Claude Code CLI commands, allowing users familiar with Claude to use auto-dev with similar syntax.

## Context and Background
Users familiar with Claude Code CLI expect certain command patterns. By providing compatible commands, we reduce the learning curve and enable drop-in replacement for basic use cases.

### Research References
- Claude CLI commands from the provided help output
- Existing CLI structure in `auto-dev/src/cli/app.rs`
- Command pattern in `auto-dev/src/cli/commands/`

## Requirements

### Primary Goals
1. Add claude-compatible command aliases
2. Support --print flag for non-interactive output
3. Map Claude flags to auto-dev functionality
4. Provide helpful migration messages
5. Support common Claude workflows

### Technical Constraints
- Must not break existing auto-dev commands
- Should detect when used in Claude mode
- Map to existing provider infrastructure
- Maintain backward compatibility

## Architectural Decisions

### Decision: Implementation Approach
**Chosen**: Add new command with Claude-style args
**Rationale**: Clean separation, no conflicts

### Decision: Flag Mapping
**Chosen**: Map Claude flags to provider options
**Rationale**: Reuse existing infrastructure

## Implementation Blueprint

### File Structure
Create in `auto-dev/src/cli/commands/`:
- Create `claude.rs` - Claude compatibility command
- Update `mod.rs` - Export claude command
- Update `app.rs` - Add Claude command variant

### Key Components
1. **ClaudeCommand** - Main command struct
2. **ClaudeArgs** - Argument parsing
3. **FlagMapper** - Map Claude to auto-dev flags
4. **ResponseFormatter** - Format output Claude-style

### Implementation Tasks (in order)
1. Create ClaudeCommand with clap structure
2. Add essential Claude flags (--print, --model)
3. Map to internal provider calls
4. Implement output formatting
5. Add session flag support
6. Create compatibility warnings
7. Add command aliases
8. Write CLI tests

## Command Structure

Primary command:
```bash
auto-dev claude [OPTIONS] [PROMPT]
```

Essential flags to support:
- `-p, --print` - Non-interactive output
- `--model <model>` - Model selection
- `--output-format <format>` - Output format
- `-c, --continue` - Continue last conversation
- `--session-id <uuid>` - Specific session
- `-v, --verbose` - Debug output

## Flag Mapping

Map Claude flags to internal:
- `--print` → Use provider directly
- `--model` → Set provider model
- `--output-format json` → Return structured
- `--continue` → Load last session
- `--session-id` → Use specific session

## Output Formatting

Match Claude output style:
- Text format: Direct response
- JSON format: Structured with metadata
- Include model info if verbose
- Show token usage if available

## Migration Helpers

When Claude-specific features used:
- Suggest auto-dev equivalent
- Explain differences
- Provide migration guide link
- Offer to run equivalent command

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev
cargo test --package auto-dev --bin auto-dev

# Test CLI parsing
cargo test --package auto-dev --lib cli::commands::claude

# Test with real commands
./target/debug/auto-dev claude --print "Hello"
./target/debug/auto-dev claude --help

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Parses Claude-style commands correctly
- Maps to appropriate providers
- Produces expected output format
- Provides helpful migration info
- Maintains compatibility

## Dependencies Required
Already in Cargo.toml:
- clap for argument parsing
- Other CLI dependencies

## Known Patterns and Conventions
- Follow command structure from existing commands
- Use clap derive macros
- Return anyhow::Result
- Log with tracing

## Common Pitfalls to Avoid
- Don't break existing commands
- Handle missing provider gracefully
- Test with various flag combinations
- Preserve prompt quotes/escaping
- Document differences clearly

## Testing Approach
- Unit test argument parsing
- Test flag combinations
- Integration test with provider
- Test output formats
- Test error messages

## Example Usage

Compatible commands:
```bash
# Basic prompt
auto-dev claude --print "Generate a README"

# With model selection
auto-dev claude --model sonnet --print "Explain this code"

# Continue conversation
auto-dev claude --continue --print "Add error handling"

# JSON output
auto-dev claude --output-format json --print "List improvements"
```

## Help Text

Provide Claude-familiar help:
```
Claude-compatible interface for auto-dev

Usage: auto-dev claude [OPTIONS] [PROMPT]

Arguments:
  [PROMPT]  Your prompt to Claude

Options:
  -p, --print              Non-interactive output
  --model <MODEL>          Model selection
  --output-format <FORMAT> Output format (text|json)
  -c, --continue           Continue last conversation
  --session-id <UUID>      Use specific session
  -h, --help               Print help
```

## Confidence Score: 8/10
Clear requirements with straightforward implementation using existing patterns.