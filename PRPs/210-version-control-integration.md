# PRP: Version Control Integration for Self-Development

## Overview
Integrate version control (git) workflows into the self-development process, enabling auto-dev-rs to create branches, commit changes, and manage its own development history.

## Context and Background
When auto-dev-rs modifies itself, these changes should be properly tracked in version control. This enables review, rollback, and collaboration while maintaining a clear history of self-improvements.

### Research References
- Git2 Rust library: https://docs.rs/git2/latest/git2/
- GitOps patterns: https://www.gitops.tech/
- Automated commits: https://github.com/semantic-release/semantic-release
- Branch strategies: https://nvie.com/posts/a-successful-git-branching-model/

## Requirements

### Primary Goals
1. Create feature branches for self-modifications
2. Generate meaningful commit messages
3. Handle merge conflicts
4. Support pull request workflows
5. Maintain clean git history

### Technical Constraints
- Must work with existing git setup
- Cannot corrupt repository
- Should support different workflows
- Must handle authentication
- Should work offline

## Architectural Decisions

### Decision: Branching Strategy
**Chosen**: Feature branches with auto-merge
**Alternatives Considered**:
- Direct main commits: Too risky
- Fork model: Too complex
- Patch files: Not integrated
**Rationale**: Feature branches allow review while supporting automation

### Decision: Commit Granularity
**Chosen**: Logical change grouping
**Alternatives Considered**:
- One commit per file: Too noisy
- Single commit per session: Too coarse
- Time-based: Not logical
**Rationale**: Logical grouping creates reviewable history

## Implementation Blueprint

### File Structure
Create VCS integration in auto-dev-core/src/vcs/
- mod.rs - VCS interface
- git_ops.rs - Git operations
- branch_manager.rs - Branch lifecycle
- commit_builder.rs - Commit message generation
- conflict_resolver.rs - Merge conflict handling
- pr_creator.rs - Pull request automation

### Key Components
1. **VcsIntegration** - Main VCS interface
2. **GitOperations** - Git command wrapper
3. **BranchManager** - Branch creation/merging
4. **CommitBuilder** - Semantic commit messages
5. **ConflictResolver** - Handle merge conflicts

### Implementation Tasks (in order)
1. Create git2-based operations wrapper
2. Implement branch creation/switching
3. Build commit message generator
4. Add staging and committing logic
5. Implement merge operations
6. Create conflict detection
7. Add PR creation via GitHub CLI
8. Build commit verification
9. Implement rollback via git
10. Add git hooks integration

## Git Workflow for Self-Development

### Modification Flow
1. Create feature branch: `auto-dev/feature-name-timestamp`
2. Make modifications
3. Run tests in branch
4. Commit with semantic message
5. Create PR or auto-merge based on config
6. Clean up branch after merge

### Commit Message Format
Follow conventional commits:
- `feat(module): add new capability`
- `fix(synthesis): correct generation bug`
- `refactor(monitor): improve performance`
- `test(self): add validation tests`
- `docs(api): update documentation`

## Validation Gates

```bash
# Test git operations
cargo test vcs::git_ops

# Verify branch creation
cargo run -- vcs create-branch test-feature

# Test commit generation
cargo run -- vcs commit --dry-run

# Check conflict handling
cargo run -- vcs test-conflict-resolution
```

## Success Criteria
- Creates valid git branches
- Generates semantic commit messages
- Handles conflicts gracefully
- Maintains clean history
- Supports GitHub/GitLab workflows

## Known Patterns and Conventions
- Follow conventional commits specification
- Use git2 library patterns
- Match existing CLI git integration
- Reuse GitHub CLI where available

## Common Pitfalls to Avoid
- Don't commit credentials
- Remember to handle detached HEAD
- Avoid force pushes on shared branches
- Don't commit build artifacts
- Consider git hooks interference

## Dependencies Required
- git2 = "0.18" - Git operations
- Optional: octocrab for GitHub API
- Optional: gitlab for GitLab API

## Configuration Options
Example VCS configuration:
- vcs.auto_branch = true
- vcs.branch_prefix = "auto-dev"
- vcs.commit_style = "conventional"
- vcs.auto_merge = false
- vcs.require_tests = true
- vcs.sign_commits = false

## Conflict Resolution Strategy
When conflicts occur:
1. Attempt automatic resolution for simple conflicts
2. Mark complex conflicts for human review
3. Create conflict report with context
4. Option to abort and rollback
5. Support manual resolution workflow

## Confidence Score: 8/10
Git operations are well-understood with good library support. Main complexity is in conflict handling and maintaining clean history during autonomous operation.