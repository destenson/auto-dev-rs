# PRP: Filesystem Monitoring and Change Detection

## Overview
Implement a robust filesystem monitoring system that watches for changes in documentation, specifications, tests, and code files. This is the sensory system that triggers the autonomous development cycle.

## Context and Background
The system needs to continuously monitor the project directory for changes to specifications, documentation (README.md, docs/*, SPEC.md), test files, and existing code. When changes are detected, it triggers analysis to determine what code needs to be implemented or updated.

### Research References
- notify crate for file watching: https://docs.rs/notify/latest/notify/
- FSEvents (macOS), inotify (Linux), ReadDirectoryChangesW (Windows)
- Debouncing strategies for file system events
- gitignore parsing with ignore crate: https://docs.rs/ignore/latest/ignore/

## Requirements

### Primary Goals
1. Monitor all relevant file types in project directory
2. Detect and categorize changes (spec, doc, test, code)
3. Implement intelligent debouncing for rapid changes
4. Track file dependencies and relationships
5. Maintain change history and context

### Technical Constraints
- Must work across Windows, Linux, macOS
- Handle large projects efficiently
- Respect .gitignore and similar exclusions
- Minimize CPU usage during idle periods
- Handle file renames and moves correctly

## Implementation Blueprint

### File Structure
```
src/
├── monitor/
│   ├── mod.rs           # Monitor module exports
│   ├── watcher.rs       # Core file system watcher
│   ├── classifier.rs    # File type classification
│   ├── debouncer.rs     # Event debouncing logic
│   ├── analyzer.rs      # Change impact analysis
│   └── queue.rs         # Change queue management
```

### Key Components
1. **FileWatcher**: Monitors filesystem events
2. **ChangeClassifier**: Categorizes file changes
3. **EventDebouncer**: Aggregates rapid changes
4. **ChangeAnalyzer**: Determines impact of changes
5. **ChangeQueue**: Prioritizes processing order

### File Categories to Monitor
```rust
enum FileCategory {
    Specification,     // SPEC.md, requirements.*, design.*
    Documentation,     // README.md, docs/*, *.md
    Test,             // *_test.rs, test_*.rs, tests/*
    Implementation,   // *.rs, *.py, *.js (existing code)
    Configuration,    // Cargo.toml, package.json, etc.
    Schema,           // *.yaml, *.json schema files
    Example,          // examples/*, *.example.*
}
```

### Implementation Tasks (in order)
1. Add notify, ignore, and walkdir to Cargo.toml
2. Create monitor module structure
3. Implement basic file watcher with notify
4. Add gitignore respect using ignore crate
5. Build file classifier based on patterns
6. Implement debouncing with configurable delay
7. Create change analyzer for impact assessment
8. Build prioritized change queue
9. Add dependency tracking between files
10. Implement change history storage
11. Create monitoring statistics and reporting
12. Add hot-reload configuration support

## Monitoring Strategy

### Watch Patterns
```toml
[monitoring.patterns]
specifications = [
    "SPEC.md",
    "SPECIFICATION.md",
    "requirements/*.md",
    "design/*.md",
    "architecture/*.md"
]

documentation = [
    "README.md",
    "docs/**/*.md",
    "*.md",
    "API.md"
]

tests = [
    "**/*_test.rs",
    "**/test_*.rs",
    "tests/**/*",
    "**/*.test.js",
    "**/*.spec.ts"
]

schemas = [
    "schemas/*.json",
    "**/*.schema.yaml",
    "api/*.yaml"
]
```

### Debouncing Logic
```rust
// Pseudo-code for debouncing
struct Debouncer {
    delay: Duration,        // e.g., 500ms
    pending: HashMap<PathBuf, ChangeEvent>,
    
    fn on_event(&mut self, event: Event) {
        // Aggregate events for same file
        // Reset timer on new events
        // Emit when timer expires
    }
}
```

## Change Analysis

### Impact Assessment
1. **Specification Change**: Trigger full implementation review
2. **Test Addition**: Generate code to pass test
3. **Documentation Update**: Update corresponding code/comments
4. **Example Addition**: Generate similar implementation
5. **Schema Change**: Update data structures

### Dependency Resolution
- Track which specs relate to which code files
- Maintain bidirectional links
- Update dependency graph on changes
- Determine cascade effects

## Validation Gates

```bash
# Build and test monitoring
cargo build --features monitoring
cargo test monitor::tests

# Test file watching
cargo run -- monitor --test-mode

# Verify ignore patterns
echo "test" > target/test.txt  # Should be ignored
echo "test" > SPEC.md          # Should trigger

# Test debouncing
for i in {1..10}; do echo "change $i" >> README.md; done
# Should result in single event
```

## Success Criteria
- Detects all file changes within 100ms
- Correctly classifies file types
- Debouncing prevents event storms
- Respects ignore patterns
- Minimal CPU usage when idle (<1%)

## Known Patterns and Conventions
- Use platform-native file watching APIs
- Implement exponential backoff for errors
- Cache file classifications
- Use async/await for event handling
- Maintain event ordering

## Common Pitfalls to Avoid
- Don't watch node_modules or target directories
- Handle file permission errors gracefully
- Avoid infinite loops from self-modifications
- Remember to handle symbolic links
- Don't lose events during processing

## Dependencies Required
- notify = "6.0"
- ignore = "0.4"
- walkdir = "2.0"
- tokio = { version = "1.0", features = ["fs", "sync"] }
- dashmap = "5.0"  # Concurrent hashmap for state

## Performance Considerations
- Batch process multiple changes
- Use memory-mapped files for large files
- Implement sampling for very active files
- Cache parsed content where possible
- Consider inotify limits on Linux

## Configuration Options
```toml
[monitor]
enabled = true
debounce_ms = 500
max_queue_size = 1000
ignore_patterns = ["target/**", "node_modules/**"]
watch_hidden = false
follow_symlinks = false
```

## Confidence Score: 8/10
File system monitoring is well-solved with notify crate. The complexity lies in intelligent classification and change analysis.