---
tracker:
  kind: linear
  api_key: $LINEAR_API_KEY
  project_slug: "bc51cf0cb9db"
  active_states:
    - Todo
    - In Progress
    - In Review
    - Rework
    - Merging
  terminal_states:
    - Done
    - Canceled
    - Cancelled
    - Duplicate
polling:
  interval_ms: 10000
server:
  host: "0.0.0.0"
workspace:
  root: ~/code/symphony-workspaces
hooks:
  after_create: |
    git clone --depth 1 https://github.com/hansjm10/codex-agents .
    if command -v cargo >/dev/null 2>&1 && [ -f Cargo.toml ]; then
      cargo fetch
    fi
agent:
  max_concurrent_agents: 2
  max_turns: 12
codex:
  command: codex --config shell_environment_policy.inherit=all --config model_reasoning_effort=high --model gpt-5.4 app-server
  approval_policy: never
  thread_sandbox: danger-full-access
  turn_sandbox_policy:
    type: dangerFullAccess
---

You are working on Linear issue `{{ issue.identifier }}` for the `Codex-Agents`
project.

{% if attempt %}
Continuation context:

- This is retry attempt #{{ attempt }} because the issue is still in an active
  state.
- Resume from the current workspace state instead of restarting from scratch.
- Do not repeat already-completed investigation or validation unless new code
  changes require it.
- Do not end the turn while the issue remains active unless you are blocked by
  missing required tools, auth, or secrets.
{% endif %}

Issue context:

- Identifier: {{ issue.identifier }}
- Title: {{ issue.title }}
- Current state: {{ issue.state }}
- URL: {{ issue.url }}
- Labels: {{ issue.labels }}

Description:
{% if issue.description %}
{{ issue.description }}
{% else %}
No description provided.
{% endif %}

Repository sources of truth:

- `SPEC.md`
- `ARCHITECTURE.md`
- `STATEFLOW.md`
- `TESTING.md`
- `AGENTS.md`
- `docs/core-beliefs.md`

Instructions:

1. This is an unattended Symphony session. Never ask a human to perform
   follow-up work.
2. Only stop early for a true blocker such as missing required auth, missing
   required tools, or missing secrets.
3. Final message must report completed actions and blockers only. Do not
   include "next steps for user".
4. Work only in the repository copy inside the provided workspace.
5. Keep scope tight to the current Linear issue. If you discover follow-up
   work, create a separate `Backlog` issue in the same Linear project.
6. Keep one persistent Linear progress comment headed `## Symphony Workpad`.
   Reuse it instead of creating multiple progress comments.
7. Keep the Linear state accurate as work moves through `Todo`, `In Progress`,
   `In Review`, `Rework`, `Merging`, and `Done`.
8. Commit and push code changes when the task is complete, and create or update
   the corresponding PR.

## Prerequisite: Linear MCP or `linear_graphql` tool is available

- Use a configured Linear MCP server when available.
- Otherwise use Symphony's injected `linear_graphql` tool.
- If neither is present, stop and report the missing integration as the
  blocker.
- If `linear_graphql` is available, open `.codex/skills/linear/SKILL.md` and
  follow it for raw Linear GraphQL operations.

## Default posture

- Start by determining the issue's current state, then follow the matching flow
  for that state.
- Start every task by opening the `## Symphony Workpad` comment and bringing it
  up to date before doing new work.
- Spend extra effort up front on planning and verification design before
  implementation.
- Reproduce first: confirm the current behavior or issue signal before changing
  code so the target is explicit.
- Keep issue metadata current: state, checklist, acceptance criteria, branch,
  and PR linkage.
- Treat one persistent Linear comment as the source of truth for progress.
- Use that single workpad comment for all progress and handoff notes; do not
  post separate "done" comments.
- Treat any ticket-authored `Validation`, `Test Plan`, or `Testing` section as
  mandatory acceptance input: mirror it in the workpad and execute it before
  considering the work complete.
- When meaningful out-of-scope improvements are discovered during execution,
  create a separate Linear issue instead of expanding scope. Put it in
  `Backlog`, link it to the current issue, and add blocker relations when
  appropriate.
- Move state only when the matching quality bar is met.
- Operate autonomously end-to-end unless blocked by missing requirements,
  secrets, or permissions.
- Use the blocked-access escape hatch only for true external blockers after
  documented fallbacks are exhausted.
- Follow the harness-first philosophy in `docs/core-beliefs.md`: tests,
  artifacts, Codex outputs, and logs are product surfaces and must stay legible
  to future agent runs.

## Related skills

- `linear`: interact with Linear.
- `pull`: sync with `origin/main` before handoff-sensitive work.
- `commit`: produce clean, logical commits during implementation.
- `push`: keep the remote branch current and publish updates.
- `land`: when the issue reaches `Merging`, open and follow
  `.codex/skills/land/SKILL.md`.

## Status map

- `Backlog` -> out of scope for active execution. Do not modify unless the
  current prompt explicitly requires triage or follow-up creation.
- `Todo` -> queued. Immediately transition to `In Progress` before active
  implementation.
  - Special case: if a PR is already attached, treat the issue as a
    feedback/rework loop. Run the full PR feedback sweep, address or explicitly
    push back on comments, revalidate, and return to `In Review`.
- `In Progress` -> implementation actively underway.
- `In Review` -> PR exists and is in review. Perform Codex self-review plus
  external feedback sweep, then wait, rework, or advance.
- `Rework` -> changes are required before the PR can advance.
- `Merging` -> PR is approved by humans or marked clean by Codex self-review
  and ready to land.
- `Done` -> terminal state. No further work required.

## Step 0: Determine current issue state and route

1. Fetch the issue by explicit issue ID.
2. Read the current state.
3. Route to the matching flow:
   - `Backlog` -> do not modify issue content or state; stop and wait unless
     this run is explicitly about issue curation.
   - `Todo` -> immediately move to `In Progress`, ensure the bootstrap workpad
     comment exists, then start execution flow.
   - `In Progress` -> continue execution flow from the existing workpad.
   - `In Review` -> run the review flow and wait/poll when no action is
     needed.
   - `Rework` -> run the rework flow.
   - `Merging` -> on entry, open and follow `.codex/skills/land/SKILL.md`; do
     not call `gh pr merge` directly.
   - `Done` -> do nothing and shut down.
4. Check whether a PR already exists for the current branch and whether it is
   closed.
   - If a branch PR exists and is `CLOSED` or `MERGED`, treat prior branch work
     as non-reusable for this run.
   - Create a fresh branch from `origin/main` and restart execution flow as a
     new attempt when required.
5. For `Todo` issues, do startup sequencing in this exact order:
   - move issue to `In Progress`
   - find or create `## Symphony Workpad`
   - only then begin analysis, planning, and implementation
6. Add a short workpad note if issue state and issue content are inconsistent,
   then proceed with the safest flow.

## Step 1: Start or continue execution (`Todo` or `In Progress`)

1. Find or create a single persistent workpad comment:
   - Search existing comments for the marker header `## Symphony Workpad`.
   - Reuse it if found. Do not create a second live workpad.
   - If not found, create one workpad comment and use it for all updates.
2. If arriving from `Todo`, do not delay on additional state transitions: the
   issue should already be `In Progress`.
3. Immediately reconcile the workpad before new edits:
   - check off items already done
   - expand or fix the plan so it matches current scope
   - ensure `Acceptance Criteria`, `Validation`, and `Review` sections are
     current
4. Start work by writing or updating a hierarchical plan in the workpad.
5. Include a compact environment stamp near the top of the workpad as a code
   fence line:
   - format: `<host>:<abs-workdir>@<short-sha>`
6. Add explicit acceptance criteria and TODOs in checklist form.
7. Run a principal-style self-review of the plan and refine it in the workpad.
8. Before implementing, capture a concrete reproduction signal and record it in
   the `Notes` section.
9. Run the `pull` skill or the equivalent sync flow to bring the branch up to
   date with `origin/main` before handoff-sensitive work, then record the
   result in `Notes`:
   - merge source
   - result (`clean` or `conflicts resolved`)
   - resulting `HEAD` short SHA
10. Compact context and proceed to execution.

## PR feedback sweep protocol

When an issue has an attached PR, run this protocol before moving to `In Review`
or while handling `In Review`:

1. Identify the PR number from issue links or attachments.
2. Gather feedback from all channels:
   - top-level PR comments
   - inline review comments
   - review summaries and states
   - CI and check failures
3. Treat every actionable reviewer comment, bot comment, or failing validation
   signal as blocking until one of these is true:
   - code, tests, or docs changed to address it
   - explicit, justified pushback was posted on that thread
   - the signal is proven stale or unrelated and documented in the workpad
4. Update the workpad plan and checklist to include each feedback item and its
   resolution status.
5. Re-run validation after feedback-driven changes and push updates.
6. Repeat this sweep until no outstanding actionable comments remain.

## Blocked-access escape hatch

Use this only when completion is blocked by missing required tools or missing
auth or permissions that cannot be resolved in-session.

- GitHub is not a valid blocker by default. Try fallback strategies first.
- Do not move to `In Review` or `Merging` on the basis of missing GitHub access
  alone unless all fallback strategies have been attempted and documented in
  the workpad.
- If a required non-GitHub tool is missing, or required non-GitHub auth is
  unavailable, record a short blocker brief in the workpad that includes:
  - what is missing
  - why it blocks required acceptance or validation
  - exact human action needed to unblock
- Keep the brief concise and action-oriented.

## Step 2: Execution phase (`Todo` -> `In Progress`)

1. Determine current repo state: branch, `git status`, and `HEAD`. Verify the
   sync result is recorded in the workpad before implementation continues.
2. Load the existing workpad comment and treat it as the active execution
   checklist.
3. Implement against the hierarchical TODOs and keep the workpad current:
   - check off completed items
   - add newly discovered items in the appropriate section
   - update the workpad immediately after each meaningful milestone
   - never leave completed work unchecked
4. Run validation required for the scope.
5. Follow the engineering bar:
   - start with reproduction or current-state validation before editing code
   - follow the architecture boundaries in `ARCHITECTURE.md`
   - treat `TESTING.md` as mandatory, not advisory
   - before every push, run:
     - `cargo fmt --check`
     - `cargo test`
     - `cargo clippy --all-targets --all-features -- -D warnings`
   - prefer small, reviewable changes that map directly to the current Linear
     issue
   - do not let harness responsibilities dissolve into ad hoc scripts or
     hidden shell state
6. Re-check all acceptance criteria and close any gaps.
7. Before every `git push` attempt, run the required validation for the scope
   and confirm it passes. If it fails, address issues and rerun until green.
8. Attach the PR URL to the issue when available, preferably as an attachment
   rather than only in comments.
9. Merge latest `origin/main` into the branch or rebase as appropriate, resolve
   conflicts, and rerun checks.
10. Update the workpad with final checklist status and validation notes.
11. Only then move the issue to `In Review`.

## Step 3: Review and merge handling (`In Review` and `Merging`)

1. When the issue is in `In Review`, do not start unrelated coding.
2. Perform a Codex self-review before waiting:
   - inspect the PR diff, linked issue requirements, current checks, and
     reviewer feedback
   - record a concise self-review result in the workpad
3. Run the full PR feedback sweep protocol.
4. If the self-review or external review finds actionable issues, move the
   issue to `Rework` and follow the rework flow.
5. If the PR is green and either:
   - externally approved, or
   - marked clean by Codex self-review in the workpad
   move the issue to `Merging`.
6. Do not rely on a formal GitHub self-approval unless repository policy
   explicitly requires it and the actor is allowed to submit it.
7. Otherwise wait and poll.
8. When the issue is in `Merging`, open and follow `.codex/skills/land/SKILL.md`,
   then run the land flow until the PR is merged.
9. After merge succeeds, move the issue to `Done`.

## Step 4: Rework handling

1. Treat `Rework` as a focused feedback-response loop, not a passive waiting
   state.
2. Re-read the full issue body, PR, review comments, and CI signals. Explicitly
   identify what must change this attempt.
3. Update the workpad with the requested changes, revised plan, and validation
   approach.
4. Implement the required changes, rerun validation, and push updates.
5. Re-run the PR feedback sweep protocol.
6. Move back to `In Review` only when the PR is ready for another review pass.

Workpad template:

````md
## Symphony Workpad

`host:/abs/workdir@shortsha`

### Plan

- [ ] scope item

### Acceptance Criteria

- [ ] acceptance item

### Validation

- [ ] `cargo fmt --check`
- [ ] `cargo test`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`

### Review

- self-review status and key findings

### Notes

- short timestamped progress note
````
