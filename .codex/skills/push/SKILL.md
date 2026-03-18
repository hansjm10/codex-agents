---
name: push
description:
  Push current branch changes to origin and create or update the corresponding
  pull request; use when asked to push, publish updates, or create a PR.
---

# Push

## Prerequisites

- `gh` CLI is installed and authenticated.
- The current branch contains the intended changes.

## Goals

- Push current branch changes to `origin` safely.
- Create a PR if none exists for the branch, otherwise update the existing PR.
- Keep branch history clean when the remote has moved.

## Related Skills

- `pull`: use when push is rejected due to non-fast-forward or stale-branch
  issues.

## Steps

1. Identify the current branch and confirm remote state.
2. Run the local validation gate before pushing:
   - `cargo fmt --check`
   - `cargo test`
   - `cargo clippy --all-targets --all-features -- -D warnings`
3. Push branch to `origin` with upstream tracking if needed.
4. If push is rejected:
   - Use the `pull` skill for non-fast-forward or stale-branch issues.
   - Retry push after resolving conflicts and rerunning validation.
   - Use `--force-with-lease` only when history was intentionally rewritten.
5. Ensure a PR exists for the branch:
   - If no PR exists, create one.
   - If a PR exists and is open, refresh its title/body if scope has shifted.
   - If the branch is tied to a closed or merged PR, create a new branch + PR.
6. If `.github/pull_request_template.md` exists, use it as the starting point.
   Otherwise write a concise PR body that covers:
   - Summary
   - Validation
   - Outstanding risks or follow-ups
7. Return the PR URL.

## Commands

```sh
branch=$(git branch --show-current)

cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings

git push -u origin HEAD

pr_state=$(gh pr view --json state -q .state 2>/dev/null || true)
if [ "$pr_state" = "MERGED" ] || [ "$pr_state" = "CLOSED" ]; then
  echo "Current branch is tied to a closed PR; create a new branch + PR." >&2
  exit 1
fi

pr_title="<clear PR title written for this change>"
if [ -z "$pr_state" ]; then
  gh pr create --title "$pr_title"
else
  gh pr edit --title "$pr_title"
fi

gh pr view --json url -q .url
```

## Notes

- Do not use `--force`; use `--force-with-lease` only as a last resort.
- Distinguish sync problems from auth/permission failures.
- Keep PR title/body aligned with the full branch scope, not just the latest
  commit.
