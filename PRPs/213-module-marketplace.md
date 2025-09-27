# PRP: Local Module Store and Discovery

**Status**: COMPLETED (2025-09-27) - Local module store implemented with storage, discovery, and installation capabilities

## Overview
Create a local module store system that allows auto-dev-rs to discover, evaluate, and integrate locally stored modules, expanding its capabilities through reusable components.

## Context and Background
A local module store enables auto-dev-rs to manage and reuse self-generated modules efficiently. This creates a local repository where modules can be stored, discovered, and integrated.

### Research References
- Cargo crates.io: https://crates.io/
- NPM registry: https://www.npmjs.com/
- VS Code marketplace: https://marketplace.visualstudio.com/
- WASM package registry: https://wapm.io/

## Requirements

### Primary Goals
1. Discover available modules
2. Evaluate module safety and quality
3. Install compatible modules
4. Share self-generated modules
5. Manage module dependencies

### Technical Constraints
- Must work offline with cache
- Should verify module signatures
- Must sandbox untrusted modules
- Should support private registries
- Must handle version conflicts

## Architectural Decisions

### Decision: Registry Protocol
**Chosen**: Git-based with metadata index
**Alternatives Considered**:
- HTTP API only: Requires server
- P2P network: Too complex
- Filesystem only: No sharing
**Rationale**: Git provides decentralization with simple protocol

### Decision: Trust Model
**Chosen**: Web of trust with sandboxing
**Alternatives Considered**:
- Centralized authority: Single point of failure
- No trust: Too dangerous
- Blockchain: Overengineered
**Rationale**: Web of trust balances security with decentralization

## Implementation Blueprint

### File Structure
Create module store in auto-dev-core/src/modules/store/
- mod.rs - Module store interface
- registry.rs - Registry client
- discovery.rs - Module discovery
- evaluator.rs - Safety/quality evaluation
- installer.rs - Module installation
- publisher.rs - Module publishing
- trust.rs - Trust management

### Key Components
1. **ModuleStore** - Main module store interface
2. **RegistryClient** - Registry communication
3. **ModuleEvaluator** - Quality assessment
4. **TrustManager** - Trust verification
5. **DependencyResolver** - Dependency management

### Implementation Tasks (in order)
1. Define module manifest format
2. Create registry client interface
3. Implement module discovery
4. Build quality evaluation system
5. Add trust verification
6. Create installation system
7. Implement dependency resolution
8. Add module publishing
9. Build module search
10. Create module recommendation engine

## Module Manifest Format
TOML manifest for modules:
```toml
[module]
name = "python-parser"
version = "1.0.0"
description = "Python code parser module"
authors = ["community"]
license = "MIT"
repository = "https://github.com/..."

[capabilities]
provides = ["parser:python"]
requires = ["filesystem:read"]

[compatibility]
auto_dev_version = ">=0.5.0"
platform = ["wasm", "native"]

[verification]
checksum = "sha256:..."
signature = "..."
```

## Validation Gates

```bash
# Search for modules in local store
cargo run -- module-store search parser

# Evaluate module safety
cargo run -- module-store evaluate python-parser

# Install module from local store
cargo run -- module-store install python-parser

# Add module to local store
cargo run -- module-store add ./my-module
```

## Success Criteria
- Discovers relevant modules quickly
- Accurately evaluates module safety
- Installs modules without conflicts
- Supports both public and private registries
- Handles offline mode gracefully

## Known Patterns and Conventions
- Follow Cargo's registry format where applicable
- Use semantic versioning
- Match existing module system interfaces
- Reuse trust patterns from GPG/PGP

## Common Pitfalls to Avoid
- Don't auto-install without verification
- Remember to check compatibility
- Avoid supply chain attacks
- Don't trust self-reported metrics
- Consider module size limits

## Dependencies Required
- Already available: git2, reqwest
- Optional: pgp for signatures
- Optional: blake3 for checksums

## Module Categories
Organize modules by capability:
- **Parsers** - Language parsing
- **Generators** - Code generation
- **Analyzers** - Code analysis
- **Formatters** - Code formatting
- **Validators** - Validation rules
- **Integrations** - External services
- **Optimizers** - Performance improvements

## Trust Levels
Module trust hierarchy:
1. **Core** - Built-in modules
2. **Verified** - Team-reviewed modules
3. **Trusted** - Community-endorsed
4. **Known** - Has reputation
5. **Unknown** - New/unverified

## Confidence Score: 8/10
Local module store is simpler than an external marketplace. Focus on local storage, discovery, and dependency management without external networking complexity.
