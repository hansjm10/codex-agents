# Architecture

## Intent

This repository will implement a Codex-backed agent runtime that executes bounded assignments on behalf of a separate orchestrator. The architecture is intentionally narrower than a full orchestration system and intentionally broader than a thin Codex CLI wrapper.

The core rule is simple:

- the orchestrator decides
- the agent runtime executes
- tools provide capabilities
- skills provide guidance
- logs and outputs preserve evidence

## Layering

### `src/domain/`

Pure domain contracts:

- `Assignment`
- `AssignmentConstraints`
- `AgentEvent`
- `AgentResult`
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
- resumable execution metadata

Rules:

- append-friendly where possible
- evidence should stay inspectable by humans and machines

### `src/cli/`

Thin operator and integration surface:

- run an assignment
- inspect tool manifests
- inspect skill packs
- replay or inspect prior run output

## Dependency Rules

- `domain` depends on nothing internal.
- `agent` may depend on `domain`.
- `codex` may depend on `domain`.
- `tools` may depend on `domain`.
- `skills` may depend on `domain`.
- `store` may depend on `domain`.
- `cli` may depend on all layers.

## Boundary Decisions

- The orchestrator is external to this repository.
- The orchestrator should remain deterministic and must not require Codex.
- Worker agents in this repository may use Codex.
- CLI tools are the default capability model.
- MCP may exist later as an adapter, but it is not the core abstraction.

## Initial Test Boundary Plan

The first implementation should be validated in these lanes:

- `domain`: contract stability and invariants
- `tools`: wrapper behavior and output normalization
- `skills`: manifest and guidance validation
- `codex`: adapter normalization and session handling
- `agent`: assignment execution and event sequencing
- `cli`: smoke tests for the shipped binary

## Agent Legibility

The repository should stay easy for both humans and agents to navigate:

- the key design decisions live in top-level Markdown files
- intended boundaries are documented before implementation grows around them
- tool behavior is visible in committed specs, not just prompt text
- the orchestrator-facing contract stays stable and reviewable
