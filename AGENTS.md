# Repository Map

This repository is the Codex-backed agent runtime, not the orchestrator.

## Primary Files

- `SPEC.md`
  - Product and system specification for the agent runtime boundary.
- `WORKFLOW.md`
  - Agent execution contract, task envelope expectations, and run posture.
- `ARCHITECTURE.md`
  - Intended module boundaries and dependency rules.
- `STATEFLOW.md`
  - Repository-owned agent run lifecycle policy.
- `TESTING.md`
  - Quality gates and planned test taxonomy.

## Working Rules

- The orchestrator decides what should run next; this repository executes one bounded assignment at a time.
- Codex is used by worker agents, not by the orchestrator.
- CLI tools are first-class capabilities; skills teach the agent when and how to use them.
- Tool contracts should stay stable, explicit, and machine-readable.
- Prefer deterministic local behavior over network-coupled runtime assumptions.
- Every behavior change should be reflected in the top-level docs before the implementation drifts.

## Planned Source Layout

- `.codex/skills/`
- `src/domain/`
- `src/agent/`
- `src/codex/`
- `src/tools/`
- `src/skills/`
- `src/store/`
- `src/cli/`
- `tests/contracts/`
- `tests/tools/`
- `tests/skills/`
- `tests/integration/`

## Forbidden Drift

- Do not grow this repo into the issue orchestrator.
- Do not make MCP the foundational abstraction for local tool usage.
- Do not hide tool contracts only inside prompts or ad hoc shell snippets.
- Do not let Codex-specific assumptions leak into the orchestrator-facing contract.
