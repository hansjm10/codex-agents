#!/usr/bin/env python3
import asyncio
import json
import random
import re
from dataclasses import dataclass
from datetime import datetime
from typing import Any

POLL_SECONDS = 10
CHECKS_APPEAR_TIMEOUT_SECONDS = 120
CODEX_BOTS = {
    "chatgpt-codex-connector[bot]",
    "github-actions[bot]",
    "codex-gc-app[bot]",
    "app/codex-gc-app",
}
MAX_GH_RETRIES = 5
BASE_GH_BACKOFF_SECONDS = 2


@dataclass
class PrInfo:
    number: int
    url: str
    head_sha: str
    mergeable: str | None
    merge_state: str | None


class RateLimitError(RuntimeError):
    pass


def is_rate_limit_error(error: str) -> bool:
    return "HTTP 429" in error or "rate limit" in error.lower()


async def run_gh(*args: str) -> str:
    max_delay = BASE_GH_BACKOFF_SECONDS * (2 ** (MAX_GH_RETRIES - 1))
    delay_seconds = BASE_GH_BACKOFF_SECONDS
    last_error = "gh command failed"
    for attempt in range(1, MAX_GH_RETRIES + 1):
        proc = await asyncio.create_subprocess_exec(
            "gh",
            *args,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        if proc.returncode == 0:
            return stdout.decode()
        error = stderr.decode().strip() or "gh command failed"
        if not is_rate_limit_error(error):
            raise RuntimeError(error)
        last_error = error
        if attempt >= MAX_GH_RETRIES:
            break
        jitter = random.uniform(0, delay_seconds)
        await asyncio.sleep(min(delay_seconds + jitter, max_delay))
        delay_seconds = min(delay_seconds * 2, max_delay)
    raise RateLimitError(last_error)


async def get_repo_name_with_owner() -> str:
    data = await run_gh("repo", "view", "--json", "nameWithOwner")
    parsed = json.loads(data)
    return parsed["nameWithOwner"]


async def get_pr_info() -> PrInfo:
    data = await run_gh(
        "pr",
        "view",
        "--json",
        "number,url,headRefOid,mergeable,mergeStateStatus",
    )
    parsed = json.loads(data)
    return PrInfo(
        number=parsed["number"],
        url=parsed["url"],
        head_sha=parsed["headRefOid"],
        mergeable=parsed.get("mergeable"),
        merge_state=parsed.get("mergeStateStatus"),
    )


async def get_paginated_list(endpoint: str) -> list[dict[str, Any]]:
    page = 1
    items: list[dict[str, Any]] = []
    while True:
        data = await run_gh(
            "api",
            "--method",
            "GET",
            endpoint,
            "-f",
            "per_page=100",
            "-f",
            f"page={page}",
        )
        batch = json.loads(data)
        if not batch:
            break
        items.extend(batch)
        page += 1
    return items


async def get_issue_comments(repo: str, pr_number: int) -> list[dict[str, Any]]:
    return await get_paginated_list(
        f"repos/{repo}/issues/{pr_number}/comments",
    )


async def get_review_comments(repo: str, pr_number: int) -> list[dict[str, Any]]:
    return await get_paginated_list(
        f"repos/{repo}/pulls/{pr_number}/comments",
    )


async def get_check_runs(repo: str, head_sha: str) -> list[dict[str, Any]]:
    page = 1
    check_runs: list[dict[str, Any]] = []
    while True:
        data = await run_gh(
            "api",
            "--method",
            "GET",
            f"repos/{repo}/commits/{head_sha}/check-runs",
            "-f",
            "per_page=100",
            "-f",
            f"page={page}",
        )
        payload = json.loads(data)
        batch = payload.get("check_runs", [])
        if not batch:
            break
        check_runs.extend(batch)
        total_count = payload.get("total_count")
        if total_count is not None and len(check_runs) >= total_count:
            break
        page += 1
    return check_runs


def parse_time(value: str) -> datetime:
    normalized = value.replace("Z", "+00:00")
    return datetime.fromisoformat(normalized)


def comment_time(comment: dict[str, Any]) -> datetime | None:
    value = comment.get("updated_at") or comment.get("created_at")
    if not value:
        return None
    return parse_time(value)


def is_codex_bot_user(user: dict[str, Any]) -> bool:
    login = (user or {}).get("login")
    return login in CODEX_BOTS


def check_timestamp(check: dict[str, Any]) -> datetime | None:
    for key in ("completed_at", "started_at", "run_started_at", "created_at"):
        value = check.get(key)
        if value:
            return parse_time(value)
    return None


def dedupe_check_runs(check_runs: list[dict[str, Any]]) -> list[dict[str, Any]]:
    latest_by_name: dict[str, dict[str, Any]] = {}
    for check in check_runs:
        name = check.get("name", "unknown")
        timestamp = check_timestamp(check)
        existing = latest_by_name.get(name)
        if existing is None:
            latest_by_name[name] = check
            continue
        existing_timestamp = check_timestamp(existing)
        if timestamp is not None and (
            existing_timestamp is None or timestamp > existing_timestamp
        ):
            latest_by_name[name] = check
    return list(latest_by_name.values())


def summarize_checks(check_runs: list[dict[str, Any]]) -> tuple[bool, bool, list[str]]:
    if not check_runs:
        return True, False, ["no checks reported"]
    check_runs = dedupe_check_runs(check_runs)
    pending = False
    failed = False
    failures: list[str] = []
    for check in check_runs:
        status = check.get("status")
        conclusion = check.get("conclusion")
        name = check.get("name", "unknown")
        if status != "completed":
            pending = True
            continue
        if conclusion not in ("success", "skipped", "neutral"):
            failed = True
            failures.append(f"{name}: {conclusion}")
    return pending, failed, failures


async def main() -> int:
    repo = await get_repo_name_with_owner()
    pr = await get_pr_info()
    initial_sha = pr.head_sha
    checks_deadline = asyncio.get_event_loop().time() + CHECKS_APPEAR_TIMEOUT_SECONDS

    while True:
        pr = await get_pr_info()
        if pr.head_sha != initial_sha:
            print("PR head changed while watching checks.")
            return 4

        issue_comments, review_comments, check_runs = await asyncio.gather(
            get_issue_comments(repo, pr.number),
            get_review_comments(repo, pr.number),
            get_check_runs(repo, pr.head_sha),
        )

        outstanding_review = any(
            not is_codex_bot_user(comment.get("user", {})) for comment in review_comments
        )
        if outstanding_review:
            print("Review comments detected.")
            return 2

        pending, failed, failures = summarize_checks(check_runs)
        if failed:
            print("Checks failed:")
            for failure in failures:
                print(f" - {failure}")
            return 3

        if not pending and check_runs:
            print("Checks green.")
            return 0

        if not check_runs and asyncio.get_event_loop().time() > checks_deadline:
            print("No checks appeared before timeout.")
            return 3

        await asyncio.sleep(POLL_SECONDS)


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
