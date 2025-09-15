# Validation System

The validation system ensures code quality and correctness through multiple stages. It's designed to be flexible, allowing you to run only the checks you need when you need them.

## Validation Stages

### Always Run (Default)
These checks are fast and catch critical issues:
- **SyntaxCheck**: Basic syntax validation (5s timeout)
- **Compilation**: Ensures code compiles (30s timeout)  
- **UnitTests**: Runs test suite (60s timeout)

### Run On Demand
These checks are expensive to fix and should be run periodically:
- **Linting**: Code quality checks with clippy (disabled by default)
- **Security**: Dependency and code security scanning (disabled by default)
- **Performance**: Performance benchmarks and profiling (disabled by default)
- **Specification**: Validates against specs (on demand)
- **Integration**: Integration test suites (on demand)

## Tool Discovery

The system automatically discovers available tools at runtime rather than hardcoding them. It will:
1. Check for essential tools based on project type (Cargo.toml, package.json, etc.)
2. Discover optional tools and suggest installation if missing
3. Allow custom tool registration via configuration

## Usage

### Quick Validation (Default)
```bash
# Runs only syntax, compilation, and tests
auto-dev validate
```

### Full Validation 
```bash
# Runs all validation stages including expensive checks
auto-dev validate --all
```

### Specific Stages
```bash
# Run only specific validation stages
auto-dev validate --compilation --tests
auto-dev validate --quality  # Runs linting and quality checks
auto-dev validate --security # Runs security scanning
```

### Tool Discovery
```bash
# See what validation tools are available
auto-dev validate discover
```

## Configuration

Configure validation in `.auto-dev/config.toml`:

```toml
[validation]
enabled = true
fail_fast = true  # Stop on first failure
parallel = true   # Run stages in parallel where possible

# Enable quality checks for CI/CD
[validation.stages.linting]
enabled = true  # Override default

[validation.quality_rules]
max_function_length = 50
max_cyclomatic_complexity = 10
min_test_coverage = 80
```

## Philosophy

The validation system follows these principles:

1. **Fast Feedback**: Critical checks run by default for quick iteration
2. **Opt-in Quality**: Expensive quality checks are optional 
3. **Tool Agnostic**: Discovers and uses available tools at runtime
4. **Project Aware**: Adapts to project type (Rust, JavaScript, Python, etc.)
5. **Progressive Enhancement**: Works with minimal tools, better with more

## Best Practices

- Run default validation during development for fast feedback
- Run quality checks (`--quality`) before commits
- Run security checks (`--security`) before releases  
- Run full validation (`--all`) in CI/CD pipelines
- Install recommended tools for better validation coverage