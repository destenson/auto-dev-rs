# Incremental Development Plan: Tiny Model Integration

## Phase 1: Binary Classification (Simplest) ✅
**Already Done**: Heuristic classifier that can determine:
- Is this code? (yes/no)
- Is this a test? (yes/no)

**Next Step**: Make it useful in the CLI
```bash
auto-dev classify file.rs       # Output: "code: rust"
auto-dev classify README.md      # Output: "documentation: markdown"
```

## Phase 2: Language Detection (Current Focus)
**Goal**: Accurately identify programming language
- Start with top 10 languages
- Use simple pattern matching
- Add to file watcher to auto-categorize projects

**Immediate Use Case**: 
```bash
auto-dev analyze .              # Scan project and report language distribution
# Output: "Project: 60% Rust, 30% TOML, 10% Markdown"
```

## Phase 3: Simple Question Detection
**Goal**: Classify question complexity
- Is this a simple definition question?
- Can tiny model answer it?
- Should we route to larger model?

**Use Case**:
```bash
auto-dev ask "What is a mutex?"    # Tiny model answers
auto-dev ask "How do I implement a compiler?"  # Routes to large model
```

## Phase 4: Code Pattern Detection
**Goal**: Find specific patterns quickly
- Has error handling?
- Uses unsafe code?
- Contains TODOs?

**Use Case**:
```bash
auto-dev scan --pattern todos     # Find all TODO comments
auto-dev scan --pattern unsafe    # Find unsafe blocks
```

## Phase 5: Simple Validation
**Goal**: Check if code matches requirements
- Basic requirement satisfaction
- Missing implementation detection

**Use Case**:
```bash
auto-dev check "must validate email" src/auth.rs
# Output: " Requirement likely satisfied"
```

## Phase 6: Load Actual GGUF Model
**Goal**: Replace some heuristics with model
- Download and load Qwen2.5-0.5B
- Compare accuracy vs heuristics
- Use model where it's better

## Phase 7: Smart Routing
**Goal**: Decide when to use which tool
- Heuristics for obvious cases
- Tiny model for fuzzy matching
- Large model for complex tasks

## Phase 8: Real-Time Monitoring
**Goal**: Use in file watcher
- Classify files as they change
- Detect what kind of work is happening
- Provide relevant suggestions

---

## Let's Start with Phase 2: Language Detection CLI

This is immediately useful and builds on what we have!
