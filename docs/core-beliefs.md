# Core Beliefs

## Harnesses Are The Product

This repository is not only a runtime for invoking Codex. It is the harness
that makes Codex reliable, testable, and debuggable.

## Repository Knowledge Is The System Of Record

If a rule, design decision, or workflow expectation matters to the agent, it
should live in this repository in a versioned, discoverable form.

## `AGENTS.md` Is A Map

`AGENTS.md` should stay short. It points to deeper sources of truth rather than
trying to carry the entire repository operating manual in one file.

## Tests And Evidence Come Before Convenience

New capabilities should usually add:

- a validation path
- a machine-readable result surface
- artifacts or logs that a later agent can inspect

## Debuggability Must Be Agent-Legible

The target is not just that a human can eventually diagnose a failure. The
target is that a later agent run can discover:

- what failed
- what changed
- what evidence exists
- how to continue

without depending on human memory or external chat history.
