#!/usr/bin/env python3
"""Check the static evidence and source contract for coordinated updates."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class GateError(Exception):
    """A human-readable static release-gate failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise GateError(message)


def read(relative: str) -> str:
    path = ROOT / relative
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise GateError(f"cannot read {relative}: {error}") from error


def require_fragments(relative: str, fragments: tuple[str, ...]) -> None:
    contents = read(relative)
    missing = [fragment for fragment in fragments if fragment not in contents]
    require(not missing, f"{relative} is missing: {', '.join(missing)}")


def check_evidence_packet() -> None:
    evidence = read("docs/msc2/clients/phase12-release-evidence/update-gate.md")
    required_rows = (
        "U01 | Signed publication",
        "U02 | Version and release notes",
        "U03 | Decline and cancel",
        "U04 | Signature and digest refusal",
        "U05 | Interrupted and already-staged work",
        "U06 | macOS and Windows coordinated replacement",
        "U07 | Linux desktop package authorization",
        "U08 | Standalone headless update",
        "U09 | Distribution package-manager guidance",
        "U10 | Preserved user data",
        "U11 | Health recovery and rollback",
        "U12 | CLI text and JSON",
        "U13 | Unsigned prerelease refusal",
        "U14 | Remote service boundary",
        "U15 | Physical platform handoff",
    )
    missing = [row for row in required_rows if row not in evidence]
    require(not missing, "update evidence packet is missing: " + ", ".join(missing))
    require(
        "physical platform runs remain pending" in evidence,
        "update evidence packet must keep physical platform runs pending",
    )


def check_schema() -> None:
    try:
        schema = json.loads(read("packaging/update-release-schema.json"))
    except json.JSONDecodeError as error:
        raise GateError(f"update-release-schema.json is invalid JSON: {error}") from error

    require(schema.get("schemaVersion") == 1, "update schema must be version 1")
    require(schema.get("releaseSet") == "msc-application", "update schema has the wrong release set")
    signature = schema.get("manifestSignature", {})
    require(signature.get("algorithm") == "Ed25519", "update schema must require Ed25519")
    require(
        signature.get("privateKeySource") == "GitHub Actions secret only",
        "update schema must keep the private key in GitHub Actions",
    )
    installation = schema.get("installation", {})
    require(
        installation.get("confirmation") == "explicit and repeated for the staged releaseId",
        "update schema must require repeated explicit confirmation",
    )
    require(installation.get("rollback"), "update schema must describe rollback")
    require(installation.get("remoteServiceControl") == "forbidden", "remote service control must be forbidden")
    require(
        set(schema.get("platforms", {}))
        == {
            "macos-desktop-x86_64",
            "macos-headless-x86_64",
            "windows-desktop-x86_64",
            "windows-headless-x86_64",
            "linux-desktop-deb-x86_64",
            "linux-desktop-rpm-x86_64",
            "linux-headless-x86_64",
        },
        "update schema must enumerate the seven x86_64 update shapes",
    )


def check_source_contract() -> None:
    require_fragments(
        "crates/msc-infrastructure/src/release_update.rs",
        (
            "Update manifest signature did not verify.",
            "did not match its signed size or SHA-256 digest",
            "This release is already staged; it will not be overwritten.",
            "Update manifest is not in canonical form.",
            "TOTAL_DOWNLOAD_MAX_BYTES",
            "fs::remove_dir_all(&temporary_directory)",
        ),
    )
    require_fragments(
        "clients/desktop-web/src/lib/updates/coordinated.ts",
        (
            "explicitlyApproved",
            "confirmation.releaseId === release.releaseId",
            "release.platform === 'macos'",
            "release.platform === 'windows'",
        ),
    )
    require_fragments(
        "clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte",
        (
            "Release notes",
            "Cancel",
            "Nothing installs automatically.",
            "can be rolled back.",
            "not update the remote host's service.",
        ),
    )
    require_fragments(
        "crates/msc-agent/src/cli/update.rs",
        (
            "UpdateCommand::Check",
            "UpdateCommand::Install",
            'state: "declined"',
            'state: "package-manager-guidance"',
            "release_notes",
            "rollback_payload",
            "--yes",
            "update commands are local-only",
        ),
    )


def check_workflow() -> None:
    workflow = read(".github/workflows/release.yml")
    require(
        "python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard"
        in workflow,
        "release workflow must run the guarded artifact publication check",
    )
    require(
        "python3 tools/release/check-update-gate.py" in workflow,
        "release workflow must run the update static gate",
    )
    require(
        re.search(r"test \"\$asset_count\" -eq 7", workflow) is not None,
        "release workflow must require the complete seven-asset set",
    )


def main() -> int:
    try:
        check_evidence_packet()
        check_schema()
        check_source_contract()
        check_workflow()
    except GateError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    print("OK: coordinated update evidence and static gate are complete")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
