# PRP: Smart Project Initialization with Native Tools

**Status**: NOT STARTED  
**Priority**: High (P1)  
**Estimated Time**: 2-3 hours

## Overview
Implement the `auto-dev init` command enhancement that detects project type from instructions and uses native ecosystem tools (cargo, deno, uv, dotnet, etc.) to initialize projects. Falls back to creating basic auto-dev configuration when project type is ambiguous.

## Context and Background
The current init command only creates `.auto-dev` directories. This PRP extends it to intelligently initialize complete projects using native tooling based on instruction files or command-line specifications.

### Research References
- Cargo init: https://doc.rust-lang.org/cargo/commands/cargo-init.html
- UV init: https://docs.astral.sh/uv/guides/projects/#creating-a-new-project
- Deno init: https://docs.deno.com/runtime/manual/getting_started/first_steps
- Dotnet new: https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-new

## Requirements

### Primary Goals
1. Detect project type from instruction string/file
2. Execute appropriate native tool for initialization
3. Create `.auto-dev` configuration alongside native project
4. Fallback to generic initialization when type unclear

### Technical Constraints
- Use existing std::process::Command patterns
- No external LLM dependencies
- Leverage existing loop_control::init_project as base
- Support both CLI strings and instruction files

## Architectural Decisions

### Decision: Project Type Detection
**Chosen**: Keyword-based detection with confidence scoring
**Rationale**: Simple, fast, no LLM needed, extensible

### Decision: Tool Execution
**Chosen**: Direct process spawning via std::process::Command
**Rationale**: Already used throughout codebase, reliable

## Implementation Blueprint

### File Structure
Update in `auto-dev/src/cli/commands/`:
- Enhance `loop_control.rs` init_project function
- Create `init/detector.rs` - Project type detection
- Create `init/executor.rs` - Tool execution logic
- Create `init/instructions.rs` - Instruction parsing

### Key Components
1. **ProjectDetector** - Analyzes instructions for project type
2. **ToolExecutor** - Spawns native init tools
3. **InstructionParser** - Reads instruction files/strings
4. **FallbackInitializer** - Generic project setup

### Implementation Tasks (in order)
1. Create instruction parser for files and strings
2. Implement keyword-based project type detection
3. Add tool executor with cargo, uv, deno, dotnet support
4. Integrate with existing init_project function
5. Add fallback for ambiguous projects
6. Create instruction file in initialized projects

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev
cargo test --package auto-dev --lib init

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings

# Integration test - Rust project
auto-dev init "create a rust cli tool for parsing json"

# Integration test - Python project  
auto-dev init "build a python web scraper with beautifulsoup"

# Integration test - Fallback
auto-dev init "make something cool"
```

## Success Criteria
- Correctly detects Rust projects and runs `cargo init`
- Correctly detects Python projects and runs `uv init` 
- Correctly detects Deno projects and runs `deno init`
- Falls back gracefully for ambiguous instructions
- Creates `.auto-dev/instructions.md` with original request
- Works with both file and string inputs

## Dependencies Required
None - uses only standard library and existing patterns

## Known Patterns and Conventions
- Follow existing command module structure
- Use tokio::process for async execution
- Create .auto-dev directory structure per existing pattern
- Use anyhow::Result for error handling

## Common Pitfalls to Avoid
- Don't over-engineer detection logic
- Check tool availability before execution
- Handle missing tools gracefully
- Don't block on long-running inits
- Preserve original instruction text exactly

## Testing Approach
- Unit test detector with various instruction strings
- Test executor with mock commands
- Integration test full flow with real tools
- Test fallback scenarios
- Verify instruction file creation

## Confidence Score: 9/10
Clear requirements, uses existing patterns, no complex dependencies.