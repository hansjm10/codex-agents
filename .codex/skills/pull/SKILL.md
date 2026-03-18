---
name: pull
description:
  Pull latest origin/main into the current local branch and resolve merge
  conflicts (aka update-branch). Use when Codex needs to sync a feature branch
  with origin, perform a merge-based update (not rebase), and guide conflict
  resolution best practices.
---

# Pull

## Workflow

1. Verify git status is clean or commit/stash changes before merging.
2. Ensure rerere is enabled locally:
   - `git config rerere.enabled true`
   - `git config rerere.autoupdate true`
3. Confirm remotes and branches:
   - Ensure the `origin` remote exists.
   - Ensure the current branch is the one to receive the merge.
4. Fetch latest refs:
   - `git fetch origin`
5. Sync the remote feature branch first:
   - `git pull --ff-only origin $(git branch --show-current)`
6. Merge in order:
   - Prefer `git -c merge.conflictstyle=zdiff3 merge origin/main`
7. If conflicts appear, resolve them, then:
   - `git add <files>`
   - `git commit` or `git merge --continue`
8. Verify with repo checks:
   - `cargo fmt --check`
   - `cargo test`
   - `cargo clippy --all-targets --all-features -- -D warnings`
9. Summarize the merge:
   - Call out the most challenging conflicts/files and how they were resolved.
   - Note any assumptions or follow-ups.

## Conflict Resolution Guidance

- Inspect context before editing:
  - Use `git status` to list conflicted files.
  - Use `git diff` or `git diff --merge` to see conflict hunks.
  - Use `git diff :1:path :2:path` and `git diff :1:path :3:path` to compare
    base vs ours/theirs.
- Prefer minimal, intention-preserving edits.
- Resolve one file at a time and rerun checks after each logical batch.
- Use `ours/theirs` only when one side should win entirely.
- For generated files, resolve source files first, then regenerate.
- Ensure no conflict markers remain:
  - `git diff --check`

## When To Ask The User

Ask only when there is no safe, reversible alternative, such as:

- The correct resolution depends on product intent not inferable from code,
  tests, or docs.
- The conflict crosses a user-visible contract or external API with no clear
  local signal.
- The merge introduces schema/data-loss risk with no safe default.
