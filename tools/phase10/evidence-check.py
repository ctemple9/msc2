#!/usr/bin/env python3
"""Validate Phase 10 Bedrock distribution and runtime evidence.

This check is deliberately separate from runtime checks.  A distribution can
be documented without proving that a native process or VM can run on a host.
It also refuses to turn a mutable third-party manifest or a fixture into an
official package identity claim.

Usage:
    evidence-check.py --distribution
    evidence-check.py --runtimes
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from pathlib import Path
from urllib.parse import urlparse


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE_DIR = ROOT / "docs" / "msc2" / "bedrock" / "evidence"
MATRIX_PATH = ROOT / "docs" / "msc2" / "bedrock" / "compatibility-matrix.csv"
SCHEMA = "msc2.phase10.distribution-evidence.v1"
RUNTIME_SCHEMA = "msc2.phase10.runtime-evidence.v1"
MATRIX_HEADER = [
    "host",
    "architecture",
    "agent_host_status",
    "agent_host_evidence",
    "bedrock_backend",
    "bedrock_runtime_status",
    "bedrock_runtime_evidence",
    "runtime_requirements",
    "notes",
]
REQUIRED_CELLS = {
    ("Linux (Debian 12)", "x86_64", "native-linux-bds"),
    ("Windows", "x86_64", "native-windows-bds"),
    ("macOS (Intel)", "x86_64", "macos-vz-swift-sidecar"),
    ("macOS (Apple Silicon)", "arm64", "macos-vz-swift-sidecar"),
}
RUNTIME_CHECKS = ("lifecycle", "udp_reachability", "clean_termination", "crash_termination")
RUNTIME_STATUSES = {"supported", "unsupported", "unavailable"}
SHA256 = re.compile(r"^[0-9a-fA-F]{64}$")


class EvidenceError(Exception):
    """A user-facing evidence validation failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def official_minecraft_url(value: object) -> bool:
    if not isinstance(value, str) or not value.strip():
        return False
    parsed = urlparse(value)
    return parsed.scheme == "https" and parsed.hostname in {"minecraft.net", "www.minecraft.net"}


def load_matrix() -> list[dict[str, str]]:
    try:
        with MATRIX_PATH.open(newline="", encoding="utf-8-sig") as handle:
            reader = csv.DictReader(handle)
            require(reader.fieldnames == MATRIX_HEADER, "compatibility matrix header mismatch")
            rows = list(reader)
    except (OSError, csv.Error) as error:
        raise EvidenceError(f"cannot read compatibility matrix: {error}") from error

    require(rows, "compatibility matrix has no rows")
    seen: set[tuple[str, str, str]] = set()
    for number, row in enumerate(rows, start=2):
        require(None not in row, f"matrix line {number}: malformed CSV row")
        cell = (row["host"], row["architecture"], row["bedrock_backend"])
        require(cell not in seen, f"matrix line {number}: duplicate cell {cell!r}")
        seen.add(cell)
        require(
            row["bedrock_runtime_status"] in {"supported", "unsupported", "unavailable", "planned"},
            f"matrix line {number}: invalid Bedrock runtime status",
        )

    require(REQUIRED_CELLS <= seen, "matrix is missing a required Phase 10 Bedrock cell")
    return rows


def validate_record(path: Path, record: object) -> tuple[str, str, str]:
    require(isinstance(record, dict), f"{path}: record must be a JSON object")
    require(record.get("schema") == SCHEMA, f"{path}: schema must be {SCHEMA!r}")
    require(record.get("kind") == "official-distribution", f"{path}: kind is not official-distribution")

    cell_fields = ("host", "architecture", "bedrock_backend")
    for field in cell_fields:
        require(isinstance(record.get(field), str) and record[field].strip(), f"{path}: missing {field}")

    distribution = record.get("distribution")
    require(isinstance(distribution, dict), f"{path}: distribution must be an object")
    require(
        distribution.get("publisher") == "Mojang Studios / Microsoft",
        f"{path}: distribution publisher must identify Mojang Studios / Microsoft",
    )
    require(official_minecraft_url(distribution.get("official_source")), f"{path}: official_source is not a minecraft.net URL")
    require(
        distribution.get("package_platform") in {"linux", "windows", "none"},
        f"{path}: package_platform must be linux, windows, or none",
    )

    identity = distribution.get("package_identity")
    require(isinstance(identity, dict), f"{path}: package_identity must be an object")
    status = identity.get("status")
    require(status in {"verified", "unavailable"}, f"{path}: package identity status is invalid")
    if status == "verified":
        require(isinstance(identity.get("version"), str) and identity["version"].strip(), f"{path}: verified identity needs a version")
        require(official_minecraft_url(identity.get("archive_url")), f"{path}: verified identity needs an official archive URL")
        require(isinstance(identity.get("sha256"), str) and SHA256.fullmatch(identity["sha256"]), f"{path}: verified identity needs a SHA-256 digest")
        require(identity.get("verification") == "sha256", f"{path}: verified identity must name SHA-256 verification")
        require(identity.get("archive_captured") is True, f"{path}: verified identity needs archive_captured=true")
    else:
        require(isinstance(identity.get("reason"), str) and identity["reason"].strip(), f"{path}: unavailable identity needs a reason")
        require(identity.get("sha256") is None, f"{path}: unavailable identity cannot contain a package digest")
        require(identity.get("verification") == "not-verified", f"{path}: unavailable identity must say not-verified")
        require(identity.get("archive_captured") is False, f"{path}: unavailable identity needs archive_captured=false")

    capture = record.get("capture")
    require(isinstance(capture, dict), f"{path}: capture must be an object")
    for field in ("recorded_at", "method", "result"):
        require(isinstance(capture.get(field), str) and capture[field].strip(), f"{path}: capture missing {field}")
    require(capture["result"] == status, f"{path}: capture result does not match package identity status")
    require(isinstance(record.get("limits"), list) and record["limits"], f"{path}: limits must be a non-empty list")

    return tuple(record[field] for field in cell_fields)


def validate_runtime_record(path: Path, record: object) -> tuple[tuple[str, str, str], str]:
    require(isinstance(record, dict), f"{path}: record must be a JSON object")
    require(record.get("schema") == RUNTIME_SCHEMA, f"{path}: schema must be {RUNTIME_SCHEMA!r}")
    require(record.get("kind") == "runtime", f"{path}: kind is not runtime")

    cell_fields = ("host", "architecture", "bedrock_backend")
    for field in cell_fields:
        require(isinstance(record.get(field), str) and record[field].strip(), f"{path}: missing {field}")

    status = record.get("status")
    require(status in RUNTIME_STATUSES, f"{path}: runtime status is invalid")

    distribution = record.get("distribution")
    require(isinstance(distribution, dict), f"{path}: distribution must be an object")
    require(distribution.get("status") == "unavailable", f"{path}: distribution status must be unavailable")
    require(
        isinstance(distribution.get("reason"), str) and distribution["reason"].strip(),
        f"{path}: unavailable distribution needs a reason",
    )

    environment = record.get("environment")
    require(isinstance(environment, dict), f"{path}: environment must be an object")
    for field in ("host_os", "host_os_version", "required_resources"):
        value = environment.get(field)
        if field == "required_resources":
            require(isinstance(value, list) and value, f"{path}: environment missing {field}")
        else:
            require(isinstance(value, str) and value.strip(), f"{path}: environment missing {field}")

    checks = record.get("checks")
    require(isinstance(checks, dict), f"{path}: checks must be an object")
    for field in RUNTIME_CHECKS:
        check = checks.get(field)
        require(isinstance(check, dict), f"{path}: checks missing {field}")
        require(check.get("status") in RUNTIME_STATUSES, f"{path}: {field} status is invalid")
        require(isinstance(check.get("result"), str) and check["result"].strip(), f"{path}: {field} needs a result")

    reproduction = record.get("reproduction")
    require(isinstance(reproduction, dict), f"{path}: reproduction must be an object")
    for field in ("recorded_at", "command", "result"):
        require(isinstance(reproduction.get(field), str) and reproduction[field].strip(), f"{path}: reproduction missing {field}")

    synthetic = record.get("synthetic_boundary")
    require(isinstance(synthetic, dict), f"{path}: synthetic_boundary must be an object")
    for field in ("command", "result"):
        require(isinstance(synthetic.get(field), str) and synthetic[field].strip(), f"{path}: synthetic_boundary missing {field}")
    require(
        synthetic.get("proves_runtime_support") is False,
        f"{path}: synthetic boundary must not claim runtime support",
    )
    require(isinstance(record.get("limits"), list) and record["limits"], f"{path}: limits must be a non-empty list")

    return tuple(record[field] for field in cell_fields), status


def check_distribution() -> str:
    rows = load_matrix()
    records: dict[tuple[str, str, str], Path] = {}
    errors: list[str] = []

    for path in sorted(EVIDENCE_DIR.glob("*.json")):
        try:
            record = json.loads(path.read_text(encoding="utf-8-sig"))
            if isinstance(record, dict) and record.get("kind") == "runtime":
                continue
            cell = validate_record(path, record)
        except (OSError, json.JSONDecodeError, EvidenceError) as error:
            errors.append(str(error))
            continue
        if cell in records:
            errors.append(f"{path}: duplicate distribution evidence cell {cell!r} (already in {records[cell]})")
        records[cell] = path

    for cell in REQUIRED_CELLS:
        if cell not in records:
            errors.append(f"missing distribution evidence for {cell!r}")

    matrix_by_cell = {
        (row["host"], row["architecture"], row["bedrock_backend"]): row for row in rows
    }
    for cell, row in matrix_by_cell.items():
        status = row["bedrock_runtime_status"]
        reference = row["bedrock_runtime_evidence"].strip()
        if status == "planned":
            if reference:
                errors.append(f"matrix cell {cell!r}: planned status must not cite runtime evidence")
            continue
        if not reference:
            errors.append(f"matrix cell {cell!r}: advertised status needs runtime evidence")
            continue
        raw_path = reference.split("#", 1)[0]
        evidence_path = (ROOT / raw_path).resolve()
        if not evidence_path.is_file():
            errors.append(f"matrix cell {cell!r}: missing runtime evidence {raw_path!r}")
        if status == "supported":
            record_path = records.get(cell)
            if record_path is None or evidence_path != record_path.resolve():
                errors.append(f"matrix cell {cell!r}: supported status is not linked to its distribution record")
            else:
                record = json.loads(record_path.read_text(encoding="utf-8-sig"))
                identity = record["distribution"]["package_identity"]
                if identity["status"] != "verified":
                    errors.append(f"matrix cell {cell!r}: supported status has no verified package identity")

    apple = matrix_by_cell[("macOS (Apple Silicon)", "arm64", "macos-vz-swift-sidecar")]
    require(apple["bedrock_runtime_status"] == "unavailable", "Apple Silicon Bedrock status must remain unavailable")
    require("d-028" in apple["bedrock_runtime_evidence"].lower(), "Apple Silicon evidence must cite D-028")

    if errors:
        raise EvidenceError("\n".join(errors))
    return f"ok: {len(records)} official distribution records; {len(rows)} matrix rows checked"


def check_runtimes() -> str:
    rows = load_matrix()
    records: dict[tuple[str, str, str], tuple[Path, str]] = {}
    errors: list[str] = []

    for path in sorted(EVIDENCE_DIR.glob("*-runtime*.json")):
        try:
            record = json.loads(path.read_text(encoding="utf-8-sig"))
            cell, status = validate_runtime_record(path, record)
        except (OSError, json.JSONDecodeError, EvidenceError) as error:
            errors.append(str(error))
            continue
        if cell in records:
            errors.append(f"{path}: duplicate runtime evidence cell {cell!r} (already in {records[cell][0]})")
        records[cell] = (path, status)

    for cell in REQUIRED_CELLS:
        if cell not in records:
            errors.append(f"missing runtime evidence for {cell!r}")

    matrix_by_cell = {
        (row["host"], row["architecture"], row["bedrock_backend"]): row for row in rows
    }
    for cell, (record_path, status) in records.items():
        row = matrix_by_cell.get(cell)
        if row is None:
            errors.append(f"runtime evidence {record_path}: cell is absent from compatibility matrix {cell!r}")
            continue
        if row["bedrock_runtime_status"] != status:
            errors.append(
                f"matrix cell {cell!r}: status {row['bedrock_runtime_status']!r} disagrees with runtime evidence {status!r}"
            )
        reference = row["bedrock_runtime_evidence"].strip()
        raw_path = reference.split("#", 1)[0]
        evidence_path = (ROOT / raw_path).resolve()
        if evidence_path != record_path.resolve():
            errors.append(f"matrix cell {cell!r}: runtime evidence is not linked to {record_path.name!r}")

    for cell, row in matrix_by_cell.items():
        if cell in REQUIRED_CELLS and row["bedrock_runtime_status"] == "planned":
            errors.append(f"matrix cell {cell!r}: runtime evidence step left status planned")
        if row["bedrock_runtime_status"] in RUNTIME_STATUSES and cell not in records:
            errors.append(f"matrix cell {cell!r}: advertised runtime status has no runtime record")

    if errors:
        raise EvidenceError("\n".join(errors))
    return f"ok: {len(records)} runtime records; {len(rows)} matrix rows checked"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--distribution", action="store_true", help="check official distribution evidence")
    parser.add_argument("--runtimes", action="store_true", help="check native and sidecar runtime evidence")
    args = parser.parse_args()
    if args.distribution == args.runtimes:
        parser.error("choose exactly one of --distribution or --runtimes")
    try:
        print(check_distribution() if args.distribution else check_runtimes())
    except EvidenceError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
