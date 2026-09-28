#!/usr/bin/env python3
"""Wait for the complete CI workflow run for one immutable release commit."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen


API_VERSION = "2022-11-28"
POLL_SECONDS = 20
REQUIRED_JOB_COUNTS = {
    "Repo invariants": 1,
    "Platform build (": 3,
    "Rust regression (": 3,
    "Platform smokes (": 3,
    "Client validation (": 3,
    "Native desktop (": 3,
    "Headless no-GUI link check": 1,
}


class EvidenceError(Exception):
    """A precise reason same-commit CI cannot authorize publication."""


def github_json(url: str, token: str) -> dict[str, object]:
    request = Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "X-GitHub-Api-Version": API_VERSION,
            "User-Agent": "msc2-release-gate",
        },
    )
    try:
        with urlopen(request, timeout=30) as response:
            value = json.load(response)
    except (HTTPError, URLError, TimeoutError, json.JSONDecodeError) as error:
        raise EvidenceError(f"GitHub Actions API request failed: {error}") from error
    if not isinstance(value, dict):
        raise EvidenceError("GitHub Actions API returned an unexpected response")
    return value


def runs_for_commit(
    api_base: str, repository: str, token: str, sha: str, tag: str
) -> list[dict[str, object]]:
    query = urlencode({"event": "push", "head_sha": sha, "per_page": 100})
    response = github_json(
        f"{api_base}/repos/{repository}/actions/workflows/ci.yml/runs?{query}", token
    )
    runs = response.get("workflow_runs")
    if not isinstance(runs, list):
        raise EvidenceError("CI workflow run lookup returned no workflow_runs list")
    return [
        run
        for run in runs
        if isinstance(run, dict)
        and run.get("event") == "push"
        and run.get("head_sha") == sha
        and run.get("head_branch") == tag
    ]


def latest_run(runs: list[dict[str, object]]) -> dict[str, object] | None:
    if not runs:
        return None
    return max(
        runs,
        key=lambda run: (
            str(run.get("created_at", "")),
            int(run.get("run_attempt", 1)),
            int(run.get("id", 0)),
        ),
    )


def verify_jobs(api_base: str, repository: str, token: str, run_id: int) -> list[str]:
    response = github_json(
        f"{api_base}/repos/{repository}/actions/runs/{run_id}/jobs?filter=latest&per_page=100",
        token,
    )
    jobs = response.get("jobs")
    if not isinstance(jobs, list):
        raise EvidenceError(f"CI run {run_id} returned no jobs list")

    names: list[str] = []
    failures: list[str] = []
    for job in jobs:
        if not isinstance(job, dict):
            continue
        name = str(job.get("name", ""))
        names.append(name)
        if job.get("status") != "completed" or job.get("conclusion") != "success":
            failures.append(
                f"{name}: status={job.get('status')} conclusion={job.get('conclusion')}"
            )

    if failures:
        raise EvidenceError("CI has incomplete or unsuccessful jobs: " + "; ".join(failures))

    missing: list[str] = []
    for prefix, expected_count in REQUIRED_JOB_COUNTS.items():
        count = sum(name == prefix if not prefix.endswith("(") else name.startswith(prefix) for name in names)
        if count != expected_count:
            missing.append(f"{prefix}: expected {expected_count}, found {count}")
    if missing:
        raise EvidenceError("CI required-job set is incomplete: " + "; ".join(missing))
    return sorted(names)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True, help="GitHub Actions output file")
    parser.add_argument("--timeout-minutes", type=int, default=180)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    sha = os.environ.get("RELEASE_SHA", "")
    tag = os.environ.get("RELEASE_TAG", "")
    token = os.environ.get("GH_TOKEN", "")
    api_base = os.environ.get("GITHUB_API_URL", "https://api.github.com").rstrip("/")
    if not repository or not sha or not tag or not token:
        print("same-commit CI gate needs repository, release SHA, tag, and GH_TOKEN", file=sys.stderr)
        return 1

    deadline = time.monotonic() + args.timeout_minutes * 60
    while time.monotonic() < deadline:
        try:
            run = latest_run(runs_for_commit(api_base, repository, token, sha, tag))
            if run is None:
                print(f"No CI push run exists yet for {tag} at {sha}; waiting.", flush=True)
            elif run.get("status") != "completed":
                print(
                    f"CI run {run.get('id')} is {run.get('status')}; waiting for completion.",
                    flush=True,
                )
            elif run.get("conclusion") != "success":
                raise EvidenceError(
                    f"CI run {run.get('id')} ended with {run.get('conclusion')!r}; publication refused"
                )
            else:
                run_id = int(run["id"])
                jobs = verify_jobs(api_base, repository, token, run_id)
                run_attempt = int(run.get("run_attempt", 1))
                html_url = str(run.get("html_url", ""))
                evidence = {
                    "workflow": "CI",
                    "run_id": run_id,
                    "run_attempt": run_attempt,
                    "head_sha": sha,
                    "head_branch": tag,
                    "event": "push",
                    "status": run.get("status"),
                    "conclusion": run.get("conclusion"),
                    "completed_at": run.get("updated_at"),
                    "html_url": html_url,
                    "successful_jobs": jobs,
                }
                args.output.parent.mkdir(parents=True, exist_ok=True)
                with args.output.open("a", encoding="utf-8") as output:
                    output.write(f"run_id={run_id}\n")
                    output.write(f"run_attempt={run_attempt}\n")
                    output.write(f"html_url={html_url}\n")
                    output.write(f"head_sha={sha}\n")
                    output.write(f"completed_at={run.get('updated_at', '')}\n")
                print(json.dumps(evidence, indent=2), flush=True)
                return 0
        except EvidenceError as error:
            print(f"Same-commit CI rejected: {error}", file=sys.stderr)
            return 1

        time.sleep(POLL_SECONDS)

    print(
        f"No successful full CI run for {tag} at {sha} completed within {args.timeout_minutes} minutes.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
