#!/usr/bin/env python3
"""Check that the Phase 11 client scope covers the frozen client contract."""

from __future__ import annotations

import csv
import json
import re
import sys
from pathlib import Path


ROUTE_RE = re.compile(
    r"(?m)^- \`\[[^]]+\]\` \`(GET|POST|PUT|PATCH|DELETE|WS) (/v1/[^`]+)\`"
)
REQUIRED_MARKERS = (
    "d-003",
    "d-013",
    "d-021",
    "d-023",
    "d-026",
    "same-screen",
    "host-keyed",
    "helpid",
    "bedrock",
    "profiles",
    "unavailable",
)


def contract_routes() -> set[tuple[str, str]]:
    openapi = json.loads(Path("docs/msc2/api-contract/openapi.json").read_text())
    routes = {
        (method.upper(), path)
        for path, operations in openapi["paths"].items()
        for method in operations
        if method in {"get", "post", "put", "patch", "delete"}
    }
    websocket = json.loads(Path("docs/msc2/api-contract/websocket-v1.json").read_text())
    routes.update(("WS", channel["path"]) for channel in websocket["channels"])
    return routes


def matrix_routes(matrix_path: Path) -> set[tuple[str, str]]:
    with matrix_path.open(newline="") as handle:
        return {(row["method"], row["path"]) for row in csv.DictReader(handle)}


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: scope-check.py SCOPE.md MATRIX.csv", file=sys.stderr)
        return 2

    scope_path = Path(sys.argv[1])
    matrix_path = Path(sys.argv[2])
    scope = scope_path.read_text()
    lowered = scope.lower()
    documented_matches = ROUTE_RE.findall(scope)
    documented = set(documented_matches)
    matrix = matrix_routes(matrix_path)
    contract = contract_routes()

    errors: list[str] = []
    if matrix != contract:
        errors.append(
            "matrix and frozen HTTP/WebSocket contract differ: "
            f"missing={sorted(contract - matrix)} extra={sorted(matrix - contract)}"
        )
    if documented != matrix:
        errors.append(
            "scope route appendix does not match the capability matrix: "
            f"missing={sorted(matrix - documented)} extra={sorted(documented - matrix)}"
        )
    if len(documented_matches) != len(matrix):
        errors.append("scope route appendix contains duplicate or malformed route entries")
    missing_markers = [marker for marker in REQUIRED_MARKERS if marker not in lowered]
    if missing_markers:
        errors.append(f"scope is missing required decisions or boundaries: {missing_markers}")
    if "no profiles screen" not in lowered or "no bedrock section" not in lowered:
        errors.append("scope must preserve the reserved profile and Bedrock extension boundary")
    if "matrix changes:" not in lowered or not re.search(r"matrix changes:\s*none", lowered):
        errors.append("P11.1 must state that it changes no capability-matrix status")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(f"OK: {len(documented)} matrix and contract operations are scoped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
