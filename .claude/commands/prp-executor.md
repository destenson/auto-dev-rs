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

### 2. Analysis Phase - Determine Implementation Status

For each PRP found, determine if it's been implemented:

**Quick Checks:**
- Does the code/module/feature described exist?
- Use `Glob` to find files that should have been created
- Use `Grep` to search for key classes, functions, or features mentioned
- Check if test files exist for the feature
- Look for TODO comments mentioning the PRP number

**Implementation States:**
- **Not Started**: No code found matching the PRP description
- **Partial**: Some components exist but incomplete
- **Complete**: All major deliverables appear implemented
- **Unknown**: Can't determine status (needs deeper investigation)

### 3. Selection Phase - Choose Next PRP

**Selection Criteria:**
1. **Dependencies Met**: If PRP mentions dependencies, check they exist
2. **Logical Order**: Lower numbered PRPs often come first
3. **Foundation First**: Infrastructure before features
4. **Simplest Viable**: Start with PRPs you can complete fully
5. **Clear Scope**: PRPs with well-defined deliverables

**Red Flags to Skip:**
- Depends on external services not available
- References code that doesn't exist
- Requires domain knowledge not in context
- Blocked by incomplete prerequisites

## Implementation Process

### Step 1: Read and Understand
```
Read "PRPs/{selected-prp}.md"
```
- Identify key deliverables
- Note architectural decisions
- List files to create/modify
- Understand validation criteria

### Step 2: Check Prerequisites
- Verify mentioned dependencies exist
- Check if referenced modules/files are present
- Ensure build system is working
- Confirm tests are passing before starting

### Step 3: Implement
- Create directories and files as specified
- Follow existing code patterns in the project
- Implement incrementally, testing as you go
- Keep changes focused on the PRP scope

### Step 4: Validate
- Run build/compilation
- Execute existing tests to ensure no regression
- Add tests for new functionality
- Run linters if available
- Check for TODO comments you may have added

### Step 5: Document Completion
Options for tracking completion:
- Move PRP to archive directory (if project uses this pattern)
- Create a `.done` or `.complete` marker file
- Update a tracking document (README, dashboard, etc.)
- Add completion note to the PRP itself
- Leave clear TODO comments if partially complete

### Step 6: Version Control (Optional but Recommended)
While git operations can be done manually for review purposes:
- **Manual Review**: Reviewing changes before committing is valuable
- **Clear Messages**: When you commit, reference the PRP number
- **Atomic Commits**: One PRP = one commit (when possible)
- **Work Tracking**: Git history shows PRP implementation progression

*Note: This executor doesn't auto-commit to allow human review of changes*

## Tool Usage Guidelines

### Prefer Native Tools Over Shell Commands
- Use language-specific tools when available (cargo, npm, etc.)
- Use file operation tools (Read, Write, Edit) over shell
- Use Grep/Glob for searching instead of find/grep commands
- Only use shell/bash as last resort

### MCP Tools (if available)
- `mcp__*` tools should be preferred over bash equivalents
- Check what MCP servers are available for your project
- Use specialized tools for builds, tests, and operations

## Common Patterns Across Projects

### Rust Projects
- PRPs often map to modules in `src/`
- Look for `mod.rs` files indicating module structure
- Tests typically in same file or `tests/` directory
- Use `cargo` commands for validation

### Node/TypeScript Projects
- PRPs may map to directories in `src/` or `lib/`
- Look for `index.ts` or feature-specific files
- Tests often in `__tests__/` or `*.test.ts` files
- Use `npm` or `yarn` commands

### Python Projects
- PRPs might map to packages or modules
- Look for `__init__.py` files
- Tests in `tests/` or `test_*.py` files
- Use `pytest` or unittest for validation

## Determining Completion

A PRP is typically complete when:
1. **Core Functionality**: Main features described are working
2. **Tests Pass**: Existing tests still work, new tests added
3. **No Placeholders**: No "TODO: implement" or stub functions
4. **Integration**: Works with rest of system
5. **Validation**: Meets criteria specified in PRP

## Handling Blockers

If you can't complete a PRP:
1. Document what's blocking (missing deps, unclear requirements)
2. Implement what you can
3. Add TODO comments with PRP reference
4. Move to next viable PRP
5. Return to blocked PRPs when dependencies are met

## Gap Analysis Phase - When All PRPs Are Complete

When all existing PRPs are implemented or blocked:

### Step 1: Comprehensive Codebase Review
- Analyze the overall system architecture
- Identify incomplete features or missing capabilities
- Check for TODOs and FIXMEs that suggest needed work
- Review documentation for mentioned but unimplemented features
- Examine test coverage for untested areas

### Step 2: Feature Gap Identification
**Common gaps to look for:**
- Missing error handling or recovery mechanisms
- Incomplete API endpoints or commands
- Lacking monitoring or observability features
- Missing integrations mentioned in docs
- Incomplete user-facing features
- Security or validation gaps
- Absent performance optimizations

### Step 3: Generate New PRPs
For each identified gap:
1. Assess if it's a genuine need vs nice-to-have
2. Determine scope (follow 2-4 hour rule)
3. Check dependencies and prerequisites
4. Use the `/generate-prp` pattern from `~/.claude/commands/generate-prp.md`

### Step 4: PRP Generation Guidelines
When creating new PRPs:
- **Scope**: Each PRP = 2-4 hours of work maximum
- **Atomic**: Single focused feature per PRP
- **Testable**: Must produce verifiable results
- **Context**: Include all necessary documentation
- **No Code**: Focus on WHAT and WHY, not HOW

## Example Execution Flow

```
1. Discover: Found PRPs/001-setup.md through PRPs/010-advanced.md
2. Analyze: 001-003 appear complete (code exists), 004 partial, 005-010 not started
3. Select: Choose 004 since partially done and dependencies met
4. Implement: Complete missing parts of 004
5. Validate: Tests pass, feature works
6. Document: Mark as complete, commit changes
7. Repeat: Move to 005
8. Gap Analysis: All PRPs complete, review codebase for gaps
9. Generate: Create new PRPs for identified missing features
```

## Tips for Success

1. **Start Small**: Pick PRPs you can complete in one session
2. **Verify First**: Always check current state before implementing
3. **Follow Patterns**: Use existing code as template
4. **Test Early and Often**: Run tests frequently during implementation
5. **Clear Commits**: Reference PRP number in commit messages
6. **Stay Focused**: Don't expand beyond PRP scope

## Project-Specific Adaptation

This executor is generic. For specific projects, consider:
- Creating a `PRPs/README.md` with project-specific notes
- Adding `.prp-config` with project conventions
- Documenting any special build/test commands
- Noting technology stack and patterns
- Listing common blockers and solutions

## Success Indicators

You're making progress when:
- PRPs are moving from "not started" to "complete"
- Tests continue passing as you add features
- Code follows consistent patterns
- Each PRP takes roughly similar time to complete
- Blockers are documented and worked around

## Automated PRP Generation

When gaps are identified, generate new PRPs following these principles:

### Research Phase
1. **Codebase Analysis**
   - Search for similar patterns
   - Identify conventions to follow
   - Note existing test approaches

2. **External Research** 
   - Find documentation URLs
   - Locate implementation examples
   - Identify best practices

### PRP Creation Rules
- **NO CODE** in PRPs - focus on requirements
- Include executable validation commands
- Reference existing patterns in codebase
- Break large features into multiple small PRPs
- Each PRP should be independently testable

### Quality Metrics for New PRPs
- Clear scope (2-4 hours of work)
- Complete context and documentation
- Executable validation gates
- Minimal dependencies
- Follows project conventions

Remember: PRPs are guides, not contracts. Implementation details may vary based on what you discover in the code. The goal is to deliver the intended functionality, not necessarily follow the PRP exactly if better approaches become apparent during implementation.

## Continuous Improvement Loop

1. **Execute** existing PRPs
2. **Review** codebase when PRPs complete
3. **Identify** gaps and missing features
4. **Generate** new PRPs for gaps
5. **Prioritize** based on value and dependencies
6. **Repeat** the cycle

This creates a self-improving system where the codebase continuously evolves based on identified needs and gaps.
