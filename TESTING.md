# Testing Strategy

This repository does not have implementation yet, but the testing bar should be set before the code starts growing.

## Principles

- Contract stability matters more than clever internals.
- Tool behavior must be tested with deterministic fixtures where possible.
- Skills must be validated against the real tool contract.
- Codex integration should be isolated behind adapter-oriented tests.
- The orchestrator-facing result contract should be cheaper to verify than to accidentally break.

## Planned Test Taxonomy

### Contract Tests

Focus:

- assignment type invariants
- event and result serialization
- tool manifest schema stability

### Tool Wrapper Tests

Focus:

- command construction
- timeout handling
- cwd and env handling
- JSON output normalization

### Skill Validation Tests

Focus:

- referenced tools actually exist in the manifest
- examples remain aligned with the real CLI surface
- forbidden or deprecated commands are caught early

### Codex Adapter Tests

Focus:

- session startup and shutdown behavior
- event normalization
- structured output handling
- failure mode mapping

### Agent Runtime Tests

Focus:

- assignment lifecycle transitions
- event emission ordering
- blocked, failed, and completed result paths

### CLI Smoke Tests

Focus:

- stable binary behavior for local inspection and execution commands
- clear machine-readable error envelopes

### Architecture Boundary Tests

Focus:

- Codex-specific details remain isolated from the public contract layer
- tool execution does not absorb orchestration responsibilities
- skill loading remains guidance-only and does not become hidden runtime policy

## Initial Merge Gates

The first implementation phase should eventually gate on:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Before those gates exist in CI, contributors should still treat them as the expected local validation baseline once code is present.
