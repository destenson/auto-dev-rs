# PRP: Claude Binary Detection and Validation

**Status**: NOT STARTED  
**Priority**: Critical (P1)  
**Estimated Time**: 2 hours

## Overview
Implement robust detection and validation of the Claude Code CLI binary, handling various installation methods and platforms. This is the foundation for all Claude Code integration.

## Context and Background
Claude Code can be installed via npm, native installers, or standalone binaries. The detection must handle different locations and validate the binary is functional and compatible.

### Research References
- Claude Code documentation: https://docs.anthropic.com/en/docs/claude-code
- Platform-specific installation: https://docs.anthropic.com/en/docs/claude-code/quickstart
- Existing pattern in `auto-dev-core/src/llm/cli_tools.rs` lines 15-38

## Requirements

### Primary Goals
1. Detect Claude binary across platforms (Windows/Mac/Linux)
2. Validate binary version and functionality
3. Cache detection results for performance
4. Handle multiple installation methods
5. Provide clear error messages when not found

### Technical Constraints
- Must check common installation paths
- Should test with --version flag
- Must handle both claude and claude.cmd on Windows
- Should detect via PATH environment variable

## Architectural Decisions

### Decision: Detection Strategy
**Chosen**: Multi-path search with caching
**Rationale**: Balances thoroughness with performance

### Decision: Version Validation
**Chosen**: Parse --version output
**Rationale**: Most reliable way to ensure compatibility

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/claude_code/`:
- Create `detector.rs` - Binary detection logic
- Create `mod.rs` - Module exports
- Update `llm/mod.rs` - Add claude_code module

### Key Components
1. **ClaudeDetector** - Main detection struct
2. **ClaudeLocation** - Stores path and version info
3. **DetectionCache** - Caches results for session
4. **PlatformDetector** - Platform-specific logic

### Implementation Tasks (in order)
1. Create claude_code module structure
2. Implement PATH environment search
3. Add platform-specific common paths
4. Implement version parsing from --version
5. Add detection caching mechanism
6. Create comprehensive error types
7. Add unit tests with mocked commands
8. Document detection precedence

## Detection Paths

Search order (first found wins):
1. PATH environment variable
2. NPM global installation
   - Windows: `%APPDATA%\npm\claude.cmd`
   - Unix: `/usr/local/bin/claude`
3. Native installation
   - Windows: `%LOCALAPPDATA%\Programs\Claude\claude.exe`
   - Mac: `/Applications/Claude.app/Contents/MacOS/claude`
   - Linux: `/opt/claude/claude`
4. User-specified custom path

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::claude_code::detector

# Integration test with real binary
cargo test --package auto-dev-core --lib llm::claude_code::detector::integration -- --ignored

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Detects Claude on all major platforms
- Caches results to avoid repeated searches
- Provides helpful error when not found
- Validates version compatibility
- Sub-100ms detection after first run

## Known Patterns and Conventions
- Follow existing CLI detection pattern from `cli_tools.rs`
- Use `which` crate for PATH searching
- Cache using `once_cell::sync::Lazy`
- Return structured errors with suggestions

## Common Pitfalls to Avoid
- Don't assume installation location
- Handle permission errors gracefully
- Test with spaces in paths
- Check both .cmd and .exe on Windows
- Don't cache negative results indefinitely

## Testing Approach
- Mock Command execution for unit tests
- Test with missing binary
- Test with invalid binary (wrong version)
- Test cache invalidation
- Integration test with real Claude if available

## Version Compatibility
Minimum supported Claude Code version: 1.0.0
Parse version string like "Claude Code 1.2.3"
Handle beta/dev versions appropriately

## Confidence Score: 9/10
Clear implementation path following existing patterns in codebase.