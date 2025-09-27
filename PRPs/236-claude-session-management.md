# PRP: Claude Session and Conversation Management

**Status**: NOT STARTED  
**Priority**: High (P2)  
**Estimated Time**: 3 hours

## Overview
Implement session management for Claude Code CLI to maintain conversation context across multiple non-interactive calls using --session-id and --continue flags.

## Context and Background
Claude Code supports conversation continuity through session IDs and the --continue flag. This enables maintaining context across multiple prompts, essential for complex multi-turn interactions.

### Research References
- Session management in `auto-dev-core/src/llm/token_manager.rs` lines 177-194
- Claude session flags: --session-id, --continue, --resume
- UUID generation and validation patterns

## Requirements

### Primary Goals
1. Generate and track session UUIDs
2. Implement session persistence
3. Support --continue for last conversation
4. Map to internal ConversationManager
5. Handle session cleanup and expiry

### Technical Constraints
- Session IDs must be valid UUIDs
- Should persist sessions to disk
- Must handle concurrent sessions
- Clean up old sessions automatically

## Architectural Decisions

### Decision: Storage Method
**Chosen**: File-based with JSON serialization
**Rationale**: Simple, portable, matches Claude's approach

### Decision: Session Lifetime
**Chosen**: 24-hour expiry with LRU eviction
**Rationale**: Balances utility with resource usage

## Implementation Blueprint

### File Structure
Create in `auto-dev-core/src/llm/claude_code/`:
- Create `session.rs` - Session management logic
- Create `persistence.rs` - Session storage
- Update `mod.rs` - Export session types

### Key Components
1. **SessionManager** - Manages active sessions
2. **Session** - Individual conversation state
3. **SessionStore** - Persistence layer
4. **SessionConfig** - Configuration options

### Implementation Tasks (in order)
1. Create Session struct with UUID and metadata
2. Implement SessionStore with file persistence
3. Add session creation and retrieval
4. Implement --continue flag support
5. Add session expiry and cleanup
6. Map to ConversationManager
7. Add concurrent session handling
8. Write tests for session lifecycle

## Session Structure

Store per session:
- session_id: UUID
- created_at: timestamp
- last_used: timestamp
- message_count: usize
- model: String (if specified)
- context_tokens: approximate count

## Persistence Format

JSON file per session:
```json
{
  "session_id": "uuid-here",
  "created_at": "2024-01-01T00:00:00Z",
  "last_used": "2024-01-01T00:00:00Z",
  "message_count": 3,
  "model": "claude-3-5-sonnet",
  "metadata": {}
}
```

Location: `~/.auto-dev/claude-sessions/`

## Command Integration

Session-aware command building:
- New session: Generate UUID, pass --session-id
- Continue: Use --continue (auto-finds last)
- Resume specific: --resume with session ID
- List sessions: Internal management command

## Session Lifecycle

1. Create: Generate UUID on first use
2. Use: Update last_used timestamp
3. Continue: Find most recent session
4. Expire: Remove after 24 hours unused
5. Cleanup: Limit to 100 sessions max

## Validation Gates

```bash
# Build and test
cargo build --package auto-dev-core
cargo test --package auto-dev-core --lib llm::claude_code::session

# Test persistence
cargo test --package auto-dev-core --lib llm::claude_code::session::persistence

# Test cleanup
cargo test --package auto-dev-core --lib llm::claude_code::session::cleanup

# Format and lint
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Success Criteria
- Creates valid UUID session IDs
- Persists sessions between runs
- Implements --continue correctly
- Cleans up old sessions
- Thread-safe for concurrent use

## Dependencies Required
Already in Cargo.toml:
- uuid crate for UUID generation
- serde for JSON serialization
- chrono for timestamps
- tokio for async file I/O

## Known Patterns and Conventions
- Use ConversationManager pattern from token_manager.rs
- Store in XDG-compliant directories
- Use atomic file operations
- Implement Default trait for config

## Common Pitfalls to Avoid
- Don't lose sessions on crash
- Handle corrupted session files
- Test with many concurrent sessions
- Clean up lock files
- Handle permission errors

## Testing Approach
- Unit tests for session CRUD
- Test persistence across restarts
- Test concurrent session access
- Test cleanup logic
- Test session recovery

## Integration Points

Connect with:
- CommandBuilder adds session flags
- Executor passes session ID
- Parser may extract session info
- Provider tracks conversation state

## Confidence Score: 8/10
Clear requirements with established patterns in codebase.