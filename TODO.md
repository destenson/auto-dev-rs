# TODO List for auto-dev-rs

## Priority: Critical

### Core Implementation Gaps
- **Code Generation System** - Replace placeholder with actual implementation (auto-dev-core/src/synthesis/pipeline/generator.rs:44)
  - Currently returns `// TODO: Implement {task}`
  - Need actual language-specific code generation
  - Integration with LLM providers for synthesis

- **CLI Commands** - All major commands unimplemented
  - Generate command (auto-dev/src/cli/commands/generate.rs:10)
  - Test command (auto-dev/src/cli/commands/test.rs:5)
  - Docs command (auto-dev/src/cli/commands/docs.rs:6)
  - Deploy command (auto-dev/src/cli/commands/deploy.rs:6)
  - Manage command (auto-dev/src/cli/commands/manage.rs:6)

### Memory & Build Issues
- **Fix Windows paging file errors** preventing parallel compilation
- **Resolve test compilation failures** due to memory constraints
- **Fix Ollama provider URL construction** (auto-dev-core/src/llm/ollama.rs:36)
  - Check that Ollama::new() expects URL to contain port

## Priority: High

### LLM Integration
- **Add Claude, OpenAI, Groq providers** to router (auto-dev-core/src/llm/router.rs:115)
- **Replace stub implementations** with actual model integration

### Claude Configuration (NEW - 11 PRPs Generated)
- **PRP-260 to PRP-270**: Full Claude configuration awareness implementation
  - Discovery of .claude directories
  - CLAUDE.md loading and context integration
  - Command parsing and registry
  - CLI integration for custom commands
  - File watching for hot-reload
  - Total: ~27-32 hours of work

### Test Framework Completion
- **Implement test assertions** instead of placeholders:
  - JavaScript: `expect(true).toBe(true); // TODO` (auto-dev-core/src/test_gen/frameworks/javascript.rs:46)
  - Python: `assert True # TODO` (auto-dev-core/src/test_gen/frameworks/python.rs:46)
- **Add proper test implementation** (auto-dev-core/src/incremental/executor.rs:424)

### Validation System
- **Implement acceptance criteria validation** (auto-dev-core/src/validation/validator.rs:165, 256)
- **Implement parallel validation** (auto-dev-core/src/validation/validator.rs:320)
- **Add validation logic to parse command** (auto-dev/src/cli/commands/parse.rs:132)

### Version Control
- **Detect default branch** instead of hardcoding "main" (auto-dev-core/src/vcs/pr_creator.rs:112, 163)
- **Implement GPG signing** for commits (auto-dev-core/src/vcs/git_ops.rs:137)

### Dogfood Mode Features
- **Integrate synthesis engine** for planned changes (auto-dev/src/cli/commands/dogfood.rs:336, 341)
- **Implement rollback functionality** (auto-dev/src/cli/commands/dogfood.rs:356)
- **Integrate with FileSystemMonitor** for continuous watching (auto-dev/src/cli/commands/self_dev.rs:445)

## Priority: Medium

### Learning System & Embeddings
- **Implement embedding index** (auto-dev-core/src/learning/knowledge_base.rs:47)
- **Implement semantic search** with embeddings (auto-dev-core/src/learning/knowledge_base.rs:76)
- **Rebuild embedding index** functionality (auto-dev-core/src/learning/knowledge_base.rs:255)
- **Replace hash-based embeddings** with actual model (auto-dev-core/src/context/embeddings.rs:307)
- **Generate actual embeddings** (auto-dev-core/src/learning/knowledge_base.rs:260)

### Module System Enhancements
- **Implement proper semver matching** (auto-dev-core/src/modules/registry.rs:259)
- **Implement signature verification** for modules (auto-dev-core/src/modules/store/mod.rs:145)
- **Execute post-install scripts safely** (auto-dev-core/src/modules/store/installer.rs:267)
- **Implement custom tests** in hot-reload (auto-dev-core/src/modules/hot_reload/verifier.rs:225)

### Duration & Metrics Tracking
- **Track actual duration** in executor (auto-dev-core/src/incremental/executor.rs:174)
- **Track actual duration** in validator (auto-dev-core/src/incremental/validator.rs:105)
- **Calculate actual uptime** (auto-dev-core/src/dev_loop/control_server.rs:88)
- **Get actual active tasks** for state preservation (auto-dev-core/src/self_upgrade/state_preserver.rs:73)

### Safety & Analysis
- **Check project builds** in invariants (auto-dev-core/src/safety/invariants.rs:43)
- **Check all tests pass** in invariants (auto-dev-core/src/safety/invariants.rs:57)
- **Check documentation validity** (auto-dev-core/src/safety/invariants.rs:71)
- **Implement contract verification** (auto-dev-core/src/safety/contracts.rs:38)
- **Calculate cyclomatic complexity** (auto-dev-core/src/safety/analyzer.rs:71)
- **Detect code duplication** (auto-dev-core/src/safety/analyzer.rs:92)

### Synthesis Pipeline
- **Replace merger placeholder** code (auto-dev-core/src/synthesis/pipeline/merger.rs:97)
- **Implement actual provider** instead of placeholder (auto-dev-core/src/synthesis/pipeline/generator.rs:181)

## Priority: Low

### Documentation & Polish
- **Add metadata field to Specification** (auto-dev-core/src/test_gen/generator.rs:220)
- **Implement temporary file cleanup** (auto-dev-core/src/dev_loop/health_monitor.rs:116)
- **Replace simple classification heuristics** (auto-dev-core/src/llm/router/classifier.rs:201)
- **Improve analyzer heuristics** (auto-dev-core/src/monitor/analyzer.rs:227)
- **Clean up TODO extractor workaround** for issue #123 (auto-dev-core/src/parser/todo_extractor.rs:429)

### Error Handling Improvements
- Replace 327 unwrap()/expect()/panic! calls with proper Result handling
- Add proper error propagation throughout codebase
- Handle underscore-prefixed unused parameters properly

## Technical Debt

### Placeholder Implementations to Replace
- Simple hash embeddings (auto-dev-core/src/context/embeddings.rs)
- Stub model implementation (auto-dev-core/src/llm/candle.rs:38)
- Classification model placeholder (auto-dev-core/src/llm/router/classifier.rs)
- Generated code placeholders (auto-dev-core/src/synthesis/pipeline/)
- Temporary orchestrator in manual mode (auto-dev/src/cli/commands/self_dev.rs:393)

### "For Now" Code to Refactor
- Monitor analyzer heuristics
- WASM host metadata extraction
- Knowledge base pattern matching
- Source file existence checks (auto-dev-core/src/vcs/history.rs:538)
- Validation result stubs
- Simple version matching in registry (auto-dev-core/src/modules/registry.rs:258)
- Simplified OpenAPI type handling (auto-dev-core/src/parser/openapi.rs:118)
- First-line only for multiline TODOs (auto-dev-core/src/parser/todo_extractor.rs:192)

### Temporary Solutions
- Skip optional dependencies in module resolution (auto-dev-core/src/modules/registry.rs:236)
- Ignore build artifacts and temporary files (auto-dev-core/src/self_monitor/self_monitor.rs:137, 144)
- Placeholder for rand generation (auto-dev-core/src/self_monitor/audit_trail.rs:365)

## Active PRPs (From Gap Analysis)

### Ready for Implementation
- **PRP-260 to PRP-270**: Claude Configuration System (NEW - High Priority)
- **PRP-215**: Self-Development Integration
- **PRP-217**: Claude API Client
- **PRP-218**: OpenAI API Client
- **PRP-219**: Code Synthesis Templates
- **PRP-220**: Prompt Templates
- **PRP-221**: Code Generator Integration
- **PRP-222**: LLM Integration Async OpenAI
- **PRP-223**: CLI Generate Command
- **PRP-224**: Error Handling Core
- **PRP-225**: Test Generation Frameworks
- **PRP-227**: Multi-Provider Orchestration
- **PRP-228**: Groq/Perplexity Providers
- **PRP-229**: Rust GenAI Integration
- **PRP-230**: Fabric Patterns Integration
- **PRP-231**: Claude Code CLI Integration
- **PRP-233-238**: Claude Binary/Command Suite

## Statistics
- **Total TODOs**: 100+ occurrences found in recent scan
- **FIXME items**: 8 critical items
- **HACK comments**: 4 temporary workarounds
- **Unwrap/Panic calls**: 327 in 63 files (non-test code)
- **Underscore-prefixed parameters**: 50+ indicating unused/placeholder code
- **"For now" comments**: 10+ temporary implementations
- **"Actual" references**: 30+ places needing real implementation

## Recent Updates (2025-09-27)
- Generated 11 new PRPs for Claude configuration awareness (PRP-260 to PRP-270)
- Completed Ollama provider (PRP-226) and OpenRouter gateway (PRP-232)
- Fixed test compilation errors in ollama_models.rs
- Updated PRP completion status to 70% (28/40 PRPs complete)
- Established PRP rebalancing strategy for maintaining 70% equilibrium

## Notes
- Windows build requires `-j 1` flag due to memory constraints
- Test compilation failing due to paging file issues
- Many underscore-prefixed parameters indicate unused/placeholder code
- Strong architecture in place, needs concrete implementations
- Claude configuration implementation will significantly enhance user customization capabilities
