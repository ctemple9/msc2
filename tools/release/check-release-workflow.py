#!/usr/bin/env python3
"""Check the artifact-only cross-platform beta workflow contract."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class WorkflowError(Exception):
    """A human-readable release workflow failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise WorkflowError(message)


def read_workflow(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise WorkflowError(f"cannot read workflow: {error}") from error


def check_yaml(path: Path) -> None:
    """Use an installed parser when available, with a Ruby fallback on CI."""
    try:
        import yaml  # type: ignore[import-not-found]
    except ImportError:
        result = subprocess.run(
            [
                "ruby",
                "-e",
                "require 'yaml'; YAML.load_file(ARGV.fetch(0))",
                str(path),
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        require(result.returncode == 0, f"workflow is not valid YAML: {result.stderr.strip()}")
    else:
        try:
            yaml.safe_load(path.read_text(encoding="utf-8"))
        except yaml.YAMLError as error:
            raise WorkflowError(f"workflow is not valid YAML: {error}") from error


def require_fragment(workflow: str, fragment: str) -> None:
    require(fragment in workflow, f"workflow is missing {fragment!r}")


def check_candidate_workflow(workflow: str) -> None:
    for fragment in (
        "workflow_dispatch:",
        "push:",
        "tags:",
        "- 'v*'",
        "jobs:",
        "runs-on: ${{ matrix.os }}",
        "fail-fast: false",
        "macos-15-intel",
        "windows-latest",
        "ubuntu-latest",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "npm run check",
        "npm run test:contract",
        "npm run test:auth-desktop",
        "cargo fmt --all -- --check",
        "cargo nextest run -p msc-agent --test web_ui",
        "cargo build --release",
        "--bundles \"${{ matrix.tauri-bundle }}\" --no-sign",
        "tauri-bundle: dmg",
        "tauri-bundle: msi",
        "tauri-bundle: deb",
        "build-linux-headless.sh",
        "build-macos-headless.sh",
        "build-windows-headless.ps1",
        "prepare-windows-icon.py",
        "actions/upload-artifact@v4",
        "beta-${{ matrix.platform }}-${{ env.RELEASE_VERSION }}",
        "UNSIGNED-BETA-NOTICE.txt",
        "notarization",
    ):
        require_fragment(workflow, fragment)

    require(
        re.search(r"cargo clippy -p msc-agent --bin msc --target", workflow) is not None,
        "workflow is missing the targeted msc-agent binary clippy check",
    )
    require(
        "-D warnings -A dead-code -A unused-mut" in workflow,
        "workflow does not isolate the deferred TUI clippy diagnostics",
    )

    require(
        re.search(r"platform:\s+macos-x86_64", workflow) is not None,
        "macOS artifact must be labelled x86_64",
    )
    require(
        re.search(r"platform:\s+windows-x86_64", workflow) is not None,
        "Windows artifact must be labelled x86_64",
    )
    require(
        re.search(r"platform:\s+linux-x86_64", workflow) is not None,
        "Linux artifact must be labelled x86_64",
    )
    require("clients/ios" not in workflow, "release workflow must not build iOS outputs")
    require("src/cli/tui" not in workflow, "release workflow must not build TUI outputs")
    require("gh release" not in workflow, "candidate workflow must not publish a GitHub release")
    require("softprops/action-gh-release" not in workflow, "candidate workflow must not publish a GitHub release")
    require("notarytool" not in workflow, "candidate workflow must not invoke notarization")
    require("signtool" not in workflow, "candidate workflow must not invoke Authenticode signing")
    require("APPLE_CERTIFICATE" not in workflow, "candidate workflow must not load a signing certificate")


def check_publish_guard(workflow: str) -> None:
    require(re.search(r"(?m)^\s+publish:", workflow) is not None, "publish job is missing")
    require("github.event_name" in workflow, "publish job is not event-guarded")
    require("refs/tags/v" in workflow, "publish job is not tag-guarded")
    require("verify-artifact-manifest.py" in workflow, "manifest verifier is not wired")
    require("sha256" in workflow.lower(), "SHA-256 manifest generation is not wired")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("workflow", type=Path)
    parser.add_argument(
        "--expect-publish-guard",
        action="store_true",
        help="also require the guarded publication job used by the next release step",
    )
    args = parser.parse_args()
    path = args.workflow if args.workflow.is_absolute() else ROOT / args.workflow
    try:
        workflow = read_workflow(path)
        check_yaml(path)
        check_candidate_workflow(workflow)
        if args.expect_publish_guard:
            check_publish_guard(workflow)
    except WorkflowError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    message = "cross-platform artifact candidate is valid"
    if args.expect_publish_guard:
        message += " with guarded publication"
    print(f"OK: {message}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
