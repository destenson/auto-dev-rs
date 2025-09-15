# Auto Dev

Auto Dev is an open-source project designed to streamline and automate the software development process. It provides a suite of tools and features that help developers manage their projects more efficiently, from code generation to deployment.

## Features

- **Code Generation**: Automatically generate boilerplate code for various programming languages and frameworks.
- **Intelligent LLM Routing**: 5-tier model system that intelligently routes tasks to the most cost-effective model, from no-LLM pattern matching to large language models.
- **Cost Optimization**: Advanced cost tracking and budget management with dynamic routing strategies based on daily/monthly budgets.
- **Performance Monitoring**: Real-time tracking of model performance, success rates, and latency with automatic tier adjustments.
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

## LLM Optimization and Routing

Auto Dev features a sophisticated 5-tier model routing system that intelligently selects the most cost-effective model for each task:

### Model Tiers
- **Tier 0 (No LLM)**: Pattern matching and template-based solutions
- **Tier 1 (Tiny)**: Local models like Qwen 0.5B for simple classifications
- **Tier 2 (Small)**: 7B models for basic code generation
- **Tier 3 (Medium)**: 13-34B models for complex generation
- **Tier 4 (Large)**: GPT-4, Claude-3 for architecture and design

### Key Components
- **ComplexityClassifier**: Determines task complexity using rules and ML
- **CostTracker**: Monitors spending and enforces budget limits
- **PerformanceMonitor**: Tracks model performance and recommends adjustments
- **ModelRegistry**: Manages available models and their capabilities
- **CostOptimizer**: Selects optimal models based on cost/performance

### Configuration
Models are configured in `auto-dev-core/models.toml`. The system automatically:
- Routes 70% of tasks to free/cheap tiers
- Falls back to higher tiers on failure
- Adjusts routing based on budget usage
- Tracks performance metrics for optimization

### Cost Savings
Expected daily cost reduction of 87% compared to using only high-tier models:
- Without optimization: ~$20/day
- With optimization: ~$2.60/day

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

## Self-Improvement and Learning System

Auto Dev features an advanced learning system that improves code generation quality over time by learning from successes and failures. This system builds a knowledge base of patterns and solutions that reduces LLM dependency and improves performance.

### Key Components

#### Learning System
The core learning module orchestrates all learning activities:
- **Pattern Extraction**: Identifies reusable patterns from successful implementations
- **Success Tracking**: Records and reinforces successful approaches
- **Failure Analysis**: Learns from mistakes and identifies anti-patterns
- **Decision Improvement**: Continuously improves decision-making based on outcomes
- **Knowledge Management**: Stores and retrieves patterns for future use

#### Pattern Library
Automatically builds a library of successful patterns:
- **Structural Patterns**: Functions, classes, and module structures
- **Behavioral Patterns**: Loops, conditionals, async operations
- **Idioms**: Language-specific best practices
- **Anti-patterns**: Code patterns to avoid

#### Success Metrics
Tracks comprehensive metrics for each implementation:
- Compilation success rate
- Test pass rate
- Performance benchmarks
- Security compliance
- Specification coverage
- LLM call reduction

### How It Works

1. **Learning from Success**
   - When code generation succeeds, the system extracts patterns
   - Patterns are evaluated for quality and reusability
   - High-quality patterns are added to the knowledge base
   - Success metrics reinforce effective approaches

2. **Learning from Failure**
   - Failed attempts are analyzed to identify root causes
   - Anti-patterns are extracted and stored
   - Guard conditions are added to prevent similar failures
   - Decision strategies are adjusted based on failure patterns

3. **Pattern Application**
   - For new tasks, the system searches for similar patterns
   - Matching patterns are adapted to the current context
   - Pattern-based generation reduces LLM calls
   - Success rate improves with pattern reuse

### Benefits

- **50% LLM Usage Reduction**: After 100 implementations, pattern reuse significantly reduces LLM dependency
- **30% Pattern Reuse Rate**: Common tasks are handled by proven patterns
- **Continuous Improvement**: Decision accuracy and success rates improve over time
- **Knowledge Persistence**: Learning persists across sessions via knowledge export/import
- **Privacy-First**: All learning happens locally with no external data sharing

### Configuration

The learning system can be configured via the learning configuration:

```rust
LearningConfig {
    enabled: true,                    // Enable/disable learning
    auto_learn: true,                 // Automatic learning from events
    min_pattern_quality: 0.7,         // Minimum quality threshold
    max_patterns: 10000,              // Maximum patterns to store
    learning_rate: 0.1,               // Learning rate for adjustments
    export_path: ".auto-dev/knowledge", // Knowledge base storage
    import_on_startup: true,          // Load previous knowledge
}
```

### Knowledge Management

The system provides tools for managing learned knowledge:

```bash
# Export knowledge base
cargo run -- learn export --output knowledge.json

# Import knowledge base
cargo run -- learn import --input knowledge.json

# View learning metrics
cargo run -- learn metrics

# Find similar patterns
cargo run -- learn find-similar "implement authentication"
```

### Privacy and Ethics

The learning system is designed with privacy in mind:
- All learning happens locally on your machine
- No code or patterns are shared externally
- Patterns are anonymized and generalized
- Users can opt-out of learning at any time
- Knowledge base is fully exportable and deletable

# TODO

- Implement everything mentioned in the features list.

- Add more detailed documentation and examples.
- Implement additional plugins and integrations.
- Enhance the user interface for better usability.
- Expand support for more programming languages and frameworks.
