# Agent Workflow Contract

This file describes the intended posture for agent runs launched from this repository.

It is a repository-owned guidance document, not an implementation artifact.

## Runtime Posture

- The caller provides one bounded assignment.
- The runtime loads the declared skills and allowed tool manifest.
- The runtime starts a Codex-backed worker agent for that assignment.
- The worker agent may use approved CLI tools to complete the work.
- The runtime emits structured events and a terminal result envelope.

## Assignment Expectations

Each assignment should provide enough context for one bounded worker run:

- assignment identifier
- objective
- repository path or worktree path
- constraints
- allowed tools
- required skill packs
- expected validation scope

## Worker Expectations

The worker agent should:

- stay inside the provided assignment scope
- prefer repository-owned CLI tools over ad hoc shell improvisation when they exist
- use skills as guidance for tool selection and repo norms
- emit clear blocked or failed outputs when the assignment cannot be completed

The worker agent should not:

- decide what unrelated work to do next
- schedule additional agents
- take over orchestration concerns

## Tool Expectations

Repository-owned CLI tools should aim for:

- stable names
- explicit flags
- machine-readable output where practical
- short help text and examples

## Skill Expectations

Skills should:

- teach the agent when to use a tool
- document constraints and examples
- stay aligned with the actual tool contract

Skills should not:

- invent capabilities that the runtime does not provide
- replace the need for explicit tool manifests

## Result Expectations

A completed run should return:

- terminal run status
- summary
- artifacts
- validation outcomes
- structured observations relevant to the caller

A blocked run should return:

- terminal blocked status
- structured blocker details
- enough evidence for the caller to decide whether to retry, defer, or escalate
