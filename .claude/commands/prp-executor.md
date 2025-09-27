# Generic PRP Executor

## Purpose
This document serves as an automated execution guide for an AI agent to discover, analyze, and implement PRPs (Project Requirement Plans) in any project. PRPs are planning documents that describe features or components to be built. This executor helps systematically work through them.

## What Are PRPs?
PRPs (Project Requirement Plans) are detailed blueprints for implementing specific features or components. They typically contain:
- Problem description and context
- Requirements and goals
- Architectural decisions and rationale
- Implementation details
- Validation criteria

## Execution Instructions

### 1. Discovery Phase - Find PRPs
- PRPs are typically in a `PRPs/`, `prps/`, or `docs/prps/` directory
- They are usually markdown files with names like `{number}-{description}.md`
- Some projects archive completed PRPs in `_archive/` or `completed/` subdirectories
- The actual code is the source of truth - PRPs are planning documents

## CRITICAL: Use MCP Tools, Not Bash Commands

**IMPORTANT**: You have access to MCP (Model Context Protocol) tools that MUST be used instead of bash commands. The MCP tools available include:
- `mcp__Cargo__*` - For all Rust/Cargo operations (build, test, run, add dependencies, etc.)
- File operations through native tools (Read, Write, Edit, MultiEdit, LS, Glob, Grep)
- Other tools as available in your environment

**DO NOT** use the Bash tool for operations that have dedicated MCP tools available. Only use Bash as a last resort for git operations.

## Finding the Next PRP

1. **Check the dashboard:**
   ```
   Read "PRPs/README.md"
   ```

2. **Find PRPs needing implementation:**
   - Look for PRPs with status "NOT STARTED" or "PARTIAL"
   - Prioritize PRP-215 (Self-Development Integration) as recommended
   - Check dependencies in each PRP's markdown file

3. **Pick the best PRP based on:**
   - Dependencies completed (check "Dependencies" section in PRP)
   - Strategic importance (PRP-215 enables self-development)
   - Current technical debt (see TODO.md for gaps)

## Execution Pattern

For each PRP:

1. **Read the PRP:**
   ```
   Read "PRPs/{number}-{name}.md"
   ```

2. **Verify prerequisites:**
   - Check the "Dependencies" section
   - Ensure required modules exist
   - Review "Requirements" and "Architectural Decisions" sections

3. **Implement according to the PRP document:**
   - Create modules in `auto-dev-core/src/` as specified
   - Follow the "Implementation Blueprint" section exactly
   - Maintain consistency with existing architecture
   - Use existing patterns from completed modules

4. **Update tests:**
   - Add unit tests in the module
   - Add integration tests if specified
   - Ensure existing tests still pass

5. **Validate:**
   ```
   mcp__Cargo__cargo_build with project_path: "C:\\Users\\deste\\repos\\auto-dev-rs"
   mcp__Cargo__cargo_test with project_path: "C:\\Users\\deste\\repos\\auto-dev-rs"
   mcp__Cargo__cargo_clippy with project_path: "C:\\Users\\deste\\repos\\auto-dev-rs"
   ```

6. **Update PRP status:**
   ```
   Edit "PRPs/{number}-{name}.md" to add/update:
   **Status**: COMPLETE (YYYY-MM-DD) - All deliverables implemented
   ```

7. **Update dashboard:**
   ```
   Edit "PRPs/README.md" to reflect completion
   ```

8. **Update TODO.md:**
   - Remove completed items
   - Add any new technical debt discovered

9. **Commit immediately:**
   ```
   git add -A
   git commit -m "Implement PRP-{number}: {title}
   
   - {list key deliverables}
   - {note any deviations}
   
   Co-Authored-By: Claude <noreply@anthropic.com>"
   ```

## Project Structure

Key directories in auto-dev-rs:
- `auto-dev-core/src/` - Core library implementation
  - `context/` - Project understanding and analysis
  - `dev_loop/` - Main development loop orchestration
  - `incremental/` - Incremental implementation engine
  - `learning/` - Pattern extraction and knowledge base
  - `llm/` - LLM integration and routing
  - `modules/` - Dynamic module system with hot-reload
  - `monitor/` - Filesystem monitoring
  - `parser/` - Specification parsing
  - `synthesis/` - Code generation pipeline
  - `test_gen/` - Test generation
  - `validation/` - Code validation
- `auto-dev/src/cli/` - CLI commands
- `PRPs/` - Project Requirement Plans
- `models/` - Local model files

## Priority PRPs for Implementation

Based on the current state:

1. **PRP-215: Self-Development Integration** (RECOMMENDED NEXT)
   - Ties together all existing infrastructure
   - Enables autonomous PRP implementation
   - Orchestrates self-development workflows

2. **PRP-212: Safety Validation Gates** (PARTIAL)
   - Critical for safe autonomous operation
   - Build on existing validation module

3. **PRP-201: Recursive Self-Monitoring** (PARTIAL)
   - Enhance monitoring for self-awareness
   - Add loop prevention and audit trails

4. **PRP-207: Module Sandboxing** (PARTIAL)
   - Complete security features
   - Add capability model and resource limits

5. **PRP-208: Self-Test Framework**
   - Validate changes before deployment
   - Ensure safe self-modification

## Validation Gates

Each PRP implementation must pass:
1. **Compilation**: Clean build with no errors
2. **Tests**: All existing tests pass, new tests added
3. **Warnings**: Address compiler warnings (149 currently)
4. **Integration**: Works with existing modules
5. **Documentation**: Update relevant docs and comments

## Common Patterns to Follow

Look at completed modules for patterns:
- **Module Structure**: See `modules/` for registry, runtime, interface patterns
- **Error Handling**: Use `Result<T>` and `anyhow::Result`
- **Async Operations**: Use `tokio` for all async code
- **Configuration**: Use `serde` for serialization
- **Testing**: Unit tests in module, integration tests in `tests/`

## Completion Status

PRP is complete when:
- All deliverables from "Requirements" section implemented
- Tests pass (unit and integration)
- Documentation updated
- Status marked as COMPLETE in PRP file
- Dashboard updated in PRPs/README.md
- No placeholder code remains (no "TODO: Implement")

## Notes for the AI Agent

- **Source of Truth**: The source code is truth, not documentation
- **Incremental Progress**: Each PRP builds on previous ones
- **Technical Debt**: Track TODOs and add to TODO.md
- **Memory Constraints**: Use `-j 1` for builds on Windows if memory errors occur
- **Existing Examples**: Reference completed modules for patterns
- **Safety First**: PRPs involving self-modification need extra care
- **Dashboard Maintenance**: Keep PRPs/README.md current

## Execution Priority Algorithm

```
1. Check dependencies are met
2. Prioritize based on:
   - Enables other PRPs (e.g., PRP-215)
   - Fixes critical gaps (safety, testing)
   - Completes partial implementations
   - Strategic value for autonomy
3. Implement smallest complete unit
4. Validate thoroughly before marking complete
```

## Common Issues and Solutions

- **Memory errors during build**: Use `mcp__Cargo__cargo_build` with args `["-j", "1"]`
- **Test compilation fails**: Check for missing dependencies or example issues
- **Placeholder code**: Replace all "TODO: Implement" with actual implementation
- **Integration conflicts**: Review existing module interfaces before adding new ones

## Success Criteria

The auto-dev-rs system is ready when:
- PRP-215 enables self-development
- Safety gates prevent dangerous modifications
- System can implement its own TODOs
- Learning system improves over time
- All critical PRPs (201, 207, 208, 212, 215) complete

Remember: The goal is autonomous development that can safely modify itself, learn from implementations, and continuously improve. Each PRP adds a critical capability toward this vision.