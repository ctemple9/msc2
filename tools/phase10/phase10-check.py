#!/usr/bin/env python3
"""Check the committed evidence for the Phase 10 Bedrock gate.

This is the documentary half of P10.28's gate.  The synthetic public-path
smoke and the workspace regression suite remain separate commands in the
step's Verify line so their test output is visible to the person verifying
the step.  This checker makes the committed corpus, contracts, boundaries,
evidence, and exact CI candidate fail closed before those executable checks
are run.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCOPE_PATH = ROOT / "docs/msc2/bedrock/phase10-scope.md"
CI_EVIDENCE_PATH = ROOT / "docs/msc2/bedrock/evidence/phase10-ci.md"
SMOKE_PATH = ROOT / "tools/phase10/phase10-smoke.sh"
WORKFLOW_PATH = ROOT / ".github/workflows/ci.yml"
FIXTURE_RUNNER = ROOT / "tools/fixture-runner/run.py"

FIXTURES = {
    "bedrock-properties": 24,
    "bedrock-players": 22,
    "bedrock-console": 16,
    "bedrock-logging": 8,
    "bedrock-leveldb": 22,
    "bedrock-nbt": 32,
    "bedrock-world-layout": 10,
    "bedrock-backup": 10,
    "bedrock-provisioning": 16,
    "bedrock-runtime": 14,
    "bedrock-sidecar": 16,
    "bedrock-udp": 5,
}


class GateError(Exception):
    """A human-readable gate failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise GateError(message)


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise GateError(f"{path.relative_to(ROOT)}: cannot read ({error})") from error


def run_check(label: str, command: list[str]) -> str:
    result = subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        detail = (result.stderr or result.stdout).strip().replace("\n", "; ")
        raise GateError(f"{label} failed: {detail}")
    output = (result.stdout or result.stderr).strip().splitlines()
    return output[-1] if output else "passed"


def check_fixture_corpus() -> list[str]:
    messages = []
    for name, expected in FIXTURES.items():
        relative = Path("fixtures") / name
        messages.append(
            run_check(
                f"fixture corpus {name}",
                [
                    sys.executable,
                    str(FIXTURE_RUNNER),
                    "--validate-dir",
                    str(relative),
                    "--expect",
                    str(expected),
                ],
            )
        )
    return messages


def check_source_boundary() -> str:
    required_files = (
        "crates/msc-application/src/bedrock_runtime.rs",
        "crates/msc-application/src/bedrock_linux.rs",
        "crates/msc-application/src/bedrock_windows.rs",
        "crates/msc-application/src/bedrock_macos.rs",
        "crates/msc-infrastructure/src/bedrock_native.rs",
        "crates/msc-infrastructure/src/bedrock_sidecar.rs",
        "sidecar/bedrock/BedrockSidecarCore.swift",
        "sidecar/bedrock/BedrockSidecar.xcodeproj/project.pbxproj",
    )
    for relative in required_files:
        require((ROOT / relative).is_file(), f"runtime boundary: missing {relative}")

    runtime = read_text(ROOT / required_files[0])
    linux = read_text(ROOT / required_files[1])
    windows = read_text(ROOT / required_files[2])
    macos = read_text(ROOT / required_files[3])
    native = read_text(ROOT / required_files[4])
    sidecar = read_text(ROOT / required_files[5])
    swift = read_text(ROOT / required_files[6])
    project = read_text(ROOT / required_files[7])

    require("pub trait BedrockRuntime" in runtime, "runtime boundary: shared BedrockRuntime trait is missing")
    require("bedrock_server" in linux, "runtime boundary: Linux native BDS process is missing")
    require("windows_bedrock_spawn_request" in windows, "runtime boundary: Windows native BDS dispatch is missing")
    require("WINDOWS_BEDROCK_EXECUTABLE_NAME: &str = \"bedrock_server.exe\"" in native, "runtime boundary: Windows BDS executable name is missing")
    require("BedrockSidecarProcess" in macos, "runtime boundary: macOS sidecar client is missing")
    require("NativeBedrockHost" in native, "runtime boundary: native host dispatch is missing")
    require("JSON" in sidecar or "Json" in sidecar, "runtime boundary: sidecar JSON-lines transport is missing")
    require("VZVirtualMachine" in swift, "runtime boundary: Swift sidecar does not own Virtualization.framework")
    require("BedrockSidecarTests.swift" in project, "runtime boundary: Swift sidecar tests are not in the project")
    return "ok: native Linux, native Windows, and macOS Swift-sidecar boundaries are present"


def check_scope() -> str:
    scope = re.sub(r"\s+", " ", read_text(SCOPE_PATH))
    required = (
        "native Linux",
        "native Windows",
        "LevelDB",
        "allowlist",
        "permissions",
        "metrics",
        "UDP",
        "Apple Silicon",
        "D-007",
        "D-022",
        "D-028",
        "BedrockSkinFetcher.swift",
        "Phase 11",
    )
    missing = [term for term in required if term.lower() not in scope.lower()]
    require(not missing, f"scope: missing Phase 10 boundary terms {missing}")
    require(
        "## P10.28 exact gate record" in scope,
        "scope: P10.28 exact gate record is missing",
    )
    return "ok: Phase 10 scope records the approved boundaries and explicit deferrals"


def check_public_gate_wiring() -> str:
    smoke = read_text(SMOKE_PATH)
    require(
        "--synthetic" in smoke and "bedrock_routes" in smoke and "bedrock_cli" in smoke,
        "smoke: synthetic Bedrock HTTP and CLI tests are not wired",
    )
    workflow = read_text(WORKFLOW_PATH)
    for marker in (
        "phase10-smoke.sh --synthetic",
        "phase10/compatibility-check.py",
        "phase10/evidence-check.py --distribution",
        "phase10/evidence-check.py --runtimes",
        "cargo nextest run --workspace",
        "headless-link-check.py --all-artifacts",
    ):
        require(marker in workflow, f"CI: missing {marker}")
    return "ok: synthetic smoke, documentary checks, workspace tests, and headless link proof are wired"


def git_output(*args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        detail = (result.stderr or result.stdout).strip()
        raise GateError(f"git {' '.join(args)} failed: {detail}")
    return result.stdout.strip()


def check_exact_ci_candidate() -> str:
    evidence = read_text(CI_EVIDENCE_PATH)
    candidates = set(re.findall(r"(?<![0-9a-f])[0-9a-f]{40}(?![0-9a-f])", evidence))
    require(len(candidates) == 1, "CI evidence: expected exactly one recorded 40-character candidate commit")
    candidate = next(iter(candidates))
    git_output("cat-file", "-e", f"{candidate}^{{commit}}")
    current = git_output("rev-parse", "HEAD")
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", candidate, current],
        cwd=ROOT,
        check=False,
    )
    require(ancestor.returncode == 0, "CI evidence: recorded candidate is not an ancestor of the checked-out code")

    runs = set(re.findall(r"actions/runs/(\d+)", evidence))
    require(len(runs) == 1, "CI evidence: expected exactly one recorded GitHub Actions run")
    require("The later P10.27 documentation commit is not the tested candidate." in evidence, "CI evidence: candidate/documentation distinction is missing")
    require(evidence.count("| success |") >= 4, "CI evidence: all macOS, Linux, Windows, and headless results are not recorded as success")
    lower = re.sub(r"\s+", " ", evidence.lower())
    for platform in ("macos", "linux", "windows", "headless"):
        require(platform in lower, f"CI evidence: missing {platform} result")
    require("did not download bds" in lower, "CI evidence: live-download limit is missing")
    require("start a live bedrock server or vm" in lower, "CI evidence: live-runtime limit is missing")
    return f"ok: exact candidate {candidate} has green CI run {next(iter(runs))}"


def check_gate() -> list[str]:
    messages = []
    messages.extend(check_fixture_corpus())
    messages.append(
        run_check(
            "API contract",
            [sys.executable, "tools/api-contract-check.py", "--v1-summary"],
        )
    )
    messages.append(
        run_check(
            "iOS Bedrock contract",
            [sys.executable, "tools/phase10/ios-contract-check.py"],
        )
    )
    messages.append(
        run_check(
            "Bedrock compatibility matrix",
            [
                sys.executable,
                "tools/phase10/compatibility-check.py",
                "docs/msc2/bedrock/compatibility-matrix.csv",
                "--require-cell",
                "macOS (Apple Silicon)=unavailable",
            ],
        )
    )
    messages.append(
        run_check(
            "Bedrock distribution evidence",
            [sys.executable, "tools/phase10/evidence-check.py", "--distribution"],
        )
    )
    messages.append(
        run_check(
            "Bedrock runtime evidence",
            [sys.executable, "tools/phase10/evidence-check.py", "--runtimes"],
        )
    )
    messages.extend(
        [
            check_source_boundary(),
            check_scope(),
            check_public_gate_wiring(),
            check_exact_ci_candidate(),
        ]
    )
    return messages


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", action="store_true", help="check the committed Phase 10 gate evidence")
    args = parser.parse_args()
    if not args.gate:
        parser.error("choose --gate")

    try:
        for message in check_gate():
            print(message)
    except (GateError, OSError):
        error = sys.exc_info()[1]
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print("PHASE 10 GATE CHECK PASSED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
