#!/usr/bin/env python3
"""Phase 5 real-corpus checker (P5.2).

Gates P5.3's evidence collection, and later P5.24/P5.25's exercise run,
against docs/msc2/config-migration/phase5-scope.md's "Evidence required"
section: this fails loudly, not silently, when the real MSC 1 corpus that
section demands isn't actually there yet.

Inventory mode (the only mode this step builds) checks a corpus directory
against its own `manifest.json`:
  - at least one parseable JSON config file (two preferred, from distinct
    schema eras -- P5.3 relaxed this to one when a second era turned out to
    be genuinely unavailable; see phase5-scope.md's "Evidence required")
  - every config file has a manifest entry recording its source era and
    whether/how it was sanitized
  - no two config files hash identically -- a repeated file presented as a
    second sample doesn't exercise anything the first one didn't
  - $MSC2_PHASE5_TRANSFER_PACKAGE names an existing `.msctransfer` file

  real-corpus-check.py [--corpus-dir DIR]   check a corpus directory
                                             (default: corpus/configs)
  real-corpus-check.py --selftest           run inventory mode against the
                                             fixtures in
                                             tools/phase5/fixtures/, proving
                                             the passing case succeeds and
                                             every deliberately-broken case
                                             fails

P5.24 extends this file with an exercise mode that runs the real Rust
config/transfer readers against the same evidence; this step only builds
the inventory gate that later mode plugs into. `corpus/configs/` itself
stays empty until P5.3 supplies real MSC 1 evidence -- the fixtures this
step ships live under tools/phase5/fixtures/ instead, exactly so nothing
invented ends up in corpus/.

Stdlib only, on purpose: same reasoning as the Phase 0 checkers this one
follows the shape of -- no dependency setup for Cameron to fight.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

DEFAULT_CORPUS_DIR = Path("corpus/configs")
FIXTURES_DIR = Path(__file__).resolve().parent / "fixtures"
TRANSFER_ENV_VAR = "MSC2_PHASE5_TRANSFER_PACKAGE"

# (fixture directory name, expected inventory exit code, set the transfer
# env var to a real file inside that fixture before running it)
SELFTEST_CASES = [
    ("pass", 0, True),
    ("empty", 1, False),
    ("single-file", 1, False),
    ("duplicate", 1, False),
    ("malformed", 1, False),
    ("missing-transfer", 1, False),
]


class CheckError(Exception):
    pass


def load_manifest(corpus_dir: Path) -> dict:
    manifest_path = corpus_dir / "manifest.json"
    if not manifest_path.is_file():
        raise CheckError(f"{manifest_path}: missing provenance manifest")
    try:
        manifest = json.loads(manifest_path.read_text())
    except json.JSONDecodeError as exc:
        raise CheckError(f"{manifest_path}: malformed JSON manifest ({exc})")
    if not isinstance(manifest.get("files"), list) or not manifest["files"]:
        raise CheckError(f"{manifest_path}: manifest has no 'files' entries")
    return manifest


def config_files_in(corpus_dir: Path) -> list[Path]:
    return sorted(p for p in corpus_dir.glob("*.json") if p.name != "manifest.json")


def check_inventory(corpus_dir: Path, transfer_package: str | None) -> str:
    """Raises CheckError on the first evidence gap found; returns an "ok"
    message describing what passed otherwise."""
    if not corpus_dir.is_dir():
        raise CheckError(f"{corpus_dir}: corpus directory does not exist")

    config_files = config_files_in(corpus_dir)
    if len(config_files) < 1:
        raise CheckError(
            f"{corpus_dir}: found {len(config_files)} config file(s), need at least 1 "
            "(two preferred, from distinct schema eras)"
        )

    manifest = load_manifest(corpus_dir)
    manifest_by_name = {}
    for entry in manifest["files"]:
        name = entry.get("file")
        if not name:
            raise CheckError(f"{corpus_dir}/manifest.json: entry missing 'file'")
        if not entry.get("era"):
            raise CheckError(f"{corpus_dir}/manifest.json: {name} missing 'era'")
        if not entry.get("sanitized"):
            raise CheckError(f"{corpus_dir}/manifest.json: {name} missing 'sanitized'")
        manifest_by_name[name] = entry

    seen_hashes: dict[str, str] = {}
    for config_file in config_files:
        raw = config_file.read_bytes()
        try:
            json.loads(raw)
        except json.JSONDecodeError as exc:
            raise CheckError(f"{config_file}: malformed JSON ({exc})")

        if config_file.name not in manifest_by_name:
            raise CheckError(
                f"{config_file}: no manifest entry recording its source era and sanitization"
            )

        digest = hashlib.sha256(raw).hexdigest()
        if digest in seen_hashes:
            raise CheckError(
                f"{config_file}: identical SHA-256 to {seen_hashes[digest]} -- "
                "a duplicate isn't a second sample"
            )
        seen_hashes[digest] = config_file.name

    if not transfer_package:
        raise CheckError(f"${TRANSFER_ENV_VAR} is not set")
    transfer_path = Path(transfer_package)
    if not transfer_path.is_file():
        raise CheckError(f"{transfer_path}: ${TRANSFER_ENV_VAR} does not name an existing file")
    if transfer_path.suffix != ".msctransfer":
        raise CheckError(f"{transfer_path}: ${TRANSFER_ENV_VAR} must name a .msctransfer file")

    return f"ok {corpus_dir} ({len(config_files)} configs, transfer package present)"


def run_inventory(corpus_dir: Path, transfer_package: str | None) -> tuple[int, str]:
    try:
        message = check_inventory(corpus_dir, transfer_package)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


def selftest() -> tuple[int, list[str]]:
    lines = []
    all_ok = True
    for name, expected_code, needs_transfer_env in SELFTEST_CASES:
        fixture_dir = FIXTURES_DIR / name
        transfer_package = None
        if needs_transfer_env:
            candidates = list(fixture_dir.glob("*.msctransfer"))
            if not candidates:
                lines.append(f"fixtures/{name}: expected a .msctransfer file, found none")
                all_ok = False
                continue
            transfer_package = str(candidates[0])

        code, message = run_inventory(fixture_dir, transfer_package)
        ok = code == expected_code
        all_ok = all_ok and ok
        lines.append(f"{'pass' if ok else 'FAIL'} {name}: expected={expected_code} got={code} ({message})")

    return (0 if all_ok else 1), lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--corpus-dir", type=Path, default=DEFAULT_CORPUS_DIR)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        code, lines = selftest()
        for line in lines:
            print(line)
        return code

    code, message = run_inventory(args.corpus_dir, os.environ.get(TRANSFER_ENV_VAR))
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
