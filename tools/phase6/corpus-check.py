#!/usr/bin/env python3
"""Phase 6 world/backup corpus checker (P6.2).

Gates P6.3's evidence collection, and later exercise-mode steps, against
`docs/msc2/worlds/phase6-scope.md`'s evidence requirements: this fails
loudly, not silently, when the real MSC 1 corpus that note demands isn't
actually there yet.

Inventory mode (the only mode this step builds) checks a worlds directory
and a backups directory, each against its own `manifest.json`:

  - Every evidence file (a Java `level.dat`, `world_slots/active_slot_id.txt`,
    a slot's `slot.json`, a slot's `world.zip`, a backup `.zip`, a backup
    `.meta.json`) has a manifest entry recording its source, whether/how it
    was sanitized, and the SHA-256 it was recorded at.
  - The recomputed SHA-256 of every evidence file matches what its manifest
    entry recorded -- an input that changed after being catalogued fails
    loudly instead of silently drifting from what the manifest claims.
  - No two evidence files (worlds or backups, checked together) share a
    SHA-256 -- a duplicate isn't a second sample.
  - Every `slot.json` and every `*.meta.json` parses as JSON.
  - Every `.zip` evidence archive (`world.zip`, backup zips) contains no
    entry with an absolute path or a `..` component -- an archive that could
    write outside its extraction root is not safe evidence to build a
    real-world exercise test on top of later.
  - The worlds directory contains a Java multi-folder world: a `level.dat`
    outside `world_slots/`, with dimension evidence in any of the three real
    layouts P6.3 found MSC 1 actually producing -- classic sibling
    directories (`<name>_nether` / `<name>_the_end`), vanilla/Fabric nested
    folders (`DIM-1` / `DIM1` inside the world folder itself), or current
    PaperMC's nested folders (`dimensions/minecraft/the_nether` /
    `dimensions/minecraft/the_end` inside the world folder). Relaxed from
    sibling-only in P6.3 once real evidence from Cameron's own Fabric and
    current-PaperMC servers proved neither one uses the sibling convention
    `WorldSlotManager.swift`'s original multi-folder assumption was written
    against -- see `corpus/worlds/README.md`.
  - The worlds directory contains a real `world_slots/` tree: a non-empty
    `active_slot_id.txt` marker that names a slot directory which actually
    exists, at least one `slot.json`, and at least one `world.zip` archive.
  - The backups directory contains at least one `.zip`; an adjacent
    `.meta.json` is checked when present but never required.
  - Bedrock evidence (a top-level directory whose name starts with
    `bedrock`) is optional and reported separately when present -- absence
    is not a failure, and this checker never fabricates it.

  corpus-check.py [--worlds DIR] [--backups DIR] --inventory
                                             check worlds/backups corpus
                                             directories (defaults:
                                             corpus/worlds, corpus/backups)
  corpus-check.py --selftest                run inventory mode against the
                                             fixtures in
                                             tools/phase6/fixtures/, proving
                                             the passing case succeeds and
                                             every deliberately-broken case
                                             fails

`corpus/worlds/` and `corpus/backups/` were populated with real MSC 1
evidence by P6.3 (large/private files kept out of git via `.gitignore`,
provenance recorded in `manifest.json`); this checker's own passing and
deliberately-broken self-test cases live under `tools/phase6/fixtures/`
instead, exactly so nothing invented ends up in `corpus/`.

Exercise mode (P6.26) extends this file the way P5.24 extended
`tools/phase5/real-corpus-check.py`. It runs every inventory check above --
never a substitute for them -- hashes every evidence file, then shells out
to `cargo test -p msc-application --test real_world_backup_corpus`, pointed
at the worlds/backups directories via `MSC2_WORLDS_CORPUS_DIR`/
`MSC2_BACKUPS_CORPUS_DIR`. That real Rust test runs the real corpus through
repository load, import reconciliation (against a temporary copy),
archive-safety validation, NBT metadata parsing, a non-destructive backup
restore into a temporary root, and a save/reload round trip -- hashing
every real source file it touches before and after and reporting each one
independently (`--nocapture` output is passed through). This wrapper
re-hashes the same evidence files again afterward as its own independent
defensive check that nothing in `corpus/` moved.

  corpus-check.py --exercise [--worlds DIR] [--backups DIR]
                  [--private-root DIR]
                                             run the exercise checks above
  corpus-check.py --exercise-selftest       (not built by this step --
                                             the real Rust exercise test
                                             above only makes sense to run
                                             against real evidence, so
                                             there is no synthetic
                                             exercise-mode fixture the way
                                             P5.24's is)

`--private-root` is this phase's own plan text's "run the real package/
world/backup through the public Phase 6 smoke where size permits" leg.
P6.35 gave `tools/phase6/phase6-gate-smoke.sh` a `--private-corpus DIR`
mode alongside its existing `--synthetic` one: a smaller, real-data run
of the same public path (bounded server import, world export/import,
activation, backup, restore) against whichever real Java world sorts
first under `DIR`, hashing every real file it touches before and after
and failing loudly if anything changed. `--private-root` here shells out
to exactly that (`check_private_root_smoke`), the same
subprocess-then-check-exit-code shape `check_exercise` already uses for
the real Rust corpus test -- when a private root is supplied, the public
leg genuinely runs; when it isn't, this only reports that plainly rather
than silently declaring it done.

Stdlib only, on purpose: same reasoning as `tools/phase5/real-corpus-check.py`
and the Phase 0 checkers both follow the shape of -- no dependency setup for
Cameron to fight.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import zipfile
from pathlib import Path

DEFAULT_WORLDS_DIR = Path("corpus/worlds")
DEFAULT_BACKUPS_DIR = Path("corpus/backups")
FIXTURES_DIR = Path(__file__).resolve().parent / "fixtures"
REPO_ROOT = Path(__file__).resolve().parent.parent.parent

# Env vars the real Rust exercise test (P6.26,
# crates/msc-application/tests/real_world_backup_corpus.rs) reads the
# evidence directories from.
WORLDS_CORPUS_DIR_ENV = "MSC2_WORLDS_CORPUS_DIR"
BACKUPS_CORPUS_DIR_ENV = "MSC2_BACKUPS_CORPUS_DIR"
PRIVATE_ROOT_ENV = "MSC2_PHASE6_PRIVATE_CORPUS"
EXERCISE_TEST_NAME = "real_world_backup_corpus"

# (fixture directory name, expected inventory exit code)
SELFTEST_CASES = [
    ("pass", 0),
    ("missing-provenance", 1),
    ("duplicate-hash", 1),
    ("malformed-metadata", 1),
    ("unsafe-archive", 1),
    ("mutated-input", 1),
    ("no-dimension-evidence", 1),
]


class CheckError(Exception):
    pass


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def is_bedrock_path(rel: Path) -> bool:
    return bool(rel.parts) and rel.parts[0].startswith("bedrock")


def load_manifest(dir_path: Path) -> dict[str, dict]:
    manifest_path = dir_path / "manifest.json"
    if not manifest_path.is_file():
        raise CheckError(f"{manifest_path}: missing provenance manifest")
    try:
        manifest = json.loads(manifest_path.read_text())
    except json.JSONDecodeError as exc:
        raise CheckError(f"{manifest_path}: malformed JSON manifest ({exc})")
    if not isinstance(manifest.get("files"), list) or not manifest["files"]:
        raise CheckError(f"{manifest_path}: manifest has no 'files' entries")

    by_path: dict[str, dict] = {}
    for entry in manifest["files"]:
        path = entry.get("path")
        if not path:
            raise CheckError(f"{manifest_path}: entry missing 'path'")
        for field in ("source", "sanitized", "sha256"):
            if not entry.get(field):
                raise CheckError(f"{manifest_path}: {path} missing '{field}'")
        by_path[path] = entry
    return by_path


def requires_provenance(rel_path: Path, root_name: str) -> bool:
    """Which evidence files must be catalogued in manifest.json. Structural
    filler (e.g. a `.gitkeep` proving a dimension directory exists, a
    bedrock `db/` file) doesn't need its own provenance entry -- only the
    files phase6-scope.md actually treats as evidence."""
    name = rel_path.name
    if root_name == "worlds":
        if is_bedrock_path(rel_path):
            return False
        if name == "level.dat":
            return True
        if "world_slots" in rel_path.parts and name in (
            "active_slot_id.txt",
            "slot.json",
            "world.zip",
        ):
            return True
        return False
    if root_name == "backups":
        return rel_path.suffix == ".zip" or name.endswith(".meta.json")
    return False


def zip_is_safe(path: Path) -> None:
    try:
        with zipfile.ZipFile(path) as zf:
            for name in zf.namelist():
                member = Path(name)
                if member.is_absolute() or ".." in member.parts:
                    raise CheckError(f"{path}: unsafe archive entry {name!r}")
    except zipfile.BadZipFile as exc:
        raise CheckError(f"{path}: not a valid zip archive ({exc})")


def check_provenance_and_hashes(dir_path: Path, root_name: str, seen_hashes: dict[str, str]) -> None:
    manifest = load_manifest(dir_path)

    for file_path in sorted(dir_path.rglob("*")):
        if file_path.is_dir() or file_path.name == "manifest.json":
            continue
        rel = file_path.relative_to(dir_path)
        if not requires_provenance(rel, root_name):
            continue
        rel_str = str(rel)

        entry = manifest.get(rel_str)
        if entry is None:
            raise CheckError(f"{file_path}: no manifest entry recording its provenance")

        actual_hash = sha256_of(file_path)
        if actual_hash != entry["sha256"]:
            raise CheckError(
                f"{file_path}: SHA-256 does not match manifest "
                f"({actual_hash} != {entry['sha256']}) -- input mutated after being recorded"
            )
        if actual_hash in seen_hashes:
            raise CheckError(
                f"{file_path}: identical SHA-256 to {seen_hashes[actual_hash]} -- "
                "a duplicate isn't a second sample"
            )
        seen_hashes[actual_hash] = str(file_path)

        if file_path.suffix == ".zip":
            zip_is_safe(file_path)
        if file_path.name == "slot.json" or file_path.name.endswith(".meta.json"):
            try:
                json.loads(file_path.read_text())
            except json.JSONDecodeError as exc:
                raise CheckError(f"{file_path}: malformed JSON ({exc})")


def _dimension_evidence_shape(base_dir: Path) -> str | None:
    """Which of the three real Java multi-folder layouts P6.3 found MSC 1
    actually producing, if any, is present next to/inside `base_dir`.
    Returns a short shape label for the summary message, or None if the
    world shows no dimension evidence in any of them.

    Originally this only accepted the classic sibling-folder shape
    (`WorldSlotManager.swift`'s own multi-folder assumption). P6.3's real
    corpus proved that assumption doesn't match either of Cameron's two real
    servers: a Fabric server (vanilla's own on-disk format, DIM-1/DIM1
    nested inside the world folder -- structurally can never produce sibling
    folders) or a current PaperMC server (which nests dimensions under
    `dimensions/minecraft/...` instead of splitting into sibling folders).
    Both are just as real as the classic shape, so all three are accepted."""
    base_name = base_dir.name
    if (base_dir.parent / f"{base_name}_nether").is_dir() or (base_dir.parent / f"{base_name}_the_end").is_dir():
        return "sibling <name>_nether/<name>_the_end"
    if (base_dir / "DIM-1").is_dir() or (base_dir / "DIM1").is_dir():
        return "nested vanilla/Fabric DIM-1/DIM1"
    nested_paper = base_dir / "dimensions" / "minecraft"
    if (nested_paper / "the_nether").is_dir() or (nested_paper / "the_end").is_dir():
        return "nested PaperMC dimensions/minecraft/the_nether|the_end"
    return None


def check_worlds_structure(dir_path: Path) -> str:
    level_dats = sorted(
        p
        for p in dir_path.rglob("level.dat")
        if not is_bedrock_path(p.relative_to(dir_path))
        and "world_slots" not in p.relative_to(dir_path).parts
    )
    if not level_dats:
        raise CheckError(f"{dir_path}: no Java level.dat found (outside world_slots/, outside bedrock*/)")
    base_dir = level_dats[0].parent
    base_name = base_dir.name
    dimension_shape = _dimension_evidence_shape(base_dir)
    if dimension_shape is None:
        raise CheckError(
            f"{base_dir}: no dimension evidence found (sibling <name>_nether/<name>_the_end, "
            "nested DIM-1/DIM1, or nested dimensions/minecraft/the_nether|the_end) -- "
            "not a Java multi-folder world"
        )

    world_slots_dir = dir_path / "world_slots"
    if not world_slots_dir.is_dir():
        raise CheckError(f"{world_slots_dir}: missing")

    marker = world_slots_dir / "active_slot_id.txt"
    if not marker.is_file() or not marker.read_text().strip():
        raise CheckError(f"{marker}: missing or empty active-slot marker")
    active_id = marker.read_text().strip()
    if not (world_slots_dir / active_id).is_dir():
        raise CheckError(f"{marker}: names slot {active_id!r}, which has no directory under {world_slots_dir}")

    slot_dirs = [p for p in world_slots_dir.iterdir() if p.is_dir()]
    if not slot_dirs:
        raise CheckError(f"{world_slots_dir}: no slot directories")
    if not any((p / "slot.json").is_file() for p in slot_dirs):
        raise CheckError(f"{world_slots_dir}: no slot has slot.json metadata")
    if not any((p / "world.zip").is_file() for p in slot_dirs):
        raise CheckError(f"{world_slots_dir}: no slot has a world.zip archive")

    bedrock_dirs = sorted(p for p in dir_path.iterdir() if p.is_dir() and p.name.startswith("bedrock"))
    if bedrock_dirs:
        for bedrock_dir in bedrock_dirs:
            if not list(bedrock_dir.rglob("level.dat")):
                raise CheckError(f"{bedrock_dir}: bedrock evidence present but has no level.dat")
        bedrock_note = f"bedrock: present ({', '.join(p.name for p in bedrock_dirs)})"
    else:
        bedrock_note = "bedrock: not provided (optional)"

    return (
        f"java multi-folder world ok ({base_name}, {dimension_shape}), "
        f"world_slots ok ({len(slot_dirs)} slot(s), active={active_id}), {bedrock_note}"
    )


def check_backups_structure(dir_path: Path) -> str:
    zips = sorted(dir_path.glob("*.zip"))
    if not zips:
        raise CheckError(f"{dir_path}: no backup .zip found")
    with_meta = sum(1 for z in zips if (dir_path / f"{z.stem}.meta.json").is_file())
    return f"{len(zips)} backup zip(s), {with_meta} with adjacent .meta.json"


def check_inventory(worlds_dir: Path, backups_dir: Path) -> str:
    """Raises CheckError on the first evidence gap found; returns an "ok"
    message describing what passed otherwise."""
    if not worlds_dir.is_dir():
        raise CheckError(f"{worlds_dir}: worlds corpus directory does not exist")
    if not backups_dir.is_dir():
        raise CheckError(f"{backups_dir}: backups corpus directory does not exist")

    seen_hashes: dict[str, str] = {}
    check_provenance_and_hashes(worlds_dir, "worlds", seen_hashes)
    check_provenance_and_hashes(backups_dir, "backups", seen_hashes)

    worlds_summary = check_worlds_structure(worlds_dir)
    backups_summary = check_backups_structure(backups_dir)

    return f"ok {worlds_dir} ({worlds_summary}); ok {backups_dir} ({backups_summary})"


def run_inventory(worlds_dir: Path, backups_dir: Path) -> tuple[int, str]:
    try:
        message = check_inventory(worlds_dir, backups_dir)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


def evidence_file_hashes(dir_path: Path, root_name: str) -> dict[str, str]:
    """The same evidence files `check_provenance_and_hashes` requires a
    manifest entry for, hashed again -- exercise mode's own before/after
    snapshot, independent of the real Rust test's own per-file hashing."""
    hashes: dict[str, str] = {}
    for file_path in sorted(dir_path.rglob("*")):
        if file_path.is_dir() or file_path.name == "manifest.json":
            continue
        rel = file_path.relative_to(dir_path)
        if not requires_provenance(rel, root_name):
            continue
        hashes[str(file_path)] = sha256_of(file_path)
    return hashes


def run_cargo_test(test_name: str, env_overrides: dict[str, str]) -> tuple[int, str]:
    env = os.environ.copy()
    env.update(env_overrides)
    proc = subprocess.run(
        ["cargo", "test", "-p", "msc-application", "--test", test_name, "--", "--nocapture"],
        cwd=REPO_ROOT,
        env=env,
        capture_output=True,
        text=True,
    )
    return proc.returncode, proc.stdout + proc.stderr


GATE_SMOKE_SCRIPT = REPO_ROOT / "tools" / "phase6" / "phase6-gate-smoke.sh"


def check_private_root_smoke(private_root: str | None) -> str:
    """This phase's own plan text's "run the real package/world/backup
    through the public Phase 6 smoke where size permits" leg. When a
    private root is supplied, this actually runs
    `phase6-gate-smoke.sh --private-corpus <root>` (P6.35) -- the real
    agent driven, over nothing but its own CLI/HTTP surface, through a
    bounded server import using the copied server's real configured world
    folder name, a bounded staged-upload world export/import round trip,
    activation, a manual backup, and a restore, all against whichever real
    Java world sorts first under `root`. That script does
    its own before/after hashing of the real source files it touches and
    fails loudly (nonzero exit) if anything changed or the run didn't
    happen -- this wrapper only needs to check the exit code, the same
    subprocess-then-check-exit-code shape `check_exercise` already uses
    for the real Rust corpus test. Absent `--private-root` this still
    reports plainly that the public leg wasn't exercised, rather than
    silently skipping."""
    if not private_root:
        return "public smoke not exercised (no --private-root supplied)"
    root = Path(private_root)
    if not root.is_dir():
        raise CheckError(f"{root}: --private-root does not name an existing directory")

    proc = subprocess.run(
        [str(GATE_SMOKE_SCRIPT), "--private-corpus", str(root)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    print(proc.stdout, end="")
    print(proc.stderr, end="", file=sys.stderr)
    if proc.returncode != 0:
        raise CheckError(
            f"phase6-gate-smoke.sh --private-corpus {root} failed (exit {proc.returncode})"
        )

    return f"ok public smoke against real private corpus {root} (phase6-gate-smoke.sh --private-corpus)"


def check_exercise(worlds_dir: Path, backups_dir: Path, private_root: str | None) -> str:
    """Raises CheckError on the first failure; returns an "ok" message
    describing what ran otherwise. Runs every inventory check first --
    exercise mode never substitutes for them -- then the real Rust reader
    (`real_world_backup_corpus.rs`, P6.26)."""
    check_inventory(worlds_dir, backups_dir)

    before = evidence_file_hashes(worlds_dir, "worlds")
    before.update(evidence_file_hashes(backups_dir, "backups"))

    code, output = run_cargo_test(
        EXERCISE_TEST_NAME,
        {
            WORLDS_CORPUS_DIR_ENV: str(worlds_dir.resolve()),
            BACKUPS_CORPUS_DIR_ENV: str(backups_dir.resolve()),
        },
    )
    print(output, end="")
    if code != 0:
        raise CheckError(f"{EXERCISE_TEST_NAME} exercise test failed (exit {code})")

    after = evidence_file_hashes(worlds_dir, "worlds")
    after.update(evidence_file_hashes(backups_dir, "backups"))
    if before != after:
        raise CheckError(f"{worlds_dir}/{backups_dir}: corpus evidence changed during the exercise run")

    smoke_note = check_private_root_smoke(private_root)

    return f"ok exercise {worlds_dir} + {backups_dir} ({len(before)} evidence files unchanged); {smoke_note}"


def run_exercise(worlds_dir: Path, backups_dir: Path, private_root: str | None) -> tuple[int, str]:
    try:
        message = check_exercise(worlds_dir, backups_dir, private_root)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


def selftest() -> tuple[int, list[str]]:
    lines = []
    all_ok = True
    for name, expected_code in SELFTEST_CASES:
        fixture_dir = FIXTURES_DIR / name
        code, message = run_inventory(fixture_dir / "worlds", fixture_dir / "backups")
        ok = code == expected_code
        all_ok = all_ok and ok
        lines.append(f"{'pass' if ok else 'FAIL'} {name}: expected={expected_code} got={code} ({message})")
    return (0 if all_ok else 1), lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--worlds", type=Path, default=DEFAULT_WORLDS_DIR)
    parser.add_argument("--backups", type=Path, default=DEFAULT_BACKUPS_DIR)
    parser.add_argument("--inventory", action="store_true")
    parser.add_argument("--selftest", action="store_true")
    parser.add_argument("--exercise", action="store_true", help="run the exercise checks (P6.26)")
    parser.add_argument(
        "--private-root",
        type=str,
        default=None,
        help="optional larger private real corpus root for the public Phase 6 smoke leg "
        "(default: $%s)" % PRIVATE_ROOT_ENV,
    )
    args = parser.parse_args()

    if args.selftest:
        code, lines = selftest()
        for line in lines:
            print(line)
        return code

    if args.exercise:
        private_root = args.private_root or os.environ.get(PRIVATE_ROOT_ENV)
        code, message = run_exercise(args.worlds, args.backups, private_root)
        print(message)
        return code

    code, message = run_inventory(args.worlds, args.backups)
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
