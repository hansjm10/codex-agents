---
name: land
description:
  Land a PR by monitoring conflicts, resolving them, waiting for checks, and
  squash-merging when green.
---

# Land

## Goals

- Ensure the PR is conflict-free with `main`.
- Keep CI green and fix failures when they occur.
- Squash-merge the PR once checks pass.
- Do not stop until the PR is merged unless blocked.

## Preconditions

- `gh` CLI is authenticated.
- You are on the PR branch with a clean working tree.

## Steps

1. Locate the PR for the current branch.
2. Confirm the full local gauntlet is green:
   - `cargo fmt --check`
   - `cargo test`
   - `cargo clippy --all-targets --all-features -- -D warnings`
3. If the working tree has uncommitted changes, use the `commit` skill and
   then the `push` skill.
4. Check mergeability and conflicts against `main`.
5. If conflicts exist, use the `pull` skill, then push the updated branch.
6. Ensure review comments are acknowledged and any required fixes are handled.
7. Watch checks until complete.
8. If checks fail, inspect logs, fix the issue, commit, push, and restart the
   watch loop.
9. When all checks are green and review feedback is addressed, squash-merge.

## Commands

```sh
branch=$(git branch --show-current)
pr_number=$(gh pr view --json number -q .number)
pr_title=$(gh pr view --json title -q .title)
pr_body=$(gh pr view --json body -q .body)
mergeable=$(gh pr view --json mergeable -q .mergeable)

if [ "$mergeable" = "CONFLICTING" ]; then
  echo "Run the pull skill, resolve conflicts, rerun validation, then push." >&2
fi

python3 .codex/skills/land/land_watch.py

gh pr merge --squash --subject "$pr_title" --body "$pr_body"
```

## Failure Handling

- If checks fail, use `gh pr checks` and `gh run view --log` to inspect the
  failure.
- Fix locally, rerun:
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
- Then commit, push, and restart the watch.

## Review Handling

- Human review comments are blocking and must be addressed before merge.
- Fetch feedback with:
  - `gh api repos/{owner}/{repo}/pulls/<pr_number>/comments`
  - `gh api repos/{owner}/{repo}/issues/<pr_number>/comments`
- Reply inline when appropriate and keep comments prefixed with `[codex]`.
