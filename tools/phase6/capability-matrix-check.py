#!/usr/bin/env python3
"""D-023 client capability matrix checker (P6.8).

Used as (part of) the Verify command for docs/msc2/rolling-plan.md's P6.8
step. Checks docs/msc2/client-capability-matrix.csv, the matrix
docs/msc2/msc2-decisions.md D-023 requires ("Full client capability, tracked
by an explicit matrix") and rolling-plan.md's Phase 6 header calls "the
overdue D-023 matrix", against the two things it must stay true to:

  1. Shape: the header is exact, every row has all ten fields, every status
     cell (agent/desktop_web/ios/cli) is one of D-023's own three values
     (Implemented, Planned, Intentional exception), and an Intentional
     exception cell names the D-0xx decision that approved it (D-023: "An
     Intentional exception requires owner approval and becomes its own
     decision entry").
  2. Coverage against docs/msc2/api-contract/openapi.json and
     docs/msc2/api-contract/websocket-v1.json: every (method, path) in the
     real contract has exactly one row in the matrix -- no operation the
     contract defines is silently untracked, and no row in the matrix names
     an operation the contract doesn't (or no longer) define.
  3. rolling-plan.md's own Phase 6 rule that desktop_web_status is Planned
     on every row this phase, without exception ("Desktop/web screens stay
     Phase 11... that is not an exception").

Stdlib only, in the style of tools/api-contract-check.py (P2.8) and
tools/phase6/corpus-check.py (P6.2).

  capability-matrix-check.py <csv-path>   check a real matrix
  capability-matrix-check.py --selftest   run the bundled clean/dirty fixtures
"""

import csv
import json
import re
import sys

HEADER = [
    "method",
    "path",
    "operation_id",
    "msc1_capability",
    "permission_category",
    "agent_status",
    "desktop_web_status",
    "ios_status",
    "cli_status",
    "notes",
]

STATUS_VALUES = {"Implemented", "Planned", "Intentional exception"}
STATUS_COLUMNS = ["agent_status", "desktop_web_status", "ios_status", "cli_status"]
# operation_id is blank for the two WS channels (no OpenAPI operationId concept
# applies to them) and notes is blank whenever a row has nothing to add --
# both legitimately optional, unlike every other column.
REQUIRED_NONBLANK = [c for c in HEADER if c not in ("operation_id", "notes")]
DECISION_RE = re.compile(r"D-\d{3}")

OPENAPI_PATH = "docs/msc2/api-contract/openapi.json"
WEBSOCKET_PATH = "docs/msc2/api-contract/websocket-v1.json"


def contract_operations(openapi_path=OPENAPI_PATH, websocket_path=WEBSOCKET_PATH):
    """The real (method, path) set a matrix row must exist for, one-to-one."""
    with open(openapi_path) as f:
        doc = json.load(f)
    ops = set()
    for path, methods in doc.get("paths", {}).items():
        for method in methods:
            ops.add((method.upper(), path))

    with open(websocket_path) as f:
        ws = json.load(f)
    for channel in ws.get("channels", []):
        ops.add(("WS", channel["path"]))

    return ops


def load_rows(csv_path):
    with open(csv_path, newline="") as f:
        reader = csv.reader(f)
        rows = list(reader)
    if not rows:
        return None, []
    return rows[0], rows[1:]


def check_matrix(csv_path, contract_ops=None):
    """Returns (exit_code, list-of-problem-strings)."""
    problems = []

    header, data_rows = load_rows(csv_path)
    if header is None:
        return 1, ["empty file"]
    if header != HEADER:
        problems.append(f"header mismatch: {header}")
        return 1, problems

    if contract_ops is None:
        contract_ops = contract_operations()

    seen_ops = set()
    for i, row in enumerate(data_rows, start=2):  # 1-indexed + header line
        if len(row) != len(HEADER):
            problems.append(f"line {i}: expected {len(HEADER)} fields, got {len(row)}")
            continue
        rec = dict(zip(HEADER, row))

        for field in REQUIRED_NONBLANK:
            if rec[field].strip() == "":
                problems.append(f"line {i}: blank {field}")

        op = (rec["method"], rec["path"])
        if op in seen_ops:
            problems.append(f"line {i}: duplicate row for {op}")
        seen_ops.add(op)

        for col in STATUS_COLUMNS:
            val = rec[col]
            if val not in STATUS_VALUES:
                problems.append(f"line {i}: {col}={val!r} not one of {sorted(STATUS_VALUES)}")

        if rec["desktop_web_status"] != "Planned":
            problems.append(
                f"line {i}: desktop_web_status={rec['desktop_web_status']!r}, "
                f"must be Planned this phase (rolling-plan.md Phase 6 preamble)"
            )

        for col in STATUS_COLUMNS:
            if rec[col] == "Intentional exception" and not DECISION_RE.search(rec["notes"]):
                problems.append(
                    f"line {i}: {col}=Intentional exception but notes ({rec['notes']!r}) "
                    f"names no D-0xx decision (D-023 requirement)"
                )

    missing = contract_ops - seen_ops
    if missing:
        problems.append(f"{len(missing)} contract operation(s) with no matrix row: {sorted(missing)[:10]}")

    orphans = seen_ops - contract_ops
    if orphans:
        problems.append(f"{len(orphans)} matrix row(s) naming an operation not in the contract: {sorted(orphans)[:10]}")

    return (1 if problems else 0), problems


CLEAN_OPS = {("GET", "/v1/x"), ("POST", "/v1/x"), ("WS", "/v1/y/stream")}


def _clean_rows():
    return [
        HEADER,
        ["GET", "/v1/x", "getX", "list Xs", "none", "Implemented", "Planned", "Implemented", "Planned", ""],
        ["POST", "/v1/x", "createX", "create an X", "worlds", "Planned", "Planned", "Planned",
         "Intentional exception", "CLI has no batch-create UI, approved D-999"],
        ["WS", "/v1/y/stream", "", "stream Ys", "none", "Planned", "Planned", "Planned", "Planned", ""],
    ]


def _dirty_rows():
    return [
        HEADER,
        # blank field, bad status value, desktop_web not Planned, exception with no decision ref,
        # duplicate, missing the WS row entirely, and one orphan row.
        ["GET", "/v1/x", "getX", "", "none", "Implemented", "Planned", "Implemented", "Planned", ""],
        ["GET", "/v1/x", "getX", "list Xs", "none", "Implemented", "Planned", "Implemented", "Planned", ""],
        ["POST", "/v1/x", "createX", "create an X", "worlds", "sorta", "Implemented",
         "Intentional exception", "Planned", "no decision cited here"],
        ["POST", "/v1/nonexistent", "ghost", "not real", "none", "Planned", "Planned", "Planned", "Planned", ""],
    ]


def _write_csv(path, rows):
    with open(path, "w", newline="") as f:
        csv.writer(f).writerows(rows)


def selftest():
    import tempfile, os

    lines = []
    with tempfile.TemporaryDirectory() as tmp:
        clean_path = os.path.join(tmp, "clean.csv")
        dirty_path = os.path.join(tmp, "dirty.csv")
        _write_csv(clean_path, _clean_rows())
        _write_csv(dirty_path, _dirty_rows())

        pass_code, pass_problems = check_matrix(clean_path, contract_ops=CLEAN_OPS)
        fail_code, fail_problems = check_matrix(dirty_path, contract_ops=CLEAN_OPS)

    lines.append(f"pass={pass_code}")
    if pass_problems:
        lines.extend(f"  unexpected: {p}" for p in pass_problems)
    lines.append(f"fail={fail_code} problems={len(fail_problems)}")

    ok = pass_code == 0 and fail_code != 0 and len(fail_problems) >= 5
    return (0 if ok else 1), lines


def main():
    args = sys.argv[1:]
    if args == ["--selftest"]:
        code, lines = selftest()
        for line in lines:
            print(line)
        sys.exit(code)

    if len(args) != 1:
        print(__doc__)
        sys.exit(2)

    code, problems = check_matrix(args[0])
    out = sys.stdout if code == 0 else sys.stderr
    if code == 0:
        ops = contract_operations()
        print(f"ok: {len(ops)} contract operations, all matched", file=out)
    else:
        for p in problems:
            print(p, file=out)
    sys.exit(code)


if __name__ == "__main__":
    main()
