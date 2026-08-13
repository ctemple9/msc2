#!/usr/bin/env python3
"""Validate Phase 4 real-service credential persistence evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE_DIR = ROOT / "docs" / "msc2" / "lifecycle" / "credential-evidence"
SCHEMA = "msc2.phase4.credential-evidence.v1"

REQUIRED_TRUE_FIELDS = [
    "credentialStoredInProductionStore",
    "protectedRequestBeforeRestart",
    "protectedRequestAfterRestart",
    "restartedActualServiceProcess",
]

REQUIRED_STRING_FIELDS = [
    "artifactDir",
    "credentialPath",
    "platform",
    "recordedAt",
    "result",
    "runId",
    "schema",
    "script",
    "serviceManager",
]


def load_records() -> list[tuple[Path, dict]]:
    if not EVIDENCE_DIR.exists():
        return []
    records: list[tuple[Path, dict]] = []
    for path in sorted(EVIDENCE_DIR.glob("*.json")):
        with path.open(encoding="utf-8") as handle:
            records.append((path, json.load(handle)))
    return records


def validate_record(path: Path, record: dict) -> list[str]:
    errors: list[str] = []
    for field in REQUIRED_STRING_FIELDS:
        if not isinstance(record.get(field), str) or not record[field].strip():
            errors.append(f"{path}: missing non-empty string field {field}")
    for field in REQUIRED_TRUE_FIELDS:
        if record.get(field) is not True:
            errors.append(f"{path}: {field} is not true")
    if record.get("schema") != SCHEMA:
        errors.append(f"{path}: schema is not {SCHEMA!r}")
    if record.get("result") != "passed":
        errors.append(f"{path}: result is not 'passed'")
    if record.get("tokenMaterialRecorded") is not False:
        errors.append(f"{path}: tokenMaterialRecorded must be false")
    process = record.get("processEvidence")
    if not isinstance(process, dict):
        errors.append(f"{path}: processEvidence must be an object")
    else:
        before = str(process.get("beforeRestartPid", "")).strip()
        after = str(process.get("afterRestartPid", "")).strip()
        if not before or not after:
            errors.append(f"{path}: beforeRestartPid/afterRestartPid must be present")
        elif before == after:
            errors.append(f"{path}: beforeRestartPid and afterRestartPid are identical")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--require",
        required=True,
        help="comma-separated required platforms, e.g. macos,linux,windows",
    )
    args = parser.parse_args()

    required = [item.strip() for item in args.require.split(",") if item.strip()]
    if not required:
        raise SystemExit("--require must name at least one platform")

    records = load_records()
    errors: list[str] = []
    by_platform: dict[str, list[tuple[Path, dict]]] = {}
    for path, record in records:
        errors.extend(validate_record(path, record))
        platform = record.get("platform")
        if isinstance(platform, str):
            by_platform.setdefault(platform, []).append((path, record))

    for platform in required:
        candidates = by_platform.get(platform, [])
        if not candidates:
            errors.append(f"missing required credential evidence for {platform}")
        elif not any(record.get("result") == "passed" for _, record in candidates):
            errors.append(f"no passing credential evidence for {platform}")

    if errors:
        raise SystemExit("\n".join(errors))

    print(
        "credential evidence ok: "
        + ", ".join(f"{platform}={len(by_platform.get(platform, []))}" for platform in required)
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
