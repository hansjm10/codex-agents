# Repository Knowledge Base

This repository keeps its durable operating knowledge in committed Markdown.
Top-level documents establish the primary contracts and boundaries. The
`docs/` directory carries the deeper system of record for guidance that needs
more room to explain intent, evidence expectations, and repository navigation.

## Document Roles

Use the top level for short, stable entry points:

- `AGENTS.md`
  - Fast repository map for agents and humans.
- `SPEC.md`
  - Product boundary, goals, and harness contract.
- `ARCHITECTURE.md`
  - Layering, dependencies, and ownership boundaries.
- `STATEFLOW.md`
  - Repository-owned agent run lifecycle.
- `TESTING.md`
  - Validation philosophy, merge gates, and test taxonomy.
- `WORKFLOW.md`
  - Session envelope and execution instructions used by the harness.

Use `docs/` for deeper repository knowledge:

- `docs/index.md`
  - Table of contents for deeper guidance.
- `docs/core-beliefs.md`
  - Short statement of repository principles.
- `docs/repository-knowledge-base.md`
  - This file; explains where durable guidance should live.
- `docs/harness-surfaces.md`
  - Detailed expectations for tests, logs, artifacts, and debug evidence.

## Placement Rules

- If guidance defines a stable contract or repository boundary, keep it in a
  top-level source-of-truth document.
- If guidance expands on how to interpret, validate, debug, or extend those
  contracts, put it in `docs/`.
- If a rule matters to a future agent run, commit it to the repository instead
  of relying on chat history or informal memory.
- If a top-level document starts carrying too much operational detail, move the
  detail into `docs/` and leave behind a durable pointer.

## Change Discipline

- Update docs before implementation drifts beyond the documented boundary.
- Keep cross-references explicit so later runs can find the right source
  quickly.
- Prefer a few well-scoped documents over one oversized catch-all narrative.
- Keep examples and guidance aligned with the actual harness, tool, and test
  surfaces shipped by the repository.
