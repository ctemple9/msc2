#!/usr/bin/env python3
"""Reject incomplete Phase 16 exact-artifact acceptance evidence."""

from __future__ import annotations

import argparse
from datetime import datetime
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PENDING = {"", "—", "-", "PENDING", "NOT RUN", "DERIVED FROM TAG"}
SHA256 = re.compile(r"[0-9a-f]{64}")
SOURCE_SHA = re.compile(r"[0-9a-f]{40,64}")
UTC_DATE = re.compile(r"20\d{2}-\d{2}-\d{2}")
UTC_TIMESTAMP = re.compile(r"20\d{2}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z")
TAG = re.compile(r"v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?")

ARTIFACTS = {
    "desktop-macos-x86_64": ("Desktop", "macOS Intel", "msc2-{version}-macos-x86_64.dmg"),
    "desktop-macos-aarch64": ("Desktop", "macOS Apple Silicon", "msc2-{version}-macos-aarch64.dmg"),
    "desktop-windows-x86_64": ("Desktop", "Windows x86_64", "msc2-{version}-windows-x86_64.msi"),
    "desktop-linux-deb-x86_64": ("Desktop .deb", "Linux x86_64", "msc2-{version}-linux-x86_64.deb"),
    "desktop-linux-rpm-x86_64": ("Desktop .rpm", "Linux x86_64", "msc2-{version}-linux-x86_64.rpm"),
    "headless-macos-x86_64": ("Headless", "macOS Intel", "msc2-headless-{version}-macos-x86_64.tar.gz"),
    "headless-macos-aarch64": ("Headless", "macOS Apple Silicon", "msc2-headless-{version}-macos-aarch64.tar.gz"),
    "headless-windows-x86_64": ("Headless", "Windows x86_64", "msc2-headless-{version}-windows-x86_64.zip"),
    "headless-linux-x86_64": ("Headless", "Linux x86_64", "msc2-headless-{version}-linux-x86_64.tar.gz"),
}

GATE_IDS = {
    "archive-confinement",
    "operation-exclusivity",
    "host-reset-exclusivity",
    "world-replacement-recovery",
    "backup-save-acknowledgement",
    "cancellation-and-revocation",
    "bounded-operation-history",
    "host-connection-generation",
    "windows-service-lifecycle",
    "windows-update-rollback",
    "macos-update-rollback",
    "browser-retirement",
    "supported-client-pairing",
    "minecraft-lifecycle",
    "world-import-backup-restore",
    "interruption-recovery",
    "service-reboot-signout",
    "permission-revocation",
    "uninstall-data-retention",
    "artifact-identity",
    "same-commit-ci",
    "release-provenance",
    "linux-minimum",
    "generated-api-and-frontend",
    "public-documents",
}


class EvidenceError(Exception):
    """A precise reason the release gate is not complete."""


def table_after(document: str, heading: str) -> tuple[list[str], list[dict[str, str]]]:
    marker = f"## {heading}"
    start = document.find(marker)
    if start < 0:
        raise EvidenceError(f"missing section: {heading}")
    end = document.find("\n## ", start + len(marker))
    section = document[start : end if end >= 0 else len(document)]
    lines = [line.strip() for line in section.splitlines() if line.strip().startswith("|")]
    if len(lines) < 3:
        raise EvidenceError(f"section {heading!r} has no complete Markdown table")

    def cells(line: str) -> list[str]:
        return [cell.strip() for cell in line.strip().strip("|").split("|")]

    headers = cells(lines[0])
    rows: list[dict[str, str]] = []
    for line in lines[1:]:
        values = cells(line)
        if values and all(re.fullmatch(r":?-{3,}:?", value or "") for value in values):
            continue
        if len(values) != len(headers):
            raise EvidenceError(f"malformed table row in section {heading!r}: {line}")
        rows.append(dict(zip(headers, values, strict=True)))
    return headers, rows


def is_pending(value: str) -> bool:
    return value.strip().upper() in PENDING


def parse_date(value: str, context: str, errors: list[str]) -> None:
    if not UTC_DATE.fullmatch(value):
        errors.append(f"{context}: observed date must be YYYY-MM-DD UTC")
        return
    try:
        datetime.strptime(value, "%Y-%m-%d")
    except ValueError:
        errors.append(f"{context}: observed date is not a real calendar date")


def check(path: Path) -> list[str]:
    try:
        document = path.read_text(encoding="utf-8")
    except OSError as error:
        raise EvidenceError(f"cannot read acceptance record: {error}") from error

    errors: list[str] = []
    _, identity_rows = table_after(document, "Candidate release identity")
    identity = {row.get("Field", ""): row.get("Value", "") for row in identity_rows}
    required_identity = (
        "Candidate tag",
        "Published release URL",
        "Source commit (full SHA)",
        "Published at (UTC)",
        "Successful full CI run for this tag commit (run ID and URL)",
        "Published SHA256SUMS verification",
        "Signed update manifest and provenance verification",
    )
    for field in required_identity:
        value = identity.get(field, "")
        if is_pending(value):
            errors.append(f"release identity is missing {field}")

    tag = identity.get("Candidate tag", "")
    match = TAG.fullmatch(tag)
    version = tag[1:] if match else ""
    if not match and not is_pending(tag):
        errors.append("candidate tag must be a version tag such as v0.2.0")

    source_commit = identity.get("Source commit (full SHA)", "")
    if not is_pending(source_commit) and not SOURCE_SHA.fullmatch(source_commit):
        errors.append("candidate source commit must be a full 40- or 64-character SHA")
    release_url = identity.get("Published release URL", "")
    if not is_pending(release_url) and (not release_url.startswith("https://") or tag not in release_url):
        errors.append("published release URL must be HTTPS and identify the candidate tag")
    published_at = identity.get("Published at (UTC)", "")
    if not is_pending(published_at) and not UTC_TIMESTAMP.fullmatch(published_at):
        errors.append("published time must use YYYY-MM-DDTHH:MM:SSZ")
    ci_run = identity.get("Successful full CI run for this tag commit (run ID and URL)", "")
    if not is_pending(ci_run) and (not re.search(r"\b\d{6,}\b", ci_run) or "https://" not in ci_run):
        errors.append("same-commit CI evidence must include a run ID and HTTPS URL")
    if identity.get("Published SHA256SUMS verification") not in {"PASS", "PENDING"}:
        errors.append("published SHA256SUMS verification must be exactly PASS")
    if identity.get("Signed update manifest and provenance verification") not in {"PASS", "PENDING"}:
        errors.append("signed manifest and provenance verification must be exactly PASS")

    _, artifact_rows = table_after(document, "Exact published artifacts")
    by_id: dict[str, dict[str, str]] = {}
    for row in artifact_rows:
        artifact_id = row.get("ID", "")
        if artifact_id in by_id:
            errors.append(f"duplicate artifact row: {artifact_id}")
        by_id[artifact_id] = row
    missing_artifacts = set(ARTIFACTS) - set(by_id)
    extra_artifacts = set(by_id) - set(ARTIFACTS)
    errors.extend(f"missing required artifact row: {item}" for item in sorted(missing_artifacts))
    errors.extend(f"unexpected artifact row: {item}" for item in sorted(extra_artifacts))

    exact_names: set[str] = set()
    for artifact_id, (installation, platform, pattern) in ARTIFACTS.items():
        row = by_id.get(artifact_id)
        if row is None:
            continue
        expected_name = pattern.format(version=version) if version else ""
        if row.get("Installation") != installation or row.get("Platform") != platform:
            errors.append(f"{artifact_id}: installation/platform does not match the release matrix")
        if not expected_name or row.get("Expected filename") != expected_name:
            errors.append(f"{artifact_id}: expected filename does not match candidate tag {tag!r}")
        published_name = row.get("Published filename", "")
        if is_pending(published_name):
            errors.append(f"{artifact_id}: exact published filename is missing")
        elif published_name != expected_name:
            errors.append(f"{artifact_id}: published filename {published_name!r} is not {expected_name!r}")
        else:
            exact_names.add(published_name)
        try:
            if int(row.get("Bytes", "")) <= 0:
                raise ValueError
        except ValueError:
            errors.append(f"{artifact_id}: positive exact byte size is required")
        digest = row.get("SHA-256", "")
        if not SHA256.fullmatch(digest):
            errors.append(f"{artifact_id}: exact lowercase SHA-256 is required")
        if row.get("SHA256SUMS") != "PASS":
            errors.append(f"{artifact_id}: Cameron must verify this asset against SHA256SUMS")
        if row.get("Install and launch") != "PASS":
            errors.append(f"{artifact_id}: Cameron must record a successful install and launch")
        if row.get("Observed by") != "Cameron Temple":
            errors.append(f"{artifact_id}: result must be observed by Cameron Temple")
        parse_date(row.get("Observed (UTC)", ""), artifact_id, errors)
        if is_pending(row.get("Evidence", "")):
            errors.append(f"{artifact_id}: install evidence link or path is missing")

    _, gate_rows = table_after(document, "Required release checks")
    by_gate: dict[str, dict[str, str]] = {}
    for row in gate_rows:
        gate_id = row.get("ID", "")
        if gate_id in by_gate:
            errors.append(f"duplicate gate row: {gate_id}")
        by_gate[gate_id] = row
    errors.extend(f"missing required gate row: {item}" for item in sorted(GATE_IDS - set(by_gate)))
    errors.extend(f"unexpected gate row: {item}" for item in sorted(set(by_gate) - GATE_IDS))

    for gate_id, row in by_gate.items():
        scope = row.get("Exact artifact(s) or source commit", "")
        if is_pending(scope):
            errors.append(f"{gate_id}: exact release artifact or source commit is missing")
        elif not any(name in scope for name in exact_names) and not (
            SOURCE_SHA.fullmatch(source_commit) and source_commit in scope
        ):
            errors.append(f"{gate_id}: evidence is not tied to a published artifact or candidate commit")
        if row.get("Result") != "PASS":
            errors.append(f"{gate_id}: result is not PASS")
        if row.get("Observed by") != "Cameron Temple":
            errors.append(f"{gate_id}: result must be observed by Cameron Temple")
        parse_date(row.get("Observed (UTC)", ""), gate_id, errors)
        if is_pending(row.get("Evidence", "")):
            errors.append(f"{gate_id}: evidence link or path is missing")

    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("acceptance_record", type=Path)
    args = parser.parse_args()
    path = args.acceptance_record if args.acceptance_record.is_absolute() else ROOT / args.acceptance_record
    try:
        errors = check(path)
    except EvidenceError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    if errors:
        print(f"FAIL: Phase 16 exact-artifact evidence is incomplete ({len(errors)} issue(s)):", file=sys.stderr)
        for error in errors[:24]:
            print(f"- {error}", file=sys.stderr)
        if len(errors) > 24:
            print(f"- ... and {len(errors) - 24} more issue(s)", file=sys.stderr)
        return 1
    print("OK: Phase 16 exact-artifact evidence is complete and Cameron-observed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
