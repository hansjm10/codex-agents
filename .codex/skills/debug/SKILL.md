---
name: debug
description:
  Investigate stuck agent-runtime runs and execution failures by tracing local
  run artifacts, logs, and Codex session identifiers.
---

# Debug

## Goals

- Find why an agent-runtime run is stuck, retrying, or failing.
- Correlate an assignment or issue identity to a Codex session quickly.
- Read the right local evidence in the right order to isolate root cause.

## Log Sources

- Primary runtime log: `log/agent-runtime.log`
- Rotated runtime logs: `log/agent-runtime.log*`
- Future run artifacts: `runs/*/events.jsonl`, `runs/*/result.json`
- Local CLI output captured during a manual run

## Correlation Keys

- `assignment_id`
- `issue_id`
- `session_id`

## Quick Triage

1. Confirm the failing assignment, issue, or branch.
2. Find recent lines for the target with `assignment_id` or `issue_id`.
3. Extract `session_id` from matching lines.
4. Trace that `session_id` across start, stream, completion/failure, and stall
   handling logs.
5. Decide the failure class: preparation failure, Codex startup failure, tool
   failure, validation failure, timeout/stall, or blocked result.

## Commands

```sh
rg -n "assignment_id=<assignment-id>" log/agent-runtime.log*
rg -n "issue_id=<issue-id>" log/agent-runtime.log*
rg -o "session_id=[^ ;]+" log/agent-runtime.log* | sort -u
rg -n "session_id=<thread>-<turn>" log/agent-runtime.log*
rg -n "blocked|timeout|failed|validation|Codex session" log/agent-runtime.log*
find runs -maxdepth 2 -type f | sort
```

## Notes

- This skill is intentionally generic until the runtime logging layout is
  implemented.
- Prefer repo-owned run artifacts over ad hoc terminal scrollback when both are
  available.
