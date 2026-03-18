# Agent Run Stateflow

This file describes the intended lifecycle for a single agent run in this repository.

It does not own issue workflow state for the larger system. That remains the responsibility of the external state engine and orchestrator.

## Lifecycle Principles

- One assignment maps to one bounded agent run.
- The agent runtime reports what happened; it does not schedule the next assignment.
- Terminal run status is distinct from issue workflow state.
- Blocked is a first-class output, not an informal log message.

## Canonical Run States

- `queued`
- `preparing`
- `running`
- `validating`
- `blocked`
- `completed`
- `failed`
- `cancelled`

## State Intent

### `queued`

The run has been accepted but execution has not started.

### `preparing`

The runtime is validating the assignment, loading skills, and preparing execution context.

### `running`

Codex and the local toolbelt are actively working on the assignment.

### `validating`

Implementation work is paused while required verification runs.

### `blocked`

The run cannot continue without an external dependency, missing capability, or explicit caller decision.

### `completed`

The assignment reached a successful terminal state and produced a final result envelope.

### `failed`

The run ended unsuccessfully due to runtime, tool, or agent failure.

### `cancelled`

The run was intentionally stopped by the caller or supervising system.

## Intended Transitions

- `queued -> preparing`
- `preparing -> running`
- `running -> validating`
- `validating -> running`
- `running -> blocked`
- `validating -> blocked`
- `running -> completed`
- `validating -> completed`
- `preparing -> failed`
- `running -> failed`
- `validating -> failed`
- `queued -> cancelled`
- `preparing -> cancelled`
- `running -> cancelled`
- `validating -> cancelled`
- `blocked -> cancelled`

## Notes

- The orchestrator may map these run states into its own runtime model, but this repository should keep its own local lifecycle explicit.
- A blocked run should always include structured blocker information.
- Completed should require an explicit terminal result, not just absence of new output.
