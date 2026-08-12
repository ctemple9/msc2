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

Exercise mode (P5.24) still runs every inventory check above -- it never
replaces them -- and then shells out to `cargo test` twice, once per real
Rust reader:

  - `crates/msc-infrastructure/tests/historical_config_corpus.rs`, pointed
    at the configs directory via `MSC2_HISTORICAL_CONFIGS_DIR`, decodes
    every manifest-listed config through `load_app_config`, re-encodes it
    through `save_app_config`, decodes the result again, and reports each
    file independently.
  - `crates/msc-application/tests/real_transfer_corpus.rs`, pointed at a
    `.msctransfer` package via `MSC2_TRANSFER_PACKAGE_PATH`, inspects and
    applies it into a temporary owned root and checks at least one server
    arrives with its manifest-declared payload.

  real-corpus-check.py --exercise [--configs-dir DIR]
                        [--transfer-package PATH] [--require-configs N]
                        [--require-transfer]
                                             run the exercise checks above
                                             (default configs dir: same as
                                             --corpus-dir)
  real-corpus-check.py --exercise-selftest  run exercise mode against
                                             tools/phase5/fixtures/exercise-pass/,
                                             proving the wiring above
                                             actually works end to end

Both readers leave the corpus evidence itself untouched -- they only ever
open it for reading, working in temporary directories of their own -- and
this script rechecks that nothing changed afterward as a defensive check,
not because the readers are trusted to need one.

Stdlib only, on purpose: same reasoning as the Phase 0 checkers this one
follows the shape of -- no dependency setup for Cameron to fight. `cargo`
itself is the one external program this file shells out to, unavoidably,
since "run the real Rust readers" is the whole point of exercise mode.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

DEFAULT_CORPUS_DIR = Path("corpus/configs")
FIXTURES_DIR = Path(__file__).resolve().parent / "fixtures"
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
EXERCISE_FIXTURE_DIR = FIXTURES_DIR / "exercise-pass"
TRANSFER_ENV_VAR = "MSC2_PHASE5_TRANSFER_PACKAGE"

# Env vars the two Rust exercise tests (P5.24) read the evidence paths from.
HISTORICAL_CONFIGS_DIR_ENV = "MSC2_HISTORICAL_CONFIGS_DIR"
TRANSFER_PACKAGE_PATH_ENV = "MSC2_TRANSFER_PACKAGE_PATH"

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


def run_cargo_test(package: str, test_name: str, env_overrides: dict[str, str]) -> tuple[int, str]:
    env = os.environ.copy()
    env.update(env_overrides)
    proc = subprocess.run(
        ["cargo", "test", "-p", package, "--test", test_name, "--", "--nocapture"],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


def config_file_stat(config_files: list[Path]) -> dict[Path, tuple[int, float]]:
    return {p: (p.stat().st_size, p.stat().st_mtime) for p in config_files}


def check_exercise(
    configs_dir: Path,
    transfer_package: str | None,
    require_configs: int,
    require_transfer: bool,
) -> str:
    """Raises CheckError on the first failure; returns an "ok" message
    describing what ran otherwise. Runs every P5.2 inventory check first --
    exercise mode never substitutes for them -- then the real Rust readers."""
    check_inventory(configs_dir, transfer_package)

    config_files = config_files_in(configs_dir)
    if len(config_files) < require_configs:
        raise CheckError(
            f"{configs_dir}: found {len(config_files)} config file(s), need at least {require_configs}"
        )

    before = config_file_stat(config_files)
    code, output = run_cargo_test(
        "msc-infrastructure",
        "historical_config_corpus",
        # `cargo test` runs the test binary with its cwd set to the crate
        # directory, not the workspace root -- a relative path here would
        # resolve against the wrong place.
        {HISTORICAL_CONFIGS_DIR_ENV: str(configs_dir.resolve())},
    )
    print(output, end="")
    if code != 0:
        raise CheckError(f"historical_config_corpus exercise test failed (exit {code})")
    after = config_file_stat(config_files)
    if before != after:
        raise CheckError(f"{configs_dir}: corpus config file(s) changed during the exercise run")

    if not require_transfer:
        return f"ok exercise {configs_dir} ({len(config_files)} configs, transfer not exercised)"

    if not transfer_package:
        raise CheckError(f"${TRANSFER_ENV_VAR} is not set")
    transfer_path = Path(transfer_package)
    if not transfer_path.is_file():
        raise CheckError(f"{transfer_path}: ${TRANSFER_ENV_VAR} does not name an existing file")

    before_stat = transfer_path.stat()
    code, output = run_cargo_test(
        "msc-application",
        "real_transfer_corpus",
        {TRANSFER_PACKAGE_PATH_ENV: str(transfer_path.resolve())},
    )
    print(output, end="")
    if code != 0:
        raise CheckError(f"real_transfer_corpus exercise test failed (exit {code})")
    after_stat = transfer_path.stat()
    if (before_stat.st_size, before_stat.st_mtime) != (after_stat.st_size, after_stat.st_mtime):
        raise CheckError(f"{transfer_path}: transfer package changed during the exercise run")

    return f"ok exercise {configs_dir} ({len(config_files)} configs, transfer package verified)"


def run_exercise(
    configs_dir: Path,
    transfer_package: str | None,
    require_configs: int,
    require_transfer: bool,
) -> tuple[int, str]:
    try:
        message = check_exercise(configs_dir, transfer_package, require_configs, require_transfer)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


def exercise_selftest() -> tuple[int, list[str]]:
    candidates = list(EXERCISE_FIXTURE_DIR.glob("*.msctransfer"))
    if not candidates:
        return 1, [f"{EXERCISE_FIXTURE_DIR}: expected a .msctransfer file, found none"]
    transfer_package = str(candidates[0])

    code, message = run_exercise(
        EXERCISE_FIXTURE_DIR,
        transfer_package,
        require_configs=2,
        require_transfer=True,
    )
    ok = code == 0
    return (0 if ok else 1), [f"{'pass' if ok else 'FAIL'} exercise-pass: ({message})"]


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
    parser.add_argument("--exercise", action="store_true", help="run the exercise checks (P5.24)")
    parser.add_argument("--exercise-selftest", action="store_true")
    parser.add_argument("--configs-dir", type=Path, default=None, help="exercise mode's configs directory (default: --corpus-dir)")
    parser.add_argument("--transfer-package", type=str, default=None, help="exercise mode's .msctransfer path (default: $%s)" % TRANSFER_ENV_VAR)
    parser.add_argument("--require-configs", type=int, default=1)
    parser.add_argument("--require-transfer", action="store_true")
    args = parser.parse_args()

    if args.selftest or args.exercise_selftest:
        code = 0
        if args.selftest:
            selftest_code, lines = selftest()
            for line in lines:
                print(line)
            code = code or selftest_code
        if args.exercise_selftest:
            exercise_code, lines = exercise_selftest()
            for line in lines:
                print(line)
            code = code or exercise_code
        return code

    transfer_package = args.transfer_package or os.environ.get(TRANSFER_ENV_VAR)

    if args.exercise:
        configs_dir = args.configs_dir or args.corpus_dir
        code, message = run_exercise(configs_dir, transfer_package, args.require_configs, args.require_transfer)
        print(message)
        return code

    code, message = run_inventory(args.corpus_dir, transfer_package)
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
