# PRP: Claude Code CLI Integration

**Status**: NOT STARTED  
**Priority**: Medium (P3)  
**Estimated Time**: 2-3 hours

## Overview
Integrate with Claude Code CLI tool to leverage its advanced features beyond API access, including terminal integration, context management, and multi-agent capabilities.

## Context and Background
Claude Code CLI provides capabilities beyond the standard API: automatic context pulling, CLAUDE.md file support, parallel processing, and deep terminal integration. This PRP explores integration strategies.

### Research References
- Claude Code docs: https://docs.claude.com/en/docs/claude-code
- Best practices: https://www.anthropic.com/engineering/claude-code-best-practices
- GitHub integration: https://github.com/hesreallyhim/awesome-claude-code

## Requirements

### Primary Goals
1. Detect and use Claude Code if available
2. Leverage CLAUDE.md context files
3. Support parallel agent execution
4. Enable terminal pipeline integration
5. Fallback to API when unavailable

### Technical Constraints
- Must detect claude-code installation
- Should preserve CLI functionality
- Must handle both modes (CLI/API)
- Should support configuration

## Architectural Decisions

### Decision: Integration Strategy
**Chosen**: Hybrid with detection
**Rationale**: Use CLI when available, API as fallback

### Decision: Context Management
**Chosen**: CLAUDE.md generation
**Rationale**: Leverages Claude Code's auto-context

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/`:
- Create `claude_cli.rs` - CLI integration
- Create `claude_context.rs` - CLAUDE.md management
- Update `claude.rs` - Add CLI detection

### Key Components
1. **ClaudeCliDetector** - Detect installation
2. **ClaudeCliExecutor** - Execute via CLI
3. **ClaudeContextManager** - Manage CLAUDE.md
4. **ClaudeHybridProvider** - CLI + API provider

### Implementation Tasks (in order)
1. Implement CLI detection logic
2. Create command executor
3. Build CLAUDE.md generator
4. Implement response parser
5. Add fallback to API
6. Create parallel agent support
7. Test integration patterns

## CLI Detection and Usage

Check for Claude Code:
```bash
which claude-code || which claude
claude --version
```

Execute via CLI:
```bash
echo "prompt" | claude --model opus-4.1
claude --file context.md "generate function"
```

## CLAUDE.md Generation

Generate context file with:
- Project structure
- Relevant code snippets
- Task requirements
- Constraints and patterns
- Previous context

## Parallel Agent Execution

Enable patterns like:
```bash
# Writer and reviewer in parallel
claude "write function" > function.rs &
claude "review code" < function.rs &
wait
```

## Configuration

```toml
[claude_cli]
enabled = true
executable = "claude-code"  # or "claude"
default_model = "opus-4.1"
use_claude_md = true
parallel_agents = true
fallback_to_api = true
```

## Validation Gates

```bash
# Check CLI availability
which claude-code && echo "CLI available"

# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::claude_cli

# Test CLI execution
cargo test --lib llm::claude_cli::exec -- --ignored

# Test context generation
cargo test --lib llm::claude_context -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Detects Claude Code installation
- Executes via CLI successfully
- CLAUDE.md generation works
- Fallback to API seamless
- Parallel execution functions

## Dependencies Required
No new crates needed:
- Use std::process for execution
- Existing tokio for async

## Known Patterns and Conventions
- Check PATH for executable
- Parse command output
- Handle process errors
- Stream stdout/stderr
- Respect exit codes

## Common Pitfalls to Avoid
- Don't assume CLI installed
- Handle permission issues
- Parse errors gracefully
- Timeout long operations
- Clean up temp files

## Unique CLI Features

Features to leverage:
- **Auto-context**: CLAUDE.md automatically included
- **Piping**: Unix-style pipeline support
- **Parallel**: Multiple agents simultaneously
- **Local**: Works offline after setup
- **Terminal**: Direct terminal integration

## CLAUDE.md Best Practices

Include in CLAUDE.md:
- Project overview
- Architecture decisions
- Code style guide
- Common patterns
- Current task context
- Previous decisions

## Testing Approach
- Mock CLI execution
- Test with/without CLI
- Verify fallback works
- Test context generation
- Benchmark CLI vs API

## Performance Considerations
- CLI may be faster (no HTTP)
- Parallel agents improve throughput
- Context caching important
- Process spawn overhead
- Memory usage with parallel

## Use Cases

Best for:
- Complex refactoring
- Multi-file changes
- Code review workflows
- Parallel generation
- Offline development

## Confidence Score: 7/10
Integration complexity with external CLI. Value depends on Claude Code availability.