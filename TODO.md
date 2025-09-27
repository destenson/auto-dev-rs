# TODO List for auto-dev-rs

## Priority: Critical

### Core Implementation Gaps
- **Code Generation System** - Replace placeholder with actual implementation (auto-dev-core/src/synthesis/pipeline/generator.rs:42)
  - Currently returns `// TODO: Implement {task}`
  - Need actual language-specific code generation
  - Integration with LLM providers for synthesis

- **CLI Commands** - All major commands unimplemented
  - Generate command (auto-dev/src/cli/commands/generate.rs:10)
  - Test command (auto-dev/src/cli/commands/test.rs:6)
  - Docs command (auto-dev/src/cli/commands/docs.rs:6)
  - Deploy command (auto-dev/src/cli/commands/deploy.rs:6)
  - Manage command (auto-dev/src/cli/commands/manage.rs:6)

### Memory & Build Issues
- **Fix Windows paging file errors** preventing parallel compilation
- **Resolve test compilation failures** due to memory constraints
- **Fix example compilation errors** (model_validation.rs, tiny_model_demo.rs)

## Priority: High

### LLM Integration
- **Add Claude, OpenAI providers** to router (auto-dev-core/src/llm/router.rs:97)
- **Implement model classification** in Candle (auto-dev-core/src/llm/candle.rs:113, 123)
- **Replace stub implementations** with actual model integration

### Test Framework Completion
- **Implement test assertions** instead of placeholders:
  - JavaScript: `expect(true).toBe(true); // TODO` (auto-dev-core/src/test_gen/frameworks/javascript.rs:45)
  - Python: `assert True # TODO` (auto-dev-core/src/test_gen/frameworks/python.rs:45)
- **Add proper test implementation** (auto-dev-core/src/incremental/executor.rs:424)

### Validation System
- **Implement acceptance criteria validation** (auto-dev-core/src/validation/validator.rs:156-157)
- **Complete behavior scenario validation** (auto-dev-core/src/validation/validator.rs:233)
- **Implement parallel validation** (auto-dev-core/src/validation/validator.rs:320)
- **Add validation logic to parse command** (auto-dev/src/cli/commands/parse.rs:132)

### Dogfood Mode Features
- **Implement file watching** using monitor module (auto-dev/src/cli/commands/dogfood.rs:208)
- **Integrate synthesis engine** for planned changes (auto-dev/src/cli/commands/dogfood.rs:336)
- **Execute synthesis tasks** (auto-dev/src/cli/commands/dogfood.rs:341)
- **Implement rollback functionality** (auto-dev/src/cli/commands/dogfood.rs:356)

## Priority: Medium

### Learning System & Embeddings
- **Implement embedding index** (auto-dev-core/src/learning/knowledge_base.rs:45)
- **Implement semantic search** with embeddings (auto-dev-core/src/learning/knowledge_base.rs:74)
- **Rebuild embedding index** functionality (auto-dev-core/src/learning/knowledge_base.rs:257)
- **Replace hash-based embeddings** with actual model (auto-dev-core/src/context/embeddings.rs:157, 291)

### Module System Enhancements
- **Implement semver matching** for modules (auto-dev-core/src/modules/registry.rs:258)
- **Replace WASM host placeholders** (auto-dev-core/src/modules/wasm_host.rs:53)
- **Implement custom tests** in hot-reload (auto-dev-core/src/modules/hot_reload/verifier.rs:225)

### Duration & Metrics Tracking
- **Track actual duration** in executor (auto-dev-core/src/incremental/executor.rs:174)
- **Track actual duration** in validator (auto-dev-core/src/incremental/validator.rs:104)
- **Calculate actual uptime** (auto-dev-core/src/dev_loop/control_server.rs:88)
- **Get actual active tasks** for state preservation (auto-dev-core/src/self_upgrade/state_preserver.rs:73)

### Synthesis Pipeline
- **Replace merger placeholder** code (auto-dev-core/src/synthesis/pipeline/merger.rs:97)
- **Implement actual provider** instead of placeholder (auto-dev-core/src/synthesis/pipeline/generator.rs:181)

## Priority: Low

### Documentation & Polish
- **Add metadata field to Specification** (auto-dev-core/src/test_gen/generator.rs:220)
- **Implement temporary file cleanup** (auto-dev-core/src/dev_loop/health_monitor.rs:116)
- **Replace simple classification heuristics** (auto-dev-core/src/llm/router/classifier.rs:201)
- **Improve analyzer heuristics** (auto-dev-core/src/monitor/analyzer.rs:227)

### Error Handling Improvements
- Replace 327 unwrap()/expect()/panic! calls with proper Result handling
- Add proper error propagation throughout codebase

## Technical Debt

### Placeholder Implementations to Replace
- Simple hash embeddings (auto-dev-core/src/context/embeddings.rs)
- Stub model implementation (auto-dev-core/src/llm/candle.rs:38)
- Classification model placeholder (auto-dev-core/src/llm/router/classifier.rs)
- Generated code placeholders (auto-dev-core/src/synthesis/pipeline/)

### "For Now" Code to Refactor
- Monitor analyzer heuristics
- WASM host metadata
- Knowledge base pattern matching
- Source file existence checks
- Validation result stubs

### HACK/Workaround Items
- TODO extractor workaround for issue #123 (auto-dev-core/src/parser/todo_extractor.rs:428)

## Self-Development Features (from PRPs)

### Active PRPs Ready for Implementation
- **PRP-215: Self-Development Integration** ⭐ RECOMMENDED NEXT
- PRP-201: Recursive Monitoring
- PRP-205: Dynamic Module System
- PRP-206: Hot-Reload Infrastructure (partially complete)
- PRP-207: Module Sandboxing
- PRP-208: Self-Test Framework
- PRP-209: Bootstrap Sequence
- PRP-210: Version Control Integration
- PRP-211: Self-Improvement Metrics
- PRP-212: Safety Validation Gates
- PRP-213: Module Marketplace
- PRP-214: Self-Documentation

## Statistics
- **Total TODOs**: 467 occurrences across 79 files
- **Unwrap/Panic calls**: 327 in 63 files (non-test code)
- **Compiler warnings**: 149 (mostly unused code)
- **Placeholder implementations**: ~25 major components

## Notes
- Windows build requires `-j 1` flag due to memory constraints
- Test compilation failing due to paging file issues
- Many underscore-prefixed parameters indicate unused/placeholder code
- Strong architecture in place, needs concrete implementations
