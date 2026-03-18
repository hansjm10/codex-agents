# Architecture

## Intent

This repository will implement a Codex-backed agent harness and runtime that executes bounded assignments on behalf of a separate orchestrator. The architecture is intentionally narrower than a full orchestration system and intentionally broader than a thin Codex CLI wrapper.

The core rule is simple:

- the orchestrator decides
- the agent runtime executes
- the harness validates and explains
- tools provide capabilities
- skills provide guidance
- logs and outputs preserve evidence

The harness is a first-class subsystem, not a test afterthought. The repository should make it easy for an AI worker to answer:

- what failed?
- which test or validation proved it?
- where are the artifacts?
- what changed between the failing and passing run?

The same legibility standard applies to repository docs: top-level files define
the architecture boundary, and `docs/` carries the deeper operational guidance
for evidence surfaces and repository knowledge ownership.

## Layering

### `src/domain/`

Pure domain contracts:

- `Assignment`
- `AssignmentConstraints`
- `AgentEvent`
- `AgentResult`
- `HarnessResult`
- `ArtifactIndex`
- `ToolSpec`
- `SkillPackRef`

Rules:

- no filesystem access
- no process spawning
- no Codex integration details

### `src/agent/`

Execution coordination inside a single agent run:

- run lifecycle
- event emission
- assignment validation
- result collation

Rules:

- may depend on `domain`
- may orchestrate Codex and tool invocations
- may not absorb external issue scheduling logic

### `src/harness/`

Validation and evidence layer:

- test orchestration
- result collation
- artifact indexing
- run comparison and replay support

Rules:

- harness output should be machine-readable first
- failing results must preserve enough evidence for a later agent run to debug
- the harness should produce the domain-level `HarnessResult` and `ArtifactIndex`
  contracts that expose test results, Codex outputs, logs, and artifacts
  through one coherent surface

### `src/codex/`

Codex integration boundary:

- session startup
- prompt assembly
- event normalization
- output capture

Rules:

- contains Codex-specific assumptions
- must not become the public contract for the whole system

### `src/tools/`

CLI capability layer:

- tool manifest
- command execution wrappers
- output normalization
- timeout and environment guards

Rules:

- tools are explicit runtime capabilities
- command surfaces should prefer machine-readable output
- tool behavior should not depend on hidden prompt state

### `src/skills/`

Guidance assets for agents:

- skill metadata
- usage guidance for tools
- examples and guardrails

Rules:

- skills teach usage; they are not the execution mechanism
- skills should align with the real CLI contract, not an aspirational one

### `src/store/`

Persistence and replay support:

- run logs
- event envelopes
- artifacts
- harness outputs
- resumable execution metadata

Rules:

- append-friendly where possible
- evidence should stay inspectable by humans and machines

### `src/cli/`

Thin operator and integration surface:

- run an assignment
- run harness validation
- inspect tool manifests
- inspect skill packs
- replay or inspect prior run output

## Dependency Rules

- `domain` depends on nothing internal.
- `agent` may depend on `domain`.
- `codex` may depend on `domain`.
- `harness` may depend on `domain`.
- `tools` may depend on `domain`.
- `skills` may depend on `domain`.
- `store` may depend on `domain`.
- `cli` may depend on all layers.

## Boundary Decisions

- The orchestrator is external to this repository.
- The orchestrator should remain deterministic and must not require Codex.
- Worker agents in this repository may use Codex.
- Harnesses, test surfaces, and debug artifacts are primary product surfaces.
- CLI tools are the default capability model.
- MCP may exist later as an adapter, but it is not the core abstraction.
- Repository-local docs should be the system of record; `AGENTS.md` is only the map.
- Harness evidence expectations should stay explicit in committed docs, not in ad hoc operator knowledge.

## Initial Test Boundary Plan

The first implementation should be validated in these lanes:

- `domain`: contract stability and invariants
- `harness`: validation contracts, result collation, and artifact exposure
- `tools`: wrapper behavior and output normalization
- `skills`: manifest and guidance validation
- `codex`: adapter normalization and session handling
- `agent`: assignment execution and event sequencing
- `cli`: smoke tests for the shipped binary

## Agent Legibility

The repository should stay easy for both humans and agents to navigate:

- the key design decisions live in top-level Markdown files
- deeper guidance lives in `docs/` as the system of record
- intended boundaries are documented before implementation grows around them
- tool behavior is visible in committed specs, not just prompt text
- the orchestrator-facing contract stays stable and reviewable
- failing runs should leave behind enough evidence for the next agent to debug them without asking a human
