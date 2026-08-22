#!/usr/bin/env python3
"""Check Phase 9's recorded evidence and public-path safety invariants.

This checker is deliberately offline.  The evidence manifest says which
third-party operations were unavailable in this workspace; the synthetic
smoke runner supplies the executable proof for the behavior that can be
checked without credentials or a real Minecraft server.

Modes:

  phase9-check.py --evidence   check P9.14's evidence record
  phase9-check.py --gate       check P9.15's documentary gate prerequisites
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE_PATH = ROOT / "docs/msc2/networking/evidence/phase9-evidence.json"
SCOPE_PATH = ROOT / "docs/msc2/networking/phase9-scope.md"
MATRIX_PATH = ROOT / "docs/msc2/client-capability-matrix.csv"
AGENT_MAIN_PATH = ROOT / "crates/msc-agent/src/main.rs"
AGENT_CLI_PATH = ROOT / "crates/msc-agent/src/cli/mod.rs"
USER_ROUTES_PATH = ROOT / "crates/msc-agent/src/routes/users.rs"
NETWORKING_ROUTES_PATH = ROOT / "crates/msc-agent/src/routes/networking.rs"
EVIDENCE_DIR = ROOT / "docs/msc2/networking/evidence"
FIXTURE_DIRS = {
    "networking": ROOT / "fixtures/networking",
    "helper-lifecycle": ROOT / "fixtures/helper-lifecycle",
    "credentials": ROOT / "fixtures/credentials",
}
CI_WORKFLOW_PATH = ROOT / ".github/workflows/ci.yml"
SMOKE_PATH = ROOT / "tools/phase9/phase9-smoke.sh"

REQUIRED_INTEGRATIONS = {
    "playit",
    "duckdns",
    "resource-pack-hosting",
    "port-diagnostics",
    "geyser-floodgate",
    "xbox-broadcast",
    "notifications",
    "helper-lifecycle",
    "named-token-revocation",
}

PHASE9_ROUTES = {
    ("GET", "/v1/connectivity"),
    ("GET", "/v1/duckdns"),
    ("POST", "/v1/duckdns"),
    ("GET", "/v1/playit"),
    ("POST", "/v1/playit/start"),
    ("POST", "/v1/playit/stop"),
    ("GET", "/v1/resourcepacks"),
    ("POST", "/v1/resourcepacks/activate"),
    ("POST", "/v1/resourcepacks/remove"),
    ("POST", "/v1/resourcepacks/seturl"),
    ("POST", "/v1/resourcepacks/toggle"),
    ("GET", "/v1/config/geyser"),
    ("POST", "/v1/config/geyser"),
    ("GET", "/v1/broadcast/auth-prompt"),
    ("POST", "/v1/broadcast/auth-prompt/dismiss"),
    ("GET", "/v1/broadcast/autostart"),
    ("POST", "/v1/broadcast/autostart"),
    ("POST", "/v1/broadcast/credentials"),
    ("POST", "/v1/broadcast/download-jar"),
    ("GET", "/v1/broadcast/jar-status"),
    ("POST", "/v1/broadcast/restart"),
    ("POST", "/v1/broadcast/start"),
    ("GET", "/v1/broadcast/status"),
    ("POST", "/v1/broadcast/stop"),
    ("WS", "/v1/notifications/stream"),
    ("GET", "/v1/users"),
    ("POST", "/v1/users"),
    ("POST", "/v1/users/revoke"),
    ("POST", "/v1/users/update"),
}


class EvidenceError(Exception):
    """A human-readable evidence or invariant failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def read_json(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise EvidenceError(f"{path.relative_to(ROOT)}: cannot read JSON ({error})") from error
    require(isinstance(value, dict), f"{path.relative_to(ROOT)}: top level must be an object")
    return value


def check_evidence_manifest() -> list[str]:
    manifest = read_json(EVIDENCE_PATH)
    require(
        manifest.get("schema") == "msc2.phase9.evidence.v1",
        f"{EVIDENCE_PATH.relative_to(ROOT)}: unexpected schema",
    )
    records = manifest.get("integrations")
    require(isinstance(records, list), "evidence manifest: integrations must be a list")
    by_id: dict[str, dict] = {}
    for record in records:
        require(isinstance(record, dict), "evidence manifest: each integration must be an object")
        integration = record.get("integration")
        require(isinstance(integration, str) and integration, "evidence manifest: missing integration id")
        require(integration not in by_id, f"evidence manifest: duplicate {integration}")
        by_id[integration] = record
        for field in ("synthetic", "live", "summary", "verification"):
            require(record.get(field), f"evidence manifest: {integration} missing {field}")
        for outcome_name in ("synthetic", "live"):
            outcome = record[outcome_name]
            require(
                outcome in {"success", "unavailable"},
                f"evidence manifest: {integration}.{outcome_name} must be success or unavailable",
            )
            if outcome == "unavailable":
                reason = record.get(f"{outcome_name}Reason")
                require(
                    isinstance(reason, str) and reason.strip(),
                    f"evidence manifest: {integration} needs {outcome_name}Reason",
                )
        evidence = record.get("evidence")
        require(isinstance(evidence, list) and evidence, f"evidence manifest: {integration} has no evidence paths")
        for relative in evidence:
            require(isinstance(relative, str), f"evidence manifest: {integration} has a non-string path")
            path = (ROOT / relative).resolve()
            require(path.is_relative_to(ROOT), f"evidence manifest: path escapes repository: {relative}")
            require(path.is_file(), f"evidence manifest: missing evidence file {relative}")

    missing = REQUIRED_INTEGRATIONS - by_id.keys()
    extra = by_id.keys() - REQUIRED_INTEGRATIONS
    require(not missing, f"evidence manifest: missing integrations {sorted(missing)}")
    require(not extra, f"evidence manifest: unexpected integrations {sorted(extra)}")
    return [f"evidence: {len(records)} integrations recorded"]


def check_capability_matrix() -> list[str]:
    with MATRIX_PATH.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    found = {(row["method"], row["path"]): row for row in rows}
    missing = PHASE9_ROUTES - found.keys()
    require(not missing, f"capability matrix: missing Phase 9 routes {sorted(missing)}")
    not_implemented = [
        operation
        for operation in sorted(PHASE9_ROUTES)
        if found[operation]["agent_status"] != "Implemented"
    ]
    require(
        not not_implemented,
        f"capability matrix: Phase 9 agent routes not marked Implemented {not_implemented}",
    )
    return [f"capabilities: {len(PHASE9_ROUTES)} Phase 9 routes marked Implemented"]


def check_management_boundary() -> list[str]:
    main = AGENT_MAIN_PATH.read_text(encoding="utf-8")
    cli = AGENT_CLI_PATH.read_text(encoding="utf-8")
    users = USER_ROUTES_PATH.read_text(encoding="utf-8")
    networking = NETWORKING_ROUTES_PATH.read_text(encoding="utf-8")

    require(
        'default_value = "127.0.0.1:48400"' in cli,
        "management boundary: the default management bind is not loopback",
    )
    require(
        'route("/health", get(routes::health::health))' in main,
        "management boundary: the public router no longer exposes only health",
    )
    require(
        "auth::require_bearer_token" in main and '.route_layer(' in main,
        "management boundary: protected routes are not behind bearer auth",
    )
    require(
        "Only an admin credential may manage named users." in users,
        "management boundary: named-token administration lost its admin check",
    )
    require(
        "approved" in networking and "resource pack" in networking.lower(),
        "management boundary: resource-pack route no longer documents approved-file serving",
    )
    require(
        "secret" in networking.lower() and "has_secret_key" in networking,
        "management boundary: Playit status no longer uses presence-only secret reporting",
    )
    return ["boundary: loopback management bind, bearer gate, and player-path guards present"]


def check_scope_references() -> list[str]:
    scope = SCOPE_PATH.read_text(encoding="utf-8")
    require("P9.14" in scope, "scope: P9.14 evidence note is missing")
    require("live evidence" in scope and "unavailable" in scope, "scope: honest live-evidence policy is missing")
    require((EVIDENCE_DIR / "README.md").is_file(), "evidence: README.md is missing")
    require((EVIDENCE_DIR / "mcsrvstat-us.md").is_file(), "evidence: mcsrvstat-us.md is missing")
    return ["provenance: scope and live-evidence notes are present"]


def check_fixture_coverage() -> list[str]:
    """Check that the gate still has the fixture families P9.2 promised.

    The acquisition amendment added cases after P9.2, so this intentionally
    checks minimum coverage rather than freezing the directory to an old
    count.  The evidence manifest and the targeted Rust suites provide the
    per-integration linkage and executable behavior checks.
    """
    minimums = {"networking": 14, "helper-lifecycle": 8, "credentials": 8}
    counts: dict[str, int] = {}
    for name, directory in FIXTURE_DIRS.items():
        require(directory.is_dir(), f"fixtures: missing {directory.relative_to(ROOT)}")
        files = sorted(directory.glob("*.json"))
        require(
            len(files) >= minimums[name],
            f"fixtures: {directory.relative_to(ROOT)} has {len(files)} files, expected at least {minimums[name]}",
        )
        for path in files:
            try:
                data = json.loads(path.read_text(encoding="utf-8"))
            except (OSError, json.JSONDecodeError) as error:
                raise EvidenceError(f"fixtures: {path.relative_to(ROOT)} is not valid JSON ({error})") from error
            require(isinstance(data, dict), f"fixtures: {path.relative_to(ROOT)} must contain an object")
        counts[name] = len(files)
    return [
        "fixtures: "
        + ", ".join(f"{name}={count}" for name, count in counts.items())
        + " (minimum Phase 9 coverage present)"
    ]


def check_gate_runner() -> list[str]:
    """Check that the committed smoke and CI paths exercise the gate scope."""
    smoke = SMOKE_PATH.read_text(encoding="utf-8")
    require(
        "phase9_routes" in smoke and "cli_phase9" in smoke,
        "gate runner: public HTTP and CLI Phase 9 suites are not in the synthetic smoke",
    )
    for required_suite in (
        "helper_process",
        "helper_acquisition",
        "network_diagnostics",
        "xbox_broadcast",
        "credential_repository",
    ):
        require(
            required_suite in smoke,
            f"gate runner: synthetic smoke omits {required_suite}",
        )

    workflow = CI_WORKFLOW_PATH.read_text(encoding="utf-8")
    for platform in ("ubuntu-latest", "macos-latest", "windows-latest"):
        require(platform in workflow, f"CI: missing {platform} toolchain leg")
    require("cargo nextest run --workspace" in workflow, "CI: workspace regression suite is missing")
    require(
        "headless-link-check.py --all-artifacts" in workflow,
        "CI: headless no-GUI link check is missing",
    )
    return ["gate runner: Phase 9 synthetic coverage and tri-platform/headless CI checks are wired"]


def check_gate_scope() -> list[str]:
    """Check the owner-approved boundary and explicit deferrals behind the gate."""
    scope = SCOPE_PATH.read_text(encoding="utf-8")
    require("duckdns_label_only" in scope, "scope: DuckDNS label-only behavior is not recorded")
    require("no DuckDNS updater is implied" in scope, "scope: DuckDNS updater deferral is missing")
    require("Approved by Cameron Temple" in scope, "scope: D-012 approval is missing")
    require("deferred to Phase 11" in scope, "scope: Phase 11 access deferrals are missing")
    return ["scope: player/management boundary and owner-approved access deferrals are recorded"]


def check_exit_gate() -> list[str]:
    messages: list[str] = []
    messages.extend(check_evidence_manifest())
    messages.extend(check_capability_matrix())
    messages.extend(check_management_boundary())
    messages.extend(check_scope_references())
    messages.extend(check_fixture_coverage())
    messages.extend(check_gate_runner())
    messages.extend(check_gate_scope())
    return messages


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", action="store_true", help="check the recorded Phase 9 evidence")
    parser.add_argument("--gate", action="store_true", help="check P9.15's committed gate prerequisites")
    args = parser.parse_args()
    if args.evidence and args.gate:
        parser.error("choose only one of --evidence or --gate")
    if not args.evidence and not args.gate:
        parser.error("choose --evidence or --gate")

    try:
        messages = check_exit_gate() if args.gate else []
        if args.evidence:
            messages.extend(check_evidence_manifest())
            messages.extend(check_capability_matrix())
            messages.extend(check_management_boundary())
            messages.extend(check_scope_references())
    except EvidenceError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    for message in messages:
        print(message)
    print("PHASE 9 GATE CHECK PASSED" if args.gate else "PHASE 9 EVIDENCE CHECK PASSED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
