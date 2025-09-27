# PRP: Module Sandboxing and Isolation

**Status**: PARTIAL (2025-09-27) - Basic WASM sandboxing implemented, capability model pending

## Overview
Implement sandboxing and isolation mechanisms for dynamically loaded modules to prevent malicious or buggy modules from compromising the system during self-development.

## Context and Background
When auto-dev-rs loads modules dynamically, especially self-generated ones, it needs strong isolation to prevent untrusted or experimental code from damaging the system. This is critical for safe self-development.

### Research References
- WebAssembly sandbox: https://webassembly.org/docs/security/
- Linux capabilities: https://man7.org/linux/man-pages/man7/capabilities.7.html
- Rust sandboxing: https://github.com/servo/gaol
- Resource limits: https://docs.rs/rlimit/latest/rlimit/

## Requirements

### Primary Goals
1. Isolate module memory access
2. Limit resource consumption
3. Control filesystem access
4. Restrict network access
5. Prevent system calls

### Technical Constraints
- Must work cross-platform
- Cannot significantly impact performance
- Should allow configurable permissions
- Must handle sandbox violations gracefully
- Should support debugging sandboxed modules

## Architectural Decisions

### Decision: Sandboxing Technology
**Chosen**: WASM-based with capability model
**Alternatives Considered**:
- OS-level sandboxing: Platform-specific
- Process isolation: Too much overhead
- Language-level only: Insufficient protection
**Rationale**: WASM provides portable, efficient sandboxing with fine-grained control

### Decision: Permission Model
**Chosen**: Capability-based security
**Alternatives Considered**:
- ACL-based: Too complex
- All-or-nothing: Too restrictive
- Trust levels: Not granular enough
**Rationale**: Capabilities provide precise, composable permissions

## Implementation Blueprint

### File Structure
Create sandboxing in auto-dev-core/src/modules/sandbox/
- mod.rs - Sandbox interface
- wasm_sandbox.rs - WASM isolation
- capabilities.rs - Permission system
- resource_limits.rs - Resource quotas
- audit.rs - Security audit logging
- violations.rs - Violation handling

### Key Components
1. **ModuleSandbox** - Sandbox environment
2. **CapabilityManager** - Permission control
3. **ResourceMonitor** - Track resource usage
4. **ViolationHandler** - Handle security violations
5. **AuditLogger** - Log security events

### Implementation Tasks (in order)
1. Define capability permission model
2. Implement WASM sandbox using wasmtime
3. Create resource limit enforcement
4. Build filesystem access controls
5. Add network access restrictions
6. Implement violation detection
7. Create security audit logging
8. Add sandbox debugging tools
9. Build permission inheritance system
10. Create sandbox test suite

## Capability Model
Modules request capabilities:
- `filesystem:read:/docs` - Read docs directory
- `filesystem:write:/tmp` - Write to temp
- `network:http:localhost` - Local HTTP
- `memory:limit:100MB` - Memory limit
- `cpu:limit:50%` - CPU limit
- `module:call:parser` - Call parser module

## Validation Gates

```bash
# Test sandbox isolation
cargo test sandbox::isolation

# Test resource limits
cargo run -- modules test-limits

# Test capability enforcement
cargo run -- modules test-capabilities

# Test violation handling
cargo run -- modules test-violations
```

## Success Criteria
- Modules cannot access outside sandbox
- Resource limits enforced within 5%
- Capability violations caught immediately
- No performance impact >10%
- Debugging remains possible

## Known Patterns and Conventions
- Follow principle of least privilege
- Use capability-based security patterns
- Match WASI standards where applicable
- Reuse existing permission enums from config

## Common Pitfalls to Avoid
- Don't trust module self-reporting
- Remember to limit CPU time, not just cycles
- Avoid sandbox escape via symlinks
- Don't allow unrestricted module communication
- Consider timing attacks

## Dependencies Required
- Already included: wasmtime (has sandboxing)
- Optional: cap-std for capability-based filesystem

## Security Boundaries
Each module operates within:
- Memory sandbox (WASM linear memory)
- Filesystem sandbox (virtualized paths)
- Network sandbox (filtered connections)
- Time sandbox (CPU quotas)
- Module sandbox (restricted inter-module calls)

## Confidence Score: 8/10
WASM provides strong sandboxing primitives. Main complexity is in designing intuitive capability model and handling edge cases.
