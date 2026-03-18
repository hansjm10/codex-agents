# Harness Surfaces

This repository treats harness outputs as product surfaces. A successful run is
not just a changed file or a passing test command. It is a result envelope that
leaves the next agent with legible proof of what happened.

## Required Evidence Surfaces

Every meaningful workflow in this repository should make it easy to discover:

- which assignment ran
- what validations executed
- what passed and failed
- which logs and artifacts were produced
- what a later run should inspect next

The harness should preserve these surfaces in machine-readable form whenever
possible:

- validation results
- replay-ready harness reports
- Codex outputs and summaries
- tool invocation records
- logs and stderr snapshots
- generated artifacts and their locations
- blocked or failure reason envelopes

## Legibility Expectations

Evidence is only useful if a later agent can read it cheaply.

- Prefer stable filenames, schemas, and result shapes.
- Prefer one coherent artifact index over scattered ad hoc outputs.
- Prefer a single replay-ready report artifact that points at the rest of the
  evidence surface.
- Include enough context for a later run to compare failing and passing states.
- Keep error messages and summaries concrete about the failing check, command,
  or missing capability.
- Avoid hidden runtime state that exists only inside prompts or terminal
  scrollback.

## Relationship To Tests

Tests are one part of the harness, not a substitute for it.

- `TESTING.md` defines the validation taxonomy and merge gates.
- The harness is responsible for collecting test output into a surface the next
  agent can inspect.
- Failing validations should emit enough evidence for a later run to debug
  without reconstructing context from humans.
- Passing validations should still leave behind a summary of what was checked.

## Relationship To Runtime Boundaries

- `SPEC.md` defines the product boundary and result contract.
- `ARCHITECTURE.md` defines which layer owns validation and evidence collation.
- `STATEFLOW.md` defines where validation and blocked states appear in the run
  lifecycle.

Together, those top-level documents define the public shape. This file explains
the repository's bar for making that shape legible in practice.
