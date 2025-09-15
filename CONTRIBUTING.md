# Contributing to Auto Dev

Thank you for your interest in contributing to Auto Dev! This document provides guidelines and instructions for contributing to the project.

## Development Setup

### Prerequisites

- Rust 1.70.0 or later
- Git
- A code editor (VS Code, IntelliJ IDEA with Rust plugin, or similar)

### Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/yourusername/auto-dev-rs.git
   cd auto-dev-rs
   ```

3. Build the project:
   ```bash
   cargo build --workspace
   ```

4. Run tests:
   ```bash
   cargo test --workspace
   ```

## Code Style Guidelines

We use `rustfmt` and `clippy` to maintain consistent code style:

- Format your code: `cargo fmt --all`
- Check for lints: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

### Key Conventions

- Use descriptive variable and function names
- Add documentation comments for public APIs
- Write unit tests for new functionality
- Keep functions focused and small
- Use the type system to prevent errors

## Pull Request Process

1. Create a new branch for your feature or fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes following the code style guidelines

3. Add or update tests as needed

4. Update documentation if you're changing public APIs

5. Ensure all tests pass:
   ```bash
   cargo test --workspace
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```

6. Commit your changes using conventional commit messages:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `test:` for test changes
   - `refactor:` for code refactoring
   - `chore:` for maintenance tasks

7. Push to your fork and create a pull request

8. Wait for review and address any feedback

## Testing Requirements

- All new features must include unit tests
- Integration tests are required for CLI commands
- Maintain or improve code coverage
- Test on multiple platforms when possible

## Documentation Standards

- Add rustdoc comments for all public items
- Include examples in documentation when helpful
- Update README.md for user-facing changes
- Document breaking changes in CHANGELOG.md

## Project Structure

```
auto-dev-rs/
├── auto-dev/           # Main binary crate
│   └── src/
│       ├── main.rs     # Entry point
│       └── cli/        # CLI implementation
├── auto-dev-core/      # Core library crate
│   └── src/
│       └── lib.rs      # Core functionality
├── tests/              # Integration tests
└── PRPs/               # Project planning documents
```

## Getting Help

- Check existing issues and PRPs for context
- Ask questions in issues or discussions
- Review the codebase documentation
- Reach out to maintainers for guidance

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Assume good intentions

## License

By contributing to Auto Dev, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).