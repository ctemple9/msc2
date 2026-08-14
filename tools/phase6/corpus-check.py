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
    outside `world_slots/`, with at least one dimension sibling directory
    (`<name>_nether` or `<name>_the_end`) next to it.
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

Exercise mode (P6.26) will extend this file the way P5.24 extended
`tools/phase5/real-corpus-check.py`; this step only builds the inventory
gate that later mode will plug into. `corpus/worlds/` and `corpus/backups/`
stay empty (beyond their READMEs) until P6.3 supplies real MSC 1 evidence --
the fixtures this step ships live under `tools/phase6/fixtures/` instead,
exactly so nothing invented ends up in `corpus/`.

Stdlib only, on purpose: same reasoning as `tools/phase5/real-corpus-check.py`
and the Phase 0 checkers both follow the shape of -- no dependency setup for
Cameron to fight.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import zipfile
from pathlib import Path

DEFAULT_WORLDS_DIR = Path("corpus/worlds")
DEFAULT_BACKUPS_DIR = Path("corpus/backups")
FIXTURES_DIR = Path(__file__).resolve().parent / "fixtures"

# (fixture directory name, expected inventory exit code)
SELFTEST_CASES = [
    ("pass", 0),
    ("missing-provenance", 1),
    ("duplicate-hash", 1),
    ("malformed-metadata", 1),
    ("unsafe-archive", 1),
    ("mutated-input", 1),
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
    nether_dir = base_dir.parent / f"{base_name}_nether"
    end_dir = base_dir.parent / f"{base_name}_the_end"
    if not nether_dir.is_dir() and not end_dir.is_dir():
        raise CheckError(
            f"{base_dir}: no dimension sibling directory ({nether_dir.name} or {end_dir.name}) -- "
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
        f"java multi-folder world ok ({base_name}), "
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
    args = parser.parse_args()

    if args.selftest:
        code, lines = selftest()
        for line in lines:
            print(line)
        return code

    code, message = run_inventory(args.worlds, args.backups)
    print(message)
    return code


if __name__ == "__main__":
    sys.exit(main())
