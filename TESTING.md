# Testing Strategy

This repository does not have implementation yet, but the testing and harness bar should be set before the code starts growing.

## Principles

- Contract stability matters more than clever internals.
- Harnesses are product features, not support code.
- Repository-local docs should explain the validation and evidence model before implementation complexity grows.
- Tool behavior must be tested with deterministic fixtures where possible.
- Skills must be validated against the real tool contract.
- Codex integration should be isolated behind adapter-oriented tests.
- The orchestrator-facing result contract should be cheaper to verify than to accidentally break.
- Every failing validation path should leave behind machine-readable evidence for a later agent run.
- Test results, Codex outputs, and artifacts should be exposed through one legible surface.

## Planned Test Taxonomy

### Contract Tests

Focus:

- assignment type invariants
- event and result serialization
- tool manifest schema stability
- harness result schema stability

### Harness Tests

Focus:

- validation orchestration
- artifact indexing
- test result normalization
- summary generation for later agent debugging

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
- artifact/log references emitted by Codex runs

### Agent Runtime Tests

Focus:

- assignment lifecycle transitions
- event emission ordering
- blocked, failed, and completed result paths

### CLI Smoke Tests

Focus:

- stable binary behavior for local inspection, execution, and result-reporting commands
- clear machine-readable error envelopes

### Architecture Boundary Tests

Focus:

- Codex-specific details remain isolated from the public contract layer
- tool execution does not absorb orchestration responsibilities
- skill loading remains guidance-only and does not become hidden runtime policy
- harness responsibilities do not dissolve into ad hoc test scripts

## Initial Merge Gates

The first implementation phase should eventually gate on:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Before those gates exist in CI, contributors should still treat them as the expected local validation baseline once code is present.

## Harness Philosophy

This repository should follow a harness-engineering posture:

- start by making the system testable and inspectable
- make validation outputs directly legible to agents
- treat logs, test results, Codex outputs, and artifacts as primary debugging inputs
- optimize for the next agent run being able to continue without human reconstruction

The target is not just “tests pass.” The target is “an AI can tell what failed, why, and where the evidence lives.”

For the deeper description of those evidence surfaces, see
`docs/harness-surfaces.md`.
