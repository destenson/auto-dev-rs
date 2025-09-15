# PRP: Project Context Management System

## Overview
Build a comprehensive context management system that maintains deep understanding of the project structure, codebase patterns, dependencies, and conventions to inform intelligent code generation.

## Context and Background
Effective code synthesis requires understanding the entire project context: existing code patterns, architectural decisions, coding conventions, dependencies, and team preferences. This system maintains and provides that context to all other components.

### Research References
- rust-analyzer architecture: https://github.com/rust-analyzer/rust-analyzer
- Language Server Protocol: https://microsoft.github.io/language-server-protocol/
- Semantic code search: https://github.com/github/semantic
- Code embeddings for similarity: https://arxiv.org/abs/2301.06731

## Requirements

### Primary Goals
1. Build comprehensive project understanding
2. Track code patterns and conventions
3. Maintain dependency graph
4. Store architectural decisions
5. Provide context for code generation

### Technical Constraints
- Must work with incomplete/broken code
- Handle multiple programming languages
- Scale to large codebases
- Update incrementally for performance
- Preserve memory across sessions

## Architectural Decisions

### Decision: Storage Strategy
**Chosen**: Hierarchical JSON with embeddings database
**Alternatives Considered**:
- Pure in-memory: Loss on restart, memory limitations
- Graph database: Complexity, against no-SQL requirement
- Flat file structure: Poor query performance
**Rationale**: JSON provides flexibility and debugging ease, embeddings enable semantic search

### Decision: Code Analysis Approach
**Chosen**: Hybrid AST + Pattern matching + LLM analysis
**Alternatives Considered**:
- Pure AST analysis: Misses semantic patterns
- Only LLM analysis: Too slow, expensive
- Regex patterns only: Too brittle
**Rationale**: Combination provides accuracy with reasonable performance

## Implementation Blueprint

### File Structure
```
src/
├── context/
│   ├── mod.rs              # Context module exports
│   ├── manager.rs          # Context manager
│   ├── analyzer/
│   │   ├── mod.rs
│   │   ├── structure.rs    # Project structure analysis
│   │   ├── patterns.rs     # Pattern detection
│   │   ├── conventions.rs  # Convention inference
│   │   └── dependencies.rs # Dependency analysis
│   ├── storage.rs          # Context persistence
│   ├── embeddings.rs       # Code embeddings
│   └── query.rs            # Context queries

.auto-dev/
├── context/
│   ├── project.json        # Project metadata
│   ├── patterns.json       # Detected patterns
│   ├── conventions.json    # Inferred conventions
│   ├── dependencies.json   # Dependency graph
│   ├── decisions/          # Architecture decisions
│   └── embeddings.db       # Vector embeddings
```

### Key Components
1. **ContextManager**: Central context coordination
2. **ProjectAnalyzer**: Analyzes project structure
3. **PatternDetector**: Identifies code patterns
4. **ConventionInferrer**: Infers coding conventions
5. **EmbeddingStore**: Semantic code search

### Context Model
```rust
struct ProjectContext {
    metadata: ProjectMetadata,
    structure: ProjectStructure,
    patterns: Vec<CodePattern>,
    conventions: CodingConventions,
    dependencies: DependencyGraph,
    decisions: Vec<ArchitectureDecision>,
    embeddings: EmbeddingStore,
    history: ContextHistory,
}

struct ProjectMetadata {
    name: String,
    languages: Vec<Language>,
    frameworks: Vec<Framework>,
    build_systems: Vec<BuildSystem>,
    team_size: Option<usize>,
    created_at: DateTime<Utc>,
    last_updated: DateTime<Utc>,
}

struct CodePattern {
    name: String,
    description: String,
    examples: Vec<CodeExample>,
    frequency: f32,
    locations: Vec<PathBuf>,
    pattern_type: PatternType,
}

enum PatternType {
    Architectural,   // MVC, Hexagonal, etc.
    Design,         // Factory, Observer, etc.
    Idiom,          // Language-specific idioms
    Convention,     // Naming, structure, etc.
}
```

### Implementation Tasks (in order)
1. Create context module structure
2. Build project structure analyzer
3. Implement pattern detection system
4. Create convention inference engine
5. Build dependency graph analyzer
6. Add embeddings generation
7. Implement semantic search
8. Create context persistence
9. Build incremental update system
10. Add context query interface
11. Implement context validation
12. Create context visualization

## Pattern Detection

### Pattern Categories
```rust
struct PatternLibrary {
    architectural_patterns: Vec<ArchPattern>,
    design_patterns: Vec<DesignPattern>,
    idioms: HashMap<Language, Vec<Idiom>>,
    anti_patterns: Vec<AntiPattern>,
}

impl PatternDetector {
    fn detect_patterns(&self, code: &str) -> Vec<DetectedPattern> {
        // 1. AST analysis for structural patterns
        // 2. Regex matching for idioms
        // 3. LLM analysis for complex patterns
        // 4. Statistical analysis for frequency
    }
}
```

### Convention Inference
```rust
struct CodingConventions {
    naming: NamingConventions,
    formatting: FormattingRules,
    structure: StructureConventions,
    documentation: DocConventions,
}

struct NamingConventions {
    functions: NamingStyle,    // snake_case, camelCase
    types: NamingStyle,        // PascalCase
    constants: NamingStyle,    // UPPER_SNAKE
    files: NamingStyle,        // kebab-case
}
```

## Embeddings and Semantic Search

### Decision: Embedding Strategy
**Chosen**: Local embeddings with BERT-based model
**Alternatives Considered**:
- OpenAI embeddings: Cost and privacy concerns
- No embeddings: Miss semantic similarity
- Simple keyword search: Insufficient for patterns
**Rationale**: Local embeddings provide privacy and cost efficiency

### Implementation
```rust
struct EmbeddingStore {
    model: EmbeddingModel,
    index: VectorIndex,
    
    async fn embed_code(&self, code: &str) -> Vector {
        // Generate embeddings for code
    }
    
    async fn find_similar(&self, query: &str, k: usize) -> Vec<SimilarCode> {
        // Find k most similar code snippets
    }
}
```

## Context Queries

### Query Interface
```rust
impl ContextManager {
    // Find similar implementations
    fn find_similar_code(&self, spec: &str) -> Vec<CodeExample>;
    
    // Get relevant patterns
    fn get_patterns_for(&self, file_type: &str) -> Vec<CodePattern>;
    
    // Get conventions
    fn get_conventions(&self) -> &CodingConventions;
    
    // Get architectural decisions
    fn get_decisions_for(&self, component: &str) -> Vec<Decision>;
}
```

## Incremental Updates

### Update Strategy
```rust
enum ContextUpdate {
    FileAdded(PathBuf),
    FileModified(PathBuf),
    FileDeleted(PathBuf),
    PatternDetected(CodePattern),
    DecisionMade(ArchitectureDecision),
}

impl ContextManager {
    async fn update(&mut self, update: ContextUpdate) {
        // Update only affected parts
        // Recalculate dependencies
        // Update embeddings incrementally
    }
}
```

## Validation Gates

```bash
# Test context analysis
cargo test context::tests

# Analyze sample project
cargo run -- context analyze ./sample-project

# Test pattern detection
cargo run -- context patterns ./src

# Query similar code
cargo run -- context find-similar "implement user authentication"

# Export context report
cargo run -- context export --format json
```

## Success Criteria
- Accurately identifies project patterns
- Maintains context across sessions
- Provides relevant examples for generation
- Updates incrementally in <1s
- Handles multi-language projects

## Known Patterns and Conventions
- Use visitor pattern for AST traversal
- Cache analysis results aggressively
- Implement observer for context updates
- Use facade for complex queries
- Apply strategy for language-specific analysis

## Common Pitfalls to Avoid
- Don't analyze binary files
- Handle symbolic links carefully
- Avoid analyzing generated code
- Remember to respect .gitignore
- Don't block on large file analysis

## Dependencies Required
- tree-sitter = "0.20"  # Multi-language parsing
- petgraph = "0.6"  # Dependency graphs
- candle = "0.3"  # Local embeddings
- hnsw = "0.11"  # Vector similarity search
- rayon = "1.7"  # Parallel analysis

## Performance Optimizations
- Incremental analysis on file changes
- Parallel pattern detection
- Cache embeddings persistently
- Lazy load context sections
- Use bloom filters for quick checks

## Privacy and Security
- All analysis happens locally
- No code sent to external services
- Embeddings stored locally
- Sensitive patterns can be excluded
- Support for private pattern libraries

## Confidence Score: 8/10
Well-defined problem space with proven approaches. The complexity lies in accurate pattern detection and maintaining performance at scale.