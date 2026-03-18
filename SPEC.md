# Codex-Agents Harness Specification

Status: Draft v1 (Rust-first)
Purpose: Define a Codex-backed agent harness and runtime that executes bounded assignments for an external orchestrator while using first-class CLI tools, repository-owned skills, and test-first validation surfaces.

## 1. Problem Statement

This project exists to isolate agent execution from orchestration.

The target system has at least two major planes:

- the orchestrator, which decides what should run next
- the agent runtime, which performs the work for one bounded assignment

The prior design pressure mixed these concerns:

- should the orchestrator itself use Codex?
- should tools be exposed through MCP or plain CLI commands?
- should the runtime boundary live in prompts or in code?

This specification chooses a narrower, clearer boundary:

- the orchestrator does not use Codex
- this repository owns Codex-backed worker execution and the harness around it
- CLI tools are first-class capabilities
- skills teach the agent how and when to use those tools
- the harness exposes structured results, artifacts, and evidence back to the orchestrator and later agent runs

Important boundary:

- this project is not the issue orchestrator
- this project is not the workflow state engine
- this project is not primarily an MCP server

The primary product is not merely “run Codex.” The primary product is a harness that lets Codex work reliably and lets later agents debug failures cheaply.

## 2. Goals and Non-Goals

### 2.1 Goals

- Define a stable Rust library boundary for running a single agent assignment.
- Define a stable harness boundary for validation, evidence, and replay.
- Integrate Codex as an execution backend for worker agents.
- Treat CLI tools as first-class runtime capabilities.
- Treat skills as the guidance layer over those CLI capabilities.
- Return structured events, outputs, terminal results, and artifact indexes to the caller.
- Keep the orchestrator-facing contract deterministic and inspectable.
- Preserve run evidence for replay, debugging, and auditability.
- Make test results, Codex results, logs, and generated artifacts directly legible to later agent runs.

### 2.2 Non-Goals

- Full issue orchestration, lease scheduling, or multi-agent coordination.
- Owning the canonical issue workflow state machine.
- Designing the final production UI or dashboard.
- Making MCP the required path for all external capabilities.
- Hiding runtime behavior in prompt-only conventions.

## 3. System Overview

### 3.1 Main Components

1. `Assignment Contract`
   - Defines what a worker agent should do.
   - Includes objective, repository/worktree, constraints, allowed tools, and skill references.

2. `Agent Runtime`
   - Owns one bounded run.
   - Starts Codex, manages local execution context, and emits structured events.

3. `Harness Layer`
   - Executes validation workflows.
   - Collects, normalizes, and indexes evidence.
   - Exposes one coherent surface for test results, Codex results, logs, and artifacts.

4. `Codex Adapter`
   - Encapsulates how Codex sessions are started and how outputs are normalized.
   - Keeps Codex-specific details out of the public contract.

5. `CLI Tool Layer`
   - Defines approved commands and wrappers.
   - Executes tools with explicit environment, cwd, timeout, and output handling.

6. `Skill Layer`
   - Provides usage guidance for tools and repo workflows.
   - Helps the agent use CLI tools effectively without turning the tools into hidden protocol magic.

7. `Run Store`
   - Persists run metadata, event streams, and artifacts.
   - Supports inspection and replay-oriented workflows.

8. `Operator Surface`
   - CLI-first interface for local runs, inspection, and diagnostics.

### 3.2 Abstraction Levels

1. `Contract Layer`
   - Stable orchestrator-facing types and results.

2. `Execution Layer`
   - Single-run coordination and lifecycle.

3. `Harness Layer`
   - Validation, result collation, artifact indexing, and replay support.

4. `Capability Layer`
   - CLI tools and their manifests.

5. `Guidance Layer`
   - Skill packs and tool usage guidance.

6. `Integration Layer`
   - Codex backend integration.

7. `Evidence Layer`
   - Logs, artifacts, and replay metadata.

## 4. Core Domain Model

### 4.1 Assignment

A bounded unit of work executed by one agent run.

Representative fields:

- `assignment_id`
- `issue_id` or external work item identifier
- `objective`
- `repo_root`
- `worktree_root`
- `constraints`
- `allowed_tools`
- `skill_packs`

### 4.2 Agent Event

An immutable event emitted during execution.

Representative kinds:

- `run_started`
- `codex_session_started`
- `tool_invoked`
- `tool_completed`
- `artifact_written`
- `validation_completed`
- `blocked`
- `run_completed`

### 4.3 Agent Result

Terminal output returned to the caller.

Representative fields:

- `status`
- `summary`
- `artifacts`
- `observations`
- `validation_results`
- `final_message`

### 4.4 Harness Result

Structured evidence produced by validation and execution.

Representative fields:

- `status`
- `failing_checks`
- `passing_checks`
- `artifact_index`
- `codex_output_refs`
- `log_refs`
- `summary_for_next_agent`

### 4.5 Tool Spec

A repository-owned definition of an allowed CLI capability.

Representative fields:

- `name`
- `command`
- `args_schema`
- `supports_json`
- `timeout_policy`
- `usage_examples`

### 4.6 Skill Pack Reference

A declared bundle of instructions the agent should load for a run.

Representative fields:

- `name`
- `path`
- `version` or revision hint
- `scope`

## 5. Key Design Decisions

### 5.1 Orchestrator Boundary

The orchestrator remains external and deterministic.

It may:

- choose assignments
- prepare worktrees
- poll external systems
- react to agent results

It should not:

- use Codex for its own control flow
- edit code
- own the runtime semantics of one worker run

### 5.2 Tooling Model

The default capability model is CLI-first:

- tools are shipped as commands or wrappers
- command behavior should be explicit and stable
- machine-readable output is preferred

Skills exist to teach usage, not to replace the real command contract.

### 5.3 Codex Role

Codex is the reasoning and execution backend for worker agents.

This repository should not expose a public API that forces callers to understand Codex-specific transport or session details.

### 5.4 Harness Philosophy

This repository should follow a harness-first development model:

- tests and validation are part of the product surface
- failures should produce reusable evidence instead of ephemeral terminal output
- local harnesses should make the application or runtime legible to future agent runs
- adding a new capability should usually include adding a way to validate and inspect it

This follows the harness-engineering pattern OpenAI described on February 11, 2026: repository-local knowledge as the system of record, agent-legible validation surfaces, and explicit evidence loops instead of human memory as the debugging substrate. Source: [Harness engineering: leveraging Codex in an agent-first world](https://openai.com/index/harness-engineering/).

### 5.5 MCP Position

MCP may be supported later as an adapter for compatibility or special cases, but it is not the architectural center of the system.

## 6. Initial Milestones

### Phase 1: Repository Knowledge and Harness Contract

- establish top-level repository docs
- establish `docs/` as the system of record
- define core types, harness outputs, and boundaries
- define the initial tool and skill model

### Phase 2: Harness and Result Surfaces

- implement validation/result collation scaffolding
- expose test results, Codex outputs, and artifact indexes coherently
- make failed runs easy for a later agent to inspect

### Phase 3: Runtime Skeleton

- implement assignment execution scaffolding
- implement event emission and result collation
- implement CLI tool manifest loading

### Phase 4: Codex Integration

- integrate Codex as a worker backend
- normalize session output into agent events

### Phase 5: Persistence and Replay

- persist run artifacts and event streams
- support replay and inspection workflows

## 7. Open Questions

- How much of the tool manifest should be static versus discovered at runtime?
- Should skill packs be selected by the orchestrator, by repository config, or both?
- What is the minimum stable result contract the orchestrator needs?
- Which CLI tools belong in this repository versus adjacent repositories?
- What should the canonical artifact index schema look like for AI-driven debugging?
- Which validation outputs should be summarized versus stored raw?
- When, if ever, should an MCP adapter be introduced as a compatibility layer?
