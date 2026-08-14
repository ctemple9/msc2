#!/usr/bin/env python3
"""UI-bucket density scanner (P0.25).

Reproduces the audit's own D1 finding (docs/msc2/audit/msc2-audit-
reconciliation.md, "D1 -- The Mixed bucket"): grep MSC 1's UI-bucket files
for engine-shaped symbols and rank by hit count, so P0.27 has a live list
of UI files that need symbol-level extraction before they can be retired --
not a number frozen in a document that may have gone stale since it was
written.

Usage:
    scan.py --bucket ui --min-hits 3 <path-to-msc1-source-root>

For each file the inventory CSV (docs/msc2/audit/msc2-file-inventory-b.csv)
marks with the given --bucket, finds that file under <source-root> and counts
occurrences of these patterns, comments stripped:
    - FileManager
    - Process(
    - URLSession
    - JSONDecoder
    - func parse*/detect*/validate*/resolve*
    - string-range extraction (.range(of:, NSRange, NSRegularExpression, String.Index)

Prints one "<count> <file>" line per file at or above --min-hits, ranked by
hit count descending (ties broken alphabetically), to stdout.

Stdlib only, on purpose -- same rule as every other Phase 0 tool.
"""

import argparse
import csv
import os
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
INVENTORY_CSV = os.path.join(REPO_ROOT, "docs", "msc2", "audit", "msc2-file-inventory-b.csv")

LINE_COMMENT_RE = re.compile(r"//.*")
BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/", re.DOTALL)

PATTERNS = [
    re.compile(r"FileManager"),
    re.compile(r"Process\("),
    re.compile(r"URLSession"),
    re.compile(r"JSONDecoder"),
    re.compile(r"\bfunc (parse|detect|validate|resolve)\w*\("),
    # "string-range extraction": text-slicing operations characteristic of
    # hand-rolled parsers, not the display-truncation .prefix/.suffix calls
    # that show up throughout ordinary UI code.
    re.compile(r"\.range\(of:"),
    re.compile(r"NSRegularExpression"),
    re.compile(r"NSRange"),
    re.compile(r"String\.Index"),
]


def strip_comments(text):
    text = BLOCK_COMMENT_RE.sub("", text)
    text = LINE_COMMENT_RE.sub("", text)
    return text


def load_inventory_files(bucket):
    with open(INVENTORY_CSV, newline="") as f:
        rows = list(csv.DictReader(f))
    return [r["file"] for r in rows if r["bucket"] == bucket]


def find_file(source_root, filename):
    for dirpath, _dirnames, filenames in os.walk(source_root):
        if filename in filenames:
            return os.path.join(dirpath, filename)
    return None


def count_hits(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        text = strip_comments(f.read())
    return sum(len(p.findall(text)) for p in PATTERNS)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("source_root", help="path to the MSC 1 source tree")
    parser.add_argument("--bucket", required=True, help="inventory bucket to scan, e.g. 'ui'")
    parser.add_argument("--min-hits", type=int, default=1)
    args = parser.parse_args()

    filenames = load_inventory_files(args.bucket)
    results = []
    missing = []
    for filename in filenames:
        path = find_file(args.source_root, filename)
        if path is None:
            missing.append(filename)
            continue
        hits = count_hits(path)
        if hits >= args.min_hits:
            results.append((hits, filename))

    if missing:
        print(f"warning: {len(missing)} inventoried file(s) not found under {args.source_root}: {', '.join(missing)}", file=sys.stderr)

    results.sort(key=lambda t: (-t[0], t[1]))
    for hits, filename in results:
        print(f"{hits} {filename}")


if __name__ == "__main__":
    main()
