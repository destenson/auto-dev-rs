# PRP: Integration Tests Framework

## Overview
Establish comprehensive integration testing for auto-dev, ensuring all components work together correctly. Tests should cover CLI commands, file operations, and end-to-end workflows.

## Context and Background
Integration tests validate that different components work together as expected. They catch issues that unit tests miss and provide confidence for releases. Tests should be fast, reliable, and easy to maintain.

### Research References
- assert_cmd for CLI testing: https://docs.rs/assert_cmd/latest/assert_cmd/
- predicates for assertions: https://docs.rs/predicates/latest/predicates/
- tempfile for test isolation: https://docs.rs/tempfile/latest/tempfile/
- insta for snapshot testing: https://docs.rs/insta/latest/insta/

## Requirements

### Primary Goals
1. Test CLI command execution and output
2. Validate file generation and modification
3. Test configuration loading and merging
4. Verify plugin discovery and loading
5. Test end-to-end workflows

### Technical Constraints
- Tests must be isolated (no side effects)
- Use temporary directories for file operations
- Mock external dependencies
- Support parallel test execution
- Maintain reasonable execution time (<30s total)

## Implementation Blueprint

### File Structure
```
tests/
├── common/
│   ├── mod.rs           # Shared test utilities
│   ├── fixtures.rs      # Test fixtures and data
│   └── helpers.rs       # Helper functions
├── cli_tests.rs         # CLI command tests
├── config_tests.rs      # Configuration tests
├── generator_tests.rs   # Code generation tests
├── plugin_tests.rs      # Plugin system tests
├── template_tests.rs    # Template engine tests
└── workflow_tests.rs    # End-to-end workflows
```

### Key Components
1. **TestContext**: Isolated test environment
2. **CommandRunner**: Execute CLI commands
3. **FileAssertions**: Verify file operations
4. **FixtureManager**: Manage test data
5. **SnapshotTester**: Compare outputs

### Test Categories

#### CLI Tests
```rust
// Example test structure
#[test]
fn test_cli_help_command() {
    let mut cmd = Command::cargo_bin("auto-dev").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Auto Dev"))
        .stdout(predicate::str::contains("USAGE"));
}

#[test]
fn test_generate_rust_struct() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("auto-dev").unwrap();
    
    cmd.current_dir(&temp)
        .args(&["generate", "rust", "struct", "User"])
        .arg("--field").arg("name:String")
        .assert()
        .success();
    
    temp.child("user.rs").assert(predicate::path::exists());
}
```

#### Configuration Tests
```rust
#[test]
fn test_config_layering() {
    let context = TestContext::new();
    
    // Create config files at different levels
    context.write_global_config(/* ... */);
    context.write_project_config(/* ... */);
    
    // Test merge order
    let config = load_config(&context.root()).unwrap();
    assert_eq!(config.get("key"), "project_value");
}
```

### Implementation Tasks (in order)
1. Set up tests directory structure
2. Add test dependencies to Cargo.toml
3. Create TestContext for isolated testing
4. Implement CLI command tests
5. Add configuration loading tests
6. Create code generation tests
7. Build template rendering tests
8. Add plugin discovery tests
9. Implement task tracking tests
10. Create end-to-end workflow tests
11. Add performance benchmarks
12. Set up CI test execution

## Test Utilities

### TestContext Helper
```rust
struct TestContext {
    temp_dir: TempDir,
    home_dir: PathBuf,
    config_dir: PathBuf,
}

impl TestContext {
    fn new() -> Self { /* ... */ }
    fn run_command(&self, args: &[&str]) -> Output { /* ... */ }
    fn write_file(&self, path: &str, content: &str) { /* ... */ }
    fn read_file(&self, path: &str) -> String { /* ... */ }
}
```

### Fixture Management
```rust
fn sample_rust_project() -> ProjectFixture {
    ProjectFixture {
        files: vec![
            ("Cargo.toml", include_str!("fixtures/rust/Cargo.toml")),
            ("src/main.rs", include_str!("fixtures/rust/main.rs")),
        ],
        config: Some(/* ... */),
    }
}
```

## Validation Gates

```bash
# Run all tests
cargo test

# Run integration tests only
cargo test --test '*'

# Run with coverage
cargo tarpaulin --out Html

# Run benchmarks
cargo bench

# Test in release mode
cargo test --release
```

## Success Criteria
- All tests pass consistently
- Code coverage >80%
- Tests run in <30 seconds
- No flaky tests
- Clear failure messages

## Known Patterns and Conventions
- Use descriptive test names
- One assertion per test when possible
- Use fixtures for complex data
- Clean up temp files automatically
- Mock external dependencies

## Common Pitfalls to Avoid
- Don't depend on test execution order
- Avoid hardcoded paths
- Don't share state between tests
- Remember to test error cases
- Handle platform differences

## Dependencies Required
```toml
[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
assert_fs = "1.0"
tempfile = "3.0"
insta = "1.0"
serial_test = "3.0"  # For tests that must run serially
proptest = "1.0"  # Property-based testing
```

## Test Scenarios

### Smoke Tests
1. Binary runs without arguments
2. Help text displays
3. Version information correct
4. Basic command execution

### Feature Tests
1. Generate code for each language
2. Load and merge configurations
3. Discover and load plugins
4. Render templates with variables
5. Track and resume tasks

### Edge Cases
1. Malformed configuration files
2. Missing dependencies
3. Invalid template syntax
4. Concurrent operations
5. Large file handling

### Error Scenarios
1. Permission denied
2. Disk full
3. Network timeout
4. Corrupted data
5. Invalid input

## CI/CD Integration

```yaml
# .github/workflows/test.yml
name: Tests
on: [push, pull_request]
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test --all-features
      - run: cargo test --no-default-features
```

## Confidence Score: 9/10
Well-established testing patterns with mature tooling. Integration testing for CLI applications is a solved problem with excellent crate support.