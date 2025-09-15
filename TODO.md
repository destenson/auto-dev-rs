# TODO List for auto-dev-rs

## Priority: Critical

### Core Implementation
- **Implement all features mentioned in README.md** (README.md:21-23)
  - Code Generation system
  - Project Management tools
  - CI/CD integration
  - Version Control Integration
  - Customizable Workflows
  - Plugin System
  - Documentation Generation
  - Testing Frameworks
  - Monitoring and Analytics
  - Security Features
  - Cloud Integration
  - User-Friendly Interface
  - Multi-language Support

## Priority: High

### Test Implementation
- **Add proper test implementation** (auto-dev-core/src/incremental/executor.rs:420)
- **Implement actual assertion logic** for test frameworks:
  - Python test assertions (auto-dev-core/src/test_gen/frameworks/python.rs:39)
  - JavaScript test assertions (auto-dev-core/src/test_gen/frameworks/javascript.rs:39)

### Duration Tracking
- **Track actual duration** in incremental validator (auto-dev-core/src/incremental/validator.rs:100)
- **Track actual duration** in incremental executor (auto-dev-core/src/incremental/executor.rs:175)

## Priority: Medium

### LLM Router Enhancement
- **Add Claude, OpenAI, etc.** to LLM router (auto-dev-core/src/llm/router.rs:88)

### Command Implementation
- **Implement generate command** functionality (auto-dev/src/cli/commands/generate.rs:5)
- **Implement test command** functionality (auto-dev/src/cli/commands/test.rs:5)
- **Implement docs command** functionality (auto-dev/src/cli/commands/docs.rs:5)
- **Implement deploy command** functionality (auto-dev/src/cli/commands/deploy.rs:5)
- **Implement manage command** functionality (auto-dev/src/cli/commands/manage.rs:5)

### Validation Features
- **Implement actual acceptance criteria validation** (auto-dev-core/src/validation/validator.rs:156-157)
- **Implement missing implementations finder** (auto-dev-core/src/validation/validator.rs:162)
- **Implement behavior scenario validation** (auto-dev-core/src/validation/validator.rs:233)
- **Complete parallel validation implementation** (auto-dev-core/src/validation/validator.rs:320)

### Synthesis Pipeline
- **Replace placeholder code generation** with actual implementation (auto-dev-core/src/synthesis/pipeline/generator.rs:44)

## Priority: Low

### Documentation & Examples
- **Add more detailed documentation and examples** (README.md:25)
- **Implement additional plugins and integrations** (README.md:26)
- **Enhance the user interface for better usability** (README.md:27)
- **Expand support for more programming languages and frameworks** (README.md:28)

### Monitoring & Health
- **Calculate actual uptime** in control server (auto-dev-core/src/dev_loop/control_server.rs:88)
- **Implement actual LLM service call** (auto-dev-core/src/dev_loop/llm_optimizer.rs:175)
- **Implement temporary file cleanup** (auto-dev-core/src/dev_loop/health_monitor.rs:158)

### Specification Metadata
- **Add metadata field to Specification if needed** (auto-dev-core/src/test_gen/generator.rs:221)

## Technical Debt

### Temporary Code ("for now" implementations)
- Command acknowledgment placeholder (auto-dev/src/main.rs:57)
- Rollback cleanup logging (auto-dev-core/src/incremental/rollback.rs:194)
- Direct tool execution in validate command (auto-dev/src/cli/commands/validate.rs:256)
- Source file existence check (auto-dev-core/src/validation/validator.rs:141)
- Placeholder implementations (auto-dev-core/src/validation/validator.rs:171)
- Scenario execution placeholder (auto-dev-core/src/validation/validator.rs:237)
- Sequential execution fallback (auto-dev-core/src/validation/validator.rs:320)
- Empty validation results (auto-dev-core/src/validation/validator.rs:496, 516)
- Queue file saving (auto-dev/src/cli/commands/loop_control.rs:262)

## Self-Development Features (from PRPs)

### TODO and Documentation Specification Parsing (PRP-202)
- Extend existing parser to recognize TODO patterns
- Map TODO markers to Priority enum (FIXME=High, TODO=Medium, HACK=Low)
- Configure file patterns for TODO search
- Add tests for TODO extraction

### Monitor Development (PRP-215)
- Watch for TODOs, issues, specs
- Generate valid specifications from own TODOs

## Notes

- Several functions have underscore parameters indicating unused arguments that may need implementation
- The project has extensive PRP (Project Requirement Proposal) documents detailing planned features
- Configuration examples exist at `.auto-dev/config.toml.example` for reference
