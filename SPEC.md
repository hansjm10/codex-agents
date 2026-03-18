# Agent Runtime Specification

Status: Draft v1 (Rust-first)
Purpose: Define a Codex-backed agent runtime that executes bounded assignments for an external orchestrator while using first-class CLI tools and repository-owned skills.

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
- this repository owns Codex-backed worker execution
- CLI tools are first-class capabilities
- skills teach the agent how and when to use those tools
- the runtime returns structured results and evidence back to the orchestrator

Important boundary:

- this project is not the issue orchestrator
- this project is not the workflow state engine
- this project is not primarily an MCP server

## 2. Goals and Non-Goals

### 2.1 Goals

- Define a stable Rust library boundary for running a single agent assignment.
- Integrate Codex as an execution backend for worker agents.
- Treat CLI tools as first-class runtime capabilities.
- Treat skills as the guidance layer over those CLI capabilities.
- Return structured events, outputs, and terminal results to the caller.
- Keep the orchestrator-facing contract deterministic and inspectable.
- Preserve run evidence for replay, debugging, and auditability.

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

3. `Codex Adapter`
   - Encapsulates how Codex sessions are started and how outputs are normalized.
   - Keeps Codex-specific details out of the public contract.

4. `CLI Tool Layer`
   - Defines approved commands and wrappers.
   - Executes tools with explicit environment, cwd, timeout, and output handling.

5. `Skill Layer`
   - Provides usage guidance for tools and repo workflows.
   - Helps the agent use CLI tools effectively without turning the tools into hidden protocol magic.

6. `Run Store`
   - Persists run metadata, event streams, and artifacts.
   - Supports inspection and replay-oriented workflows.

7. `Operator Surface`
   - CLI-first interface for local runs, inspection, and diagnostics.

### 3.2 Abstraction Levels

1. `Contract Layer`
   - Stable orchestrator-facing types and results.

2. `Execution Layer`
   - Single-run coordination and lifecycle.

3. `Capability Layer`
   - CLI tools and their manifests.

4. `Guidance Layer`
   - Skill packs and tool usage guidance.

5. `Integration Layer`
   - Codex backend integration.

6. `Evidence Layer`
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

### 4.4 Tool Spec

A repository-owned definition of an allowed CLI capability.

Representative fields:

- `name`
- `command`
- `args_schema`
- `supports_json`
- `timeout_policy`
- `usage_examples`

### 4.5 Skill Pack Reference

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

### 5.4 MCP Position

MCP may be supported later as an adapter for compatibility or special cases, but it is not the architectural center of the system.

## 6. Initial Milestones

### Phase 1: Repository Contract

- establish top-level repository docs
- define core types and boundaries
- define the initial tool and skill model

### Phase 2: Runtime Skeleton

- implement assignment execution scaffolding
- implement event emission and result collation
- implement CLI tool manifest loading

### Phase 3: Codex Integration

- integrate Codex as a worker backend
- normalize session output into agent events

### Phase 4: Persistence and Replay

- persist run artifacts and event streams
- support replay and inspection workflows

## 7. Open Questions

- How much of the tool manifest should be static versus discovered at runtime?
- Should skill packs be selected by the orchestrator, by repository config, or both?
- What is the minimum stable result contract the orchestrator needs?
- Which CLI tools belong in this repository versus adjacent repositories?
- When, if ever, should an MCP adapter be introduced as a compatibility layer?
