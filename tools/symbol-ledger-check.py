#!/usr/bin/env python3
"""Symbol ledger bucket-count checker (P0.26a).

Used as P0.27's Verify command. Counts the unique `file` values in
docs/msc2/audit/msc2-symbol-ledger.csv for a given bucket, re-runs P0.25's
density scanner (tools/symbol-scan/scan.py --bucket ui --min-hits 3) against
a live MSC 1 source tree, and asserts the two counts match exactly -- so the
check stays live against whatever the scanner currently finds, never a
number frozen in the plan.

Usage:
    symbol-ledger-check.py <bucket> --scan-source <path-to-msc1-source-root>
    symbol-ledger-check.py --selftest

Stdlib only, on purpose -- same rule as every other Phase 0 tool.
"""

import argparse
import csv
import os
import subprocess
import sys
import tempfile

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LEDGER_CSV = os.path.join(REPO_ROOT, "docs", "msc2", "audit", "msc2-symbol-ledger.csv")
SCANNER = os.path.join(REPO_ROOT, "tools", "symbol-scan", "scan.py")

FIELDNAMES = ["file", "bucket", "symbol", "kind", "disposition", "target_domain", "source_line", "notes"]


def ledger_file_count(csv_path, bucket):
    with open(csv_path, newline="") as f:
        rows = list(csv.DictReader(f))
    return len({r["file"] for r in rows if r["bucket"] == bucket})


def scanner_file_count(scan_source):
    result = subprocess.run(
        [sys.executable, SCANNER, "--bucket", "ui", "--min-hits", "3", scan_source],
        capture_output=True, text=True,
    )
    lines = [line for line in result.stdout.splitlines() if line.strip()]
    return len(lines)


def check(bucket, scan_source, ledger_csv=LEDGER_CSV):
    ledger_count = ledger_file_count(ledger_csv, bucket)
    scan_count = scanner_file_count(scan_source)
    if ledger_count != scan_count:
        print(
            f"mismatch: ledger has {ledger_count} distinct file(s) in bucket "
            f"'{bucket}', live scan found {scan_count}",
            file=sys.stderr,
        )
        return 1
    print(f"ok {ledger_count}")
    return 0


def _write_temp_csv(rows):
    fd, path = tempfile.mkstemp(suffix=".csv")
    with os.fdopen(fd, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=FIELDNAMES)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)
    return path


def _row(file, bucket="ui-flagged"):
    return {
        "file": file, "bucket": bucket, "symbol": "x", "kind": "parser",
        "disposition": "agent", "target_domain": "n/a", "source_line": "1", "notes": "selftest",
    }


def selftest():
    scan_out = subprocess.run(
        [sys.executable, SCANNER, "--bucket", "ui", "--min-hits", "3",
         os.path.expanduser("~/Documents/Swift Projects/minecraft-server-controller")],
        capture_output=True, text=True,
    ).stdout
    live_files = [line.split(" ", 1)[1] for line in scan_out.splitlines() if line.strip()]

    if not live_files:
        print("selftest requires a reachable MSC 1 source tree with at least one flagged file", file=sys.stderr)
        return 1

    matching_csv = _write_temp_csv([_row(f) for f in live_files])
    short_csv = _write_temp_csv([_row(f) for f in live_files[:-1]])

    try:
        scan_source = os.path.expanduser("~/Documents/Swift Projects/minecraft-server-controller")
        pass_code = check("ui-flagged", scan_source, ledger_csv=matching_csv)
        fail_code = check("ui-flagged", scan_source, ledger_csv=short_csv)
        print(f"pass={pass_code}")
        print(f"fail={fail_code}")
        return 0
    finally:
        os.remove(matching_csv)
        os.remove(short_csv)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("bucket", nargs="?", help="ledger bucket to check, e.g. 'ui-flagged'")
    parser.add_argument("--scan-source", help="path to the MSC 1 source tree")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        return selftest()

    if not args.bucket or not args.scan_source:
        parser.error("bucket and --scan-source are required unless --selftest is given")

    return check(args.bucket, args.scan_source)


if __name__ == "__main__":
    sys.exit(main())
