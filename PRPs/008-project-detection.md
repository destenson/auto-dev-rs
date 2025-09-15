# PRP: Project Structure Detection System

## Overview
Implement intelligent project detection that identifies project type, language, framework, and structure by analyzing filesystem markers, configuration files, and code patterns.

## Context and Background
Auto-dev needs to understand the project context to provide appropriate suggestions and actions. This includes detecting language, build tools, frameworks, and project conventions.

### Research References
- tokei for language detection: https://github.com/XAMPPRocky/tokei
- ignore crate for gitignore parsing: https://docs.rs/ignore/latest/ignore/
- Project detection patterns from VS Code and other IDEs
- Build tool detection strategies

## Requirements

### Primary Goals
1. Detect programming languages used
2. Identify build systems and package managers
3. Recognize frameworks and libraries
4. Understand project structure and conventions
5. Detect version control configuration

### Technical Constraints
- Fast detection without full file parsing
- Support multiple languages in one project
- Handle nested projects and monorepos
- Respect .gitignore and similar files
- Work with incomplete or broken projects

## Implementation Blueprint

### File Structure
```
src/
├── project/
│   ├── mod.rs           # Project module exports
│   ├── detector.rs      # Main detection logic
│   ├── language.rs      # Language detection
│   ├── framework.rs     # Framework identification
│   ├── structure.rs     # Project structure analysis
│   └── patterns/        # Detection patterns
│       ├── mod.rs
│       ├── rust.rs      # Rust-specific patterns
│       ├── javascript.rs
│       └── python.rs
```

### Key Components
1. **ProjectDetector**: Orchestrates detection process
2. **LanguageDetector**: Identifies programming languages
3. **FrameworkDetector**: Recognizes frameworks/libraries
4. **StructureAnalyzer**: Maps project organization
5. **BuildToolDetector**: Finds build systems

### Detection Markers

#### Language Detection
- File extensions (.rs, .py, .js, .java)
- Shebang lines (#!/usr/bin/env python)
- Configuration files (Cargo.toml, package.json)
- Directory structures (src/, lib/, test/)

#### Framework Detection
- Dependencies in manifest files
- Framework-specific configs
- Directory conventions
- Import patterns

#### Build Tools
- Cargo.toml → Rust/Cargo
- package.json → Node.js/npm
- requirements.txt → Python/pip
- build.gradle → Java/Gradle
- Makefile → Make

### Implementation Tasks (in order)
1. Create src/project module structure
2. Define ProjectInfo struct with detected data
3. Implement file extension mapping
4. Create configuration file detectors
5. Build language detection with priorities
6. Add framework detection from dependencies
7. Implement build tool identification
8. Create project structure analyzer
9. Add monorepo detection
10. Implement caching for detection results
11. Add confidence scoring for detections
12. Create project type inference

## Detection Algorithm

### Phase 1: Quick Scan
1. Check root directory for config files
2. Identify primary language from configs
3. Detect build tools and package managers
4. Note VCS configuration (.git, .hg)

### Phase 2: Deep Analysis
1. Scan src/ and similar directories
2. Count file types and calculate percentages
3. Parse dependency files for frameworks
4. Analyze import statements (sample files)
5. Detect testing frameworks

### Phase 3: Structure Mapping
1. Identify source directories
2. Locate test directories
3. Find documentation paths
4. Map build output directories
5. Detect CI/CD configurations

## Project Type Categories

```rust
enum ProjectType {
    Application,      // Standalone application
    Library,         // Reusable library
    Plugin,          // Plugin for auto-dev
    Monorepo,        // Multiple projects
    Documentation,   // Docs-only project
    Mixed,          // Multiple types
}

struct ProjectInfo {
    root_path: PathBuf,
    project_type: ProjectType,
    languages: Vec<Language>,
    primary_language: Language,
    frameworks: Vec<Framework>,
    build_tools: Vec<BuildTool>,
    structure: ProjectStructure,
    vcs: Option<VersionControl>,
    confidence: f32,
}
```

## Validation Gates

```bash
# Build and test
cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Test detection on sample projects
cargo test project::tests

# Test on real project
cargo run -- project detect .

# Test on various project types
cargo run -- project detect ../sample-rust-project
cargo run -- project detect ../sample-node-project
```

## Success Criteria
- Correctly identifies common project types
- Detects multiple languages in polyglot projects
- Recognizes major frameworks
- Fast detection (<100ms for average project)
- Handles edge cases gracefully

## Known Patterns and Conventions
- Check config files before scanning directories
- Use lazy evaluation for expensive operations
- Cache results with filesystem watching
- Higher weight to root-level indicators
- Consider .gitignore for scan optimization

## Common Pitfalls to Avoid
- Don't scan entire file contents
- Avoid scanning node_modules or vendor
- Don't assume single language
- Remember hidden files (.env, .gitignore)
- Handle symbolic links carefully

## Dependencies Required
- ignore = "0.4"  # Gitignore parsing
- walkdir = "2.0"
- once_cell = "1.0"
- regex = "1.0"
- globset = "0.4"

## Advanced Features
- Custom detection rules via configuration
- Machine learning for uncertain cases
- Integration with language servers
- Historical project analysis
- Template matching for common patterns

## Example Detection Output
```
Project Detection Results:
  Root: /home/user/my-project
  Type: Application
  Primary Language: Rust (95%)
  Other Languages: Shell (3%), TOML (2%)
  Framework: None detected
  Build Tool: Cargo
  Structure: Standard Rust layout
  VCS: Git
  Confidence: 98%
```

## Confidence Score: 9/10
Well-defined patterns with clear detection markers. Similar systems exist in many tools providing good reference implementations.