# PRP: Code Synthesis and Implementation Engine

## Overview
Build the core engine that orchestrates the transformation of specifications into working code, managing the entire synthesis pipeline from requirement analysis to code generation and integration.

## Context and Background
This is the central orchestrator that coordinates between specification parsing, LLM code generation, existing code analysis, and incremental implementation. It maintains the state of what's been implemented and what remains.

### Research References
- Roslyn architecture for code synthesis: https://github.com/dotnet/roslyn/wiki
- Language Server Protocol: https://microsoft.github.io/language-server-protocol/
- Tree-sitter for code analysis: https://tree-sitter.github.io/
- syn for Rust AST manipulation: https://docs.rs/syn/latest/syn/

## Requirements

### Primary Goals
1. Orchestrate end-to-end code synthesis
2. Maintain implementation state and progress
3. Merge generated code with existing code
4. Ensure consistency across implementations
5. Track specification coverage

### Technical Constraints
- Must preserve existing functionality
- Handle partial implementations
- Support incremental synthesis
- Maintain code quality standards
- Respect project conventions

## Architectural Decisions

### Decision: Pipeline Architecture
**Chosen**: Pipeline pattern with stages
**Alternatives Considered**:
- Event-driven architecture: Too complex for initial version
- Monolithic processor: Lack of flexibility
- Actor model: Overhead without clear benefits
**Rationale**: Pipeline provides clear stages, easy testing, and natural parallelization points

### Decision: State Management
**Chosen**: Filesystem-based state with JSON
**Alternatives Considered**:
- In-memory only: Loss of state on restart
- SQLite database: Against no-SQL requirement
- Binary format: Lack of transparency
**Rationale**: JSON files provide transparency, debugging ease, and version control compatibility

## Implementation Blueprint

### File Structure
```
src/
├── synthesis/
│   ├── mod.rs              # Synthesis module exports
│   ├── engine.rs           # Main synthesis orchestrator
│   ├── pipeline/
│   │   ├── mod.rs
│   │   ├── analyzer.rs     # Analyze existing code
│   │   ├── planner.rs      # Plan implementation
│   │   ├── generator.rs    # Generate new code
│   │   ├── merger.rs       # Merge with existing
│   │   └── validator.rs    # Validate implementation
│   ├── state.rs            # Implementation state tracking
│   └── coverage.rs         # Specification coverage analysis
```

### Key Components
1. **SynthesisEngine**: Main orchestrator
2. **ImplementationPlanner**: Plans what to implement
3. **CodeGenerator**: Manages code generation
4. **CodeMerger**: Integrates new with existing code
5. **StateTracker**: Tracks what's implemented

### Synthesis Pipeline
```rust
enum PipelineStage {
    Analysis,      // Analyze current state
    Planning,      // Plan what to implement
    Generation,    // Generate code via LLM
    Merging,       // Merge with existing
    Validation,    // Validate against spec
    Integration,   // Integrate into project
}

struct SynthesisPipeline {
    stages: Vec<Box<dyn PipelineStage>>,
    state: SynthesisState,
    
    async fn execute(&mut self, spec: Specification) -> Result<SynthesisResult> {
        for stage in &self.stages {
            self.state = stage.process(self.state, &spec).await?;
        }
        Ok(self.state.into())
    }
}
```

### Implementation State
```rust
struct SynthesisState {
    specifications: HashMap<String, SpecificationStatus>,
    implementations: HashMap<PathBuf, ImplementationStatus>,
    coverage: CoverageReport,
    pending_tasks: Vec<ImplementationTask>,
    completed_tasks: Vec<ImplementationTask>,
    decisions: Vec<ArchitectureDecision>,
}

struct ImplementationTask {
    id: Uuid,
    spec_id: String,
    description: String,
    target_file: PathBuf,
    status: TaskStatus,
    attempts: Vec<GenerationAttempt>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}
```

### Implementation Tasks (in order)
1. Create synthesis module structure
2. Build synthesis state management
3. Implement code analyzer for existing code
4. Create implementation planner
5. Build code generation coordinator
6. Implement AST-based code merger
7. Add validation pipeline
8. Create coverage tracking
9. Implement rollback mechanism
10. Add decision recording system
11. Build progress reporting
12. Create synthesis configuration

## Implementation Planning

### Planning Strategy
```rust
struct ImplementationPlanner {
    fn plan(&self, spec: &Specification, state: &SynthesisState) -> Plan {
        // 1. Identify unimplemented requirements
        // 2. Resolve dependencies
        // 3. Order by priority and dependencies
        // 4. Group related implementations
        // 5. Create task list
    }
}

struct Plan {
    tasks: Vec<ImplementationTask>,
    dependencies: HashMap<TaskId, Vec<TaskId>>,
    estimated_complexity: Complexity,
    approach: ImplementationApproach,
}
```

## Code Merging Strategy

### Merge Approaches
1. **File-level replacement**: For new files
2. **Function-level insertion**: Add new functions
3. **Block-level modification**: Modify existing functions
4. **Line-level patches**: Small changes

### Decision: Merge Strategy
**Chosen**: AST-based merging with fallback to text
**Alternatives Considered**:
- Pure text-based diff/patch: Loss of semantic understanding
- Full AST rewriting: Too complex, language-specific
- Manual merge markers: Requires human intervention
**Rationale**: AST provides semantic awareness while text fallback ensures compatibility

## Architecture Decision Records

### ADR Template
```markdown
# ADR-{number}: {title}

## Status
{Proposed|Accepted|Deprecated|Superseded}

## Context
{What is the issue that we're seeing that is motivating this decision?}

## Decision
{What is the change that we're proposing and/or doing?}

## Alternatives Considered
- {Alternative 1}: {Why not chosen}
- {Alternative 2}: {Why not chosen}

## Consequences
{What becomes easier or more difficult because of this change?}
```

## Validation Gates

```bash
# Test synthesis pipeline
cargo test synthesis::tests

# Test with mock specifications
cargo run -- synthesize --dry-run samples/spec.md

# Test incremental synthesis
cargo run -- synthesize --incremental samples/

# Verify state persistence
cargo run -- synthesize --status
```

## Success Criteria
- Successfully synthesizes code from specs
- Preserves existing functionality
- Maintains implementation state
- Handles failures gracefully
- Provides clear progress tracking

## Known Patterns and Conventions
- Use Command pattern for pipeline stages
- Implement Memento for state rollback
- Use Builder for complex configurations
- Apply Strategy for merge algorithms
- Follow Repository pattern for state

## Common Pitfalls to Avoid
- Don't lose existing code
- Handle merge conflicts carefully
- Avoid infinite generation loops
- Remember to validate generated code
- Don't ignore existing patterns

## Dependencies Required
- syn = "2.0"  # Rust AST manipulation
- quote = "1.0"  # Code generation
- tree-sitter = "0.20"  # Multi-language parsing
- similar = "2.0"  # Diff generation
- ropey = "1.6"  # Efficient text manipulation

## Performance Considerations
- Cache AST representations
- Parallelize independent generations
- Incremental synthesis for large projects
- Lazy load existing code
- Batch related changes

## Recovery and Rollback
```rust
struct RecoveryManager {
    fn checkpoint(&self, state: &SynthesisState) -> CheckpointId;
    fn rollback(&self, checkpoint: CheckpointId) -> Result<()>;
    fn list_checkpoints(&self) -> Vec<Checkpoint>;
}
```

## Confidence Score: 7/10
Complex orchestration with many moving parts. The architecture is sound but implementation requires careful coordination between components.