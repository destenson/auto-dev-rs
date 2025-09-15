# Auto Dev

Auto Dev is an open-source project designed to streamline and automate the software development process. It provides a suite of tools and features that help developers manage their projects more efficiently, from code generation to deployment.

## Features

- **Code Generation**: Automatically generate boilerplate code for various programming languages and frameworks.
- **Project Management**: Tools to help manage tasks, track progress, and collaborate with team members.
- **Continuous Integration/Continuous Deployment (CI/CD)**: Integrate with popular CI/CD tools to automate testing and deployment.
- **Version Control Integration**: Seamlessly integrate with Git and other version control systems.
- **Customizable Workflows**: Create and customize workflows to fit your team's development process.
- **Extensive Plugin System**: Extend functionality with a wide range of plugins or create your own.
- **Documentation Generation**: Automatically generate and maintain project documentation.
- **Testing Frameworks**: Built-in support for various testing frameworks to ensure code quality.
- **Monitoring and Analytics**: Track project metrics and performance over time.
- **Security Features**: Tools to help identify and mitigate security vulnerabilities in your code.
- **Cloud Integration**: Support for deploying applications to popular cloud platforms.
- **User-Friendly Interface**: Intuitive UI to simplify project management and development tasks.
- **Multi-language Support**: Compatible with multiple programming languages including Rust, Python, JavaScript, Java, and more.
- **Self-Targeting Mode**: Auto Dev can analyze and improve its own codebase using the `--target-self` flag.

## Self-Targeting Mode

Auto Dev includes a powerful self-targeting feature that allows it to analyze and improve its own codebase. This enables continuous self-improvement and serves as an excellent example of the tool's capabilities.

### Usage

To use self-targeting mode, add the `--target-self` flag to any command:

```bash
# Analyze auto-dev's own codebase
cargo run -- analyze --target-self .

# Start the development loop targeting itself
cargo run -- loop start --target-self

# Run auto-dev on itself
cargo run -- run --target-self
```

### Configuration

When using `--target-self`, auto-dev automatically:
- Uses cargo metadata to find the workspace root
- Configures monitoring for Rust source files and PRPs
- Sets up appropriate safety validations
- Creates a `.auto-dev/self.toml` configuration file

### Safety Features

Self-targeting mode includes built-in safety features:
- Strict safety mode by default
- Forbidden paths (`.git`, `target`, `Cargo.lock`)
- File size limits for modifications
- Backup creation before modifications
- Confirmation required for destructive operations

# TODO

- Implement everything mentioned in the features list.

- Add more detailed documentation and examples.
- Implement additional plugins and integrations.
- Enhance the user interface for better usability.
- Expand support for more programming languages and frameworks.
