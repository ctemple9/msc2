#!/usr/bin/env python3
"""Check the separate Phase 10 Bedrock compatibility matrix.

The matrix keeps agent-host support independent from Bedrock-runtime support.
Only an explicit ``planned`` runtime cell is allowed to wait for later
evidence; every advertised status must point to a repository file that can be
inspected and reproduced.  This is intentionally stdlib-only so the checker
works on a fresh checkout.

Usage:
    compatibility-check.py MATRIX.csv
    compatibility-check.py MATRIX.csv --require-cell "macOS (Apple Silicon)=unavailable"
"""

from __future__ import annotations

import argparse
import csv
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HEADER = [
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
STATUSES = {"supported", "unsupported", "unavailable", "planned"}
ADVERTISED_STATUSES = STATUSES - {"planned"}
REQUIRED_FIELDS = {
    "host",
    "architecture",
    "agent_host_status",
    "bedrock_backend",
    "bedrock_runtime_status",
    "runtime_requirements",
    "notes",
}


class MatrixError(Exception):
    """A user-facing matrix validation failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise MatrixError(message)


def evidence_path(reference: str, *, row_number: int, column: str) -> Path:
    """Resolve a repository-relative evidence path and optional heading anchor."""
    raw_path, _, anchor = reference.partition("#")
    raw_path = raw_path.strip()
    require(raw_path, f"line {row_number}: {column} has no file path")
    path = (ROOT / raw_path).resolve()
    require(path.is_relative_to(ROOT), f"line {row_number}: {column} escapes the repository")
    require(path.is_file(), f"line {row_number}: {column} references missing file {raw_path!r}")
    if anchor:
        expected = anchor.strip().lower()
        require(expected, f"line {row_number}: {column} has an empty heading anchor")
        headings = []
        for line in path.read_text(encoding="utf-8-sig").splitlines():
            if line.startswith("#"):
                heading = re.sub(r"^#+\s*", "", line).lower()
                slug = re.sub(r"[^a-z0-9 -]", "", heading).strip().replace(" ", "-")
                headings.append(slug)
        require(
            any(slug == expected or slug.startswith(f"{expected}-") for slug in headings),
            f"line {row_number}: {column} references missing heading #{anchor.strip()}",
        )
    return path


def check_matrix(matrix_path: Path, required_cell: str | None = None) -> tuple[int, int]:
    try:
        with matrix_path.open(newline="", encoding="utf-8-sig") as handle:
            reader = csv.DictReader(handle)
            require(reader.fieldnames == HEADER, f"header mismatch: {reader.fieldnames!r}")
            rows = list(reader)

        require(rows, "matrix has no data rows")
        seen_cells: set[tuple[str, str, str]] = set()
        apple_rows: list[dict[str, str]] = []
        intel_rows: list[dict[str, str]] = []

        for row_number, row in enumerate(rows, start=2):
            require(None not in row, f"line {row_number}: malformed CSV row")
            for field in REQUIRED_FIELDS:
                require(row[field].strip(), f"line {row_number}: blank {field}")

            cell = (row["host"], row["architecture"], row["bedrock_backend"])
            require(cell not in seen_cells, f"line {row_number}: duplicate cell {cell!r}")
            seen_cells.add(cell)

            for status_field in ("agent_host_status", "bedrock_runtime_status"):
                status = row[status_field]
                require(
                    status in STATUSES,
                    f"line {row_number}: {status_field}={status!r} is not one of {sorted(STATUSES)}",
                )
                evidence_field = (
                    "agent_host_evidence"
                    if status_field == "agent_host_status"
                    else "bedrock_runtime_evidence"
                )
                evidence = row[evidence_field].strip()
                if status in ADVERTISED_STATUSES:
                    require(
                        evidence,
                        f"line {row_number}: advertised {status_field}={status!r} needs {evidence_field}",
                    )
                    evidence_path(evidence, row_number=row_number, column=evidence_field)
                else:
                    require(
                        not evidence,
                        f"line {row_number}: planned {status_field} must not cite evidence",
                    )

            if row["host"] == "macOS (Apple Silicon)":
                apple_rows.append(row)
            if row["host"] == "macOS (Intel)":
                intel_rows.append(row)

        require(
            {row["bedrock_backend"] for row in rows}
            >= {"native-linux-bds", "native-windows-bds", "macos-vz-swift-sidecar"},
            "matrix must name native Linux, native Windows, and macOS VZ sidecar backends",
        )

        require(
            len(apple_rows) == 1,
            "matrix must contain exactly one distinct macOS (Apple Silicon) cell",
        )
        require(
            len(intel_rows) == 1,
            "matrix must contain exactly one distinct macOS (Intel) cell",
        )
        require(
            intel_rows[0]["architecture"] == "x86_64"
            and intel_rows[0]["bedrock_backend"] == "macos-vz-swift-sidecar",
            "macOS (Intel) must be an x86_64 VZ Swift sidecar row",
        )
        apple = apple_rows[0]
        require(
            apple["architecture"] == "arm64",
            "macOS (Apple Silicon) must be an arm64 row",
        )
        require(
            apple["bedrock_backend"] == "macos-vz-swift-sidecar",
            "macOS (Apple Silicon) must name the macOS VZ Swift sidecar",
        )
        require(
            apple["bedrock_runtime_status"] == "unavailable",
            "macOS (Apple Silicon) Bedrock status must be exactly unavailable",
        )
        require(
            "no test hardware" in apple["notes"].lower()
            or "no test hardware" in apple["runtime_requirements"].lower(),
            "macOS (Apple Silicon) unavailable reason must cite no test hardware",
        )
        require(
            "d-028" in apple["bedrock_runtime_evidence"].lower(),
            "macOS (Apple Silicon) evidence must cite D-028",
        )

        if required_cell is not None:
            try:
                host, expected_status = required_cell.rsplit("=", 1)
            except ValueError as error:
                raise MatrixError("--require-cell must have the form HOST=STATUS") from error
            host = host.strip()
            expected_status = expected_status.strip()
            matches = [row for row in rows if row["host"] == host]
            require(matches, f"required cell host not found: {host!r}")
            matching_status = [
                row
                for row in matches
                if row["bedrock_runtime_status"] == expected_status
            ]
            require(
                len(matching_status) == 1,
                f"required cell {required_cell!r} did not identify exactly one runtime cell",
            )

        return len(rows), len(seen_cells)
    except (OSError, csv.Error, MatrixError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 0, 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("matrix", type=Path)
    parser.add_argument("--require-cell", metavar="HOST=STATUS")
    args = parser.parse_args()

    rows, cells = check_matrix(args.matrix, args.require_cell)
    if not rows:
        return 1
    print(f"OK: {rows} Bedrock compatibility rows, {cells} distinct backend cells")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
