#!/usr/bin/env python3
"""Phase 8 add-on/modpack corpus and gate checker (P8.2).

Built before any real evidence is collected, so the bar for `corpus/addons/`
and `corpus/packs/` is set before it can be bent to fit whatever P8.3 turns
up -- the same ordering `tools/phase7/provider-corpus-check.py` and
`tools/phase6/corpus-check.py` used for their own corpora. Three
independent modes:

Inventory mode checks an add-on evidence corpus directory (default
`corpus/addons/`) against its own `manifest.json`:

  - Every recorded evidence file (anything under the directory except
    `manifest.json`, `README.md`, and dotfiles) has a manifest entry
    recording: which of the five providers this phase's create/update flow
    talks to (`provider`), what it was captured for (`purpose`), where it
    came from (`source_url`), when it was captured (`captured`), its
    SHA-256 (`sha256`), and its byte size (`byte_size`). A missing field
    fails loudly.
  - `provider` must be one of the five `docs/msc2/addons/phase8-scope.md`'s
    "Provider purposes" table names (`modrinth`, `hangar`, `curseforge`,
    `github`, `direct`) -- a typo or a provider outside this phase's scope
    fails loudly rather than silently miscounting coverage later.
  - No two evidence files share a SHA-256 -- a duplicate isn't a second
    sample.
  - Every `.json` file parses as JSON.
  - Every `.zip`/`.mrpack`/`.jar` evidence file (an evidence sample that
    happens to itself be an archive, e.g. a captured author-blocked
    CurseForge file) is a valid zip with no entry carrying an absolute path
    or a `..` component -- archive evidence must be safe to keep around,
    not just present.
  - The recomputed SHA-256 of every evidence file matches what its manifest
    entry recorded -- an input that changed after being catalogued fails
    loudly instead of silently drifting from what the manifest claims
    (this is this mode's "archive immutability" check: the catalogued
    bytes, archive or not, may not move).

Pack mode checks a modpack archive corpus directory (default
`corpus/packs/`) against its own `manifest.json`:

  - Every recorded pack archive has a manifest entry recording `source_url`,
    `captured`, `sha256`, `byte_size`, and `pack_format` (`mrpack` or
    `curseforge`).
  - The recomputed SHA-256 matches the manifest, and no two archives share
    one -- the same provenance/mutation/duplicate checks inventory mode
    runs.
  - Every archive is a valid zip with no entry carrying an absolute path or
    a `..` component -- nothing in it could extract outside a bounded root.
    Nothing is ever actually extracted to disk by this checker; every
    manifest and override-root check below reads member bytes/names
    in-memory via `zipfile`, which is what "without extracting outside a
    temporary root" means in practice -- there is no root to escape,
    because there is no extraction.
  - An `mrpack`-format archive contains a genuine `modrinth.index.json` at
    its root, parseable as JSON with non-empty `game`, `versionId`, `name`,
    and `dependencies` fields, and every other entry in the archive falls
    under one of the three known override roots (`overrides/`,
    `client-overrides/`, `server-overrides/`) -- an mrpack that smuggles a
    stray top-level entry isn't a genuine pack shape.
  - A `curseforge`-format archive contains a genuine `manifest.json` at its
    root, parseable as JSON with `manifestType == "minecraftModpack"` and
    non-empty `minecraft` (with `version`/`modLoaders`), `name`, `version`,
    and `overrides` fields, and the folder `overrides` names actually
    appears as an entry prefix in the archive -- a manifest that references
    an override root the archive doesn't actually contain isn't genuine
    either.

Fixture-coverage mode takes a fixture directory (e.g.
`fixtures/add-on-providers/`, built from P8.4 onward) and checks it against
an add-on corpus:

  - Every fixture `*.json` file may carry an optional top-level
    `corpus_source` field -- a list of paths, relative to the add-on
    corpus root, naming which recorded response(s) that case was
    characterized from. This is additive to the six required fields
    `docs/msc2/fixture-format.md` already defines; existing tooling ignores
    unknown fields, so this doesn't touch that spec.
  - Every path a fixture cites must actually have a manifest entry in the
    add-on corpus -- a fixture cannot claim a response that was never
    recorded.
  - A fixture may also carry a top-level `workflow` field naming which of
    Phase 8's eight symbol-ledger domains (`docs/msc2/addons/
    phase8-scope.md`'s "Symbol-ledger rows owned by this phase" table:
    `addon-updates`, `modpack-client-only`, `modpack-import`, `modpacks`,
    `modrinth-deps`, `mods`, `plugin-management`, `plugins`) it
    characterizes.
  - Across every fixture in the directory: all five providers must be cited
    by at least one fixture's `corpus_source`, and all eight workflows must
    be named by at least one fixture's `workflow` -- a fixture directory
    that never exercises (say) Hangar, or never characterizes
    `modrinth-deps`, isn't complete coverage of what P8.1 scoped, even if
    every citation it does have is real.

Passing and deliberately failing self-tests (`--selftest`) prove each
rejection fires, for all three modes, against small synthetic fixtures
under `tools/phase8/fixtures/` -- never against `corpus/addons/` or
`corpus/packs/` themselves, so nothing invented ends up standing in for the
real thing. No network access anywhere in this tool.

  phase8-check.py --inventory [DIR]          check an add-on evidence corpus
                                              directory (default:
                                              corpus/addons)
  phase8-check.py --packs [DIR]              check a modpack archive corpus
                                              directory (default:
                                              corpus/packs)
  phase8-check.py --coverage FIXTURE_DIR [--inventory DIR]
                                              check a fixture directory's
                                              citations against an add-on
                                              corpus
  phase8-check.py --selftest                 run all three modes against the
                                              fixtures in
                                              tools/phase8/fixtures/,
                                              proving the passing case
                                              succeeds and every
                                              deliberately-broken case fails

`--inventory` and `--packs` may be combined in one invocation (each runs
independently; the command fails if either does) -- P8.3's own Verify line
does exactly this.

Stdlib only, on purpose: same reasoning as `tools/phase7/provider-corpus-check.py`
and `tools/phase6/corpus-check.py` -- no dependency setup for Cameron to fight.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import zipfile
from pathlib import Path

DEFAULT_ADDONS_DIR = Path("corpus/addons")
DEFAULT_PACKS_DIR = Path("corpus/packs")
FIXTURES_DIR = Path(__file__).resolve().parent / "fixtures"

# The five provider purposes `docs/msc2/addons/phase8-scope.md`'s "Provider
# purposes" table names -- Modrinth, Hangar, CurseForge, GitHub Releases,
# and the direct-URL fallback (not a real provider, but its own dispatch
# case in `fetchOnlineVersion`).
PROVIDERS = ("modrinth", "hangar", "curseforge", "github", "direct")

# The eight symbol-ledger `target_domain` values `docs/msc2/addons/
# phase8-scope.md`'s "Symbol-ledger rows owned by this phase" table groups
# Phase 8's 35 owned rows under.
WORKFLOWS = (
    "addon-updates",
    "modpack-client-only",
    "modpack-import",
    "modpacks",
    "modrinth-deps",
    "mods",
    "plugin-management",
    "plugins",
)

PACK_FORMATS = ("mrpack", "curseforge")

REQUIRED_ENTRY_FIELDS = ("provider", "purpose", "source_url", "captured", "sha256", "byte_size")
REQUIRED_PACK_FIELDS = ("source_url", "captured", "sha256", "byte_size", "pack_format")

ARCHIVE_SUFFIXES = {".zip", ".mrpack", ".jar"}

IGNORED_NAMES = {"manifest.json", "README.md"}

MRPACK_MANIFEST_NAME = "modrinth.index.json"
CURSEFORGE_MANIFEST_NAME = "manifest.json"
MRPACK_OVERRIDE_ROOTS = ("overrides/", "client-overrides/", "server-overrides/")

# (mode, fixture case directory name, expected exit code)
SELFTEST_CASES = [
    ("inventory", "pass", 0),
    ("inventory", "missing-provenance", 1),
    ("inventory", "duplicate-hash", 1),
    ("inventory", "malformed-json", 1),
    ("inventory", "mutated-input", 1),
    ("inventory", "unknown-provider", 1),
    ("packs", "pack-pass", 0),
    ("packs", "pack-unsafe-path", 1),
    ("packs", "pack-malformed-archive", 1),
    ("packs", "pack-missing-manifest", 1),
    ("coverage", "coverage-pass", 0),
    ("coverage", "coverage-missing-provider", 1),
    ("coverage", "coverage-missing-workflow", 1),
    ("coverage", "coverage-dangling-citation", 1),
]


class CheckError(Exception):
    pass


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def is_ignored(path: Path) -> bool:
    return path.name in IGNORED_NAMES or path.name.startswith(".")


def load_manifest(dir_path: Path, required_fields: tuple[str, ...]) -> dict[str, dict]:
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
        for field in required_fields:
            value = entry.get(field)
            if value is None or value == "":
                raise CheckError(f"{manifest_path}: {path} missing '{field}'")
        by_path[path] = entry
    return by_path


def zip_is_safe(path: Path) -> zipfile.ZipFile:
    """Opens the archive and checks every entry is confined to a relative,
    non-parent-traversing path. Returns the open ZipFile so callers can
    read manifest/override-root shape from it without a second open --
    nothing is ever extracted to disk."""
    try:
        zf = zipfile.ZipFile(path)
    except zipfile.BadZipFile as exc:
        raise CheckError(f"{path}: not a valid zip archive ({exc})")
    for name in zf.namelist():
        member = Path(name)
        if member.is_absolute() or ".." in member.parts:
            raise CheckError(f"{path}: unsafe archive entry {name!r}")
    return zf


# ---------------------------------------------------------------------------
# Inventory mode
# ---------------------------------------------------------------------------


def check_inventory(addons_dir: Path) -> str:
    """Raises CheckError on the first evidence gap found; returns an "ok"
    message describing what passed otherwise."""
    if not addons_dir.is_dir():
        raise CheckError(f"{addons_dir}: add-on corpus directory does not exist")

    manifest = load_manifest(addons_dir, REQUIRED_ENTRY_FIELDS)

    seen_hashes: dict[str, str] = {}
    providers_present: set[str] = set()
    file_count = 0

    for file_path in sorted(addons_dir.rglob("*")):
        if file_path.is_dir() or is_ignored(file_path):
            continue
        rel_str = str(file_path.relative_to(addons_dir))

        entry = manifest.get(rel_str)
        if entry is None:
            raise CheckError(f"{file_path}: no manifest entry recording its provenance")

        if entry["provider"] not in PROVIDERS:
            raise CheckError(
                f"{file_path}: has unknown provider {entry['provider']!r} "
                f"(expected one of {', '.join(PROVIDERS)})"
            )

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

        if file_path.suffix == ".json":
            try:
                json.loads(file_path.read_text())
            except json.JSONDecodeError as exc:
                raise CheckError(f"{file_path}: malformed JSON ({exc})")
        elif file_path.suffix in ARCHIVE_SUFFIXES:
            zip_is_safe(file_path).close()

        providers_present.add(entry["provider"])
        file_count += 1

    unrecorded = set(manifest) - {
        str(p.relative_to(addons_dir)) for p in addons_dir.rglob("*") if not p.is_dir()
    }
    if unrecorded:
        raise CheckError(f"{addons_dir}: manifest cites path(s) with no file on disk: {sorted(unrecorded)}")

    return (
        f"ok {addons_dir} ({file_count} evidence file(s), "
        f"providers present: {', '.join(sorted(providers_present)) or 'none'})"
    )


def run_inventory(addons_dir: Path) -> tuple[int, str]:
    try:
        message = check_inventory(addons_dir)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


# ---------------------------------------------------------------------------
# Pack mode
# ---------------------------------------------------------------------------


def check_mrpack_shape(path: Path, zf: zipfile.ZipFile) -> None:
    names = zf.namelist()
    if MRPACK_MANIFEST_NAME not in names:
        raise CheckError(f"{path}: no {MRPACK_MANIFEST_NAME} at archive root -- not a genuine mrpack")
    try:
        index = json.loads(zf.read(MRPACK_MANIFEST_NAME))
    except json.JSONDecodeError as exc:
        raise CheckError(f"{path}: {MRPACK_MANIFEST_NAME} is malformed JSON ({exc})")
    for field in ("game", "versionId", "name", "dependencies"):
        if not index.get(field):
            raise CheckError(f"{path}: {MRPACK_MANIFEST_NAME} missing '{field}'")

    for name in names:
        if name == MRPACK_MANIFEST_NAME:
            continue
        if not name.startswith(MRPACK_OVERRIDE_ROOTS):
            raise CheckError(
                f"{path}: entry {name!r} is outside the manifest and the three known "
                f"override roots ({', '.join(MRPACK_OVERRIDE_ROOTS)})"
            )


def check_curseforge_shape(path: Path, zf: zipfile.ZipFile) -> None:
    names = zf.namelist()
    if CURSEFORGE_MANIFEST_NAME not in names:
        raise CheckError(f"{path}: no {CURSEFORGE_MANIFEST_NAME} at archive root -- not a genuine CurseForge pack")
    try:
        manifest = json.loads(zf.read(CURSEFORGE_MANIFEST_NAME))
    except json.JSONDecodeError as exc:
        raise CheckError(f"{path}: {CURSEFORGE_MANIFEST_NAME} is malformed JSON ({exc})")
    if manifest.get("manifestType") != "minecraftModpack":
        raise CheckError(f"{path}: manifest.json manifestType is not 'minecraftModpack'")
    for field in ("minecraft", "name", "version", "overrides"):
        if not manifest.get(field):
            raise CheckError(f"{path}: manifest.json missing '{field}'")
    minecraft = manifest["minecraft"]
    if not isinstance(minecraft, dict) or not minecraft.get("version") or not minecraft.get("modLoaders"):
        raise CheckError(f"{path}: manifest.json 'minecraft' missing 'version'/'modLoaders'")

    override_root = manifest["overrides"].rstrip("/") + "/"
    if not any(name.startswith(override_root) for name in names):
        raise CheckError(
            f"{path}: manifest.json names override root {manifest['overrides']!r}, "
            "but no archive entry falls under it"
        )


def check_packs(packs_dir: Path) -> str:
    """Raises CheckError on the first evidence gap found; returns an "ok"
    message describing what passed otherwise."""
    if not packs_dir.is_dir():
        raise CheckError(f"{packs_dir}: pack corpus directory does not exist")

    manifest = load_manifest(packs_dir, REQUIRED_PACK_FIELDS)

    seen_hashes: dict[str, str] = {}
    formats_present: set[str] = set()
    file_count = 0

    for file_path in sorted(packs_dir.rglob("*")):
        if file_path.is_dir() or is_ignored(file_path):
            continue
        rel_str = str(file_path.relative_to(packs_dir))

        entry = manifest.get(rel_str)
        if entry is None:
            raise CheckError(f"{file_path}: no manifest entry recording its provenance")

        pack_format = entry["pack_format"]
        if pack_format not in PACK_FORMATS:
            raise CheckError(
                f"{file_path}: has unknown pack_format {pack_format!r} "
                f"(expected one of {', '.join(PACK_FORMATS)})"
            )

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

        zf = zip_is_safe(file_path)
        try:
            if pack_format == "mrpack":
                check_mrpack_shape(file_path, zf)
            else:
                check_curseforge_shape(file_path, zf)
        finally:
            zf.close()

        formats_present.add(pack_format)
        file_count += 1

    unrecorded = set(manifest) - {
        str(p.relative_to(packs_dir)) for p in packs_dir.rglob("*") if not p.is_dir()
    }
    if unrecorded:
        raise CheckError(f"{packs_dir}: manifest cites path(s) with no file on disk: {sorted(unrecorded)}")

    return (
        f"ok {packs_dir} ({file_count} pack archive(s), "
        f"formats present: {', '.join(sorted(formats_present)) or 'none'})"
    )


def run_packs(packs_dir: Path) -> tuple[int, str]:
    try:
        message = check_packs(packs_dir)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


# ---------------------------------------------------------------------------
# Fixture-coverage mode
# ---------------------------------------------------------------------------


def check_coverage(fixture_dir: Path, addons_dir: Path) -> str:
    """Raises CheckError on the first gap found; returns an "ok" message
    describing what passed otherwise. Trusts the add-on corpus is already
    sound -- run inventory mode on it separately; this only checks
    citations against whatever manifest is there."""
    if not fixture_dir.is_dir():
        raise CheckError(f"{fixture_dir}: fixture directory does not exist")
    manifest = load_manifest(addons_dir, REQUIRED_ENTRY_FIELDS)

    providers_cited: set[str] = set()
    workflows_named: set[str] = set()
    citation_count = 0

    for fixture_path in sorted(fixture_dir.rglob("*.json")):
        try:
            data = json.loads(fixture_path.read_text())
        except json.JSONDecodeError as exc:
            raise CheckError(f"{fixture_path}: malformed JSON ({exc})")
        if not isinstance(data, dict):
            continue

        workflow = data.get("workflow")
        if workflow is not None:
            if workflow not in WORKFLOWS:
                raise CheckError(
                    f"{fixture_path}: 'workflow' is {workflow!r}, expected one of {', '.join(WORKFLOWS)}"
                )
            workflows_named.add(workflow)

        citations = data.get("corpus_source")
        if citations is None:
            continue
        if not isinstance(citations, list) or not all(isinstance(c, str) for c in citations):
            raise CheckError(f"{fixture_path}: 'corpus_source' must be a list of strings")

        for cited_path in citations:
            entry = manifest.get(cited_path)
            if entry is None:
                raise CheckError(
                    f"{fixture_path}: cites {cited_path!r}, which has no manifest "
                    f"entry in {addons_dir} -- a fixture cannot claim a response "
                    "that was never recorded"
                )
            providers_cited.add(entry["provider"])
            citation_count += 1

    missing_providers = sorted(set(PROVIDERS) - providers_cited)
    if missing_providers:
        raise CheckError(
            f"{fixture_dir}: no fixture cites a corpus response for provider(s) "
            f"{', '.join(missing_providers)} -- coverage of the five-provider "
            "boundary is incomplete"
        )

    missing_workflows = sorted(set(WORKFLOWS) - workflows_named)
    if missing_workflows:
        raise CheckError(
            f"{fixture_dir}: no fixture names workflow(s) {', '.join(missing_workflows)} -- "
            "coverage of P8.1's eight owned symbol-ledger domains is incomplete"
        )

    return (
        f"ok {fixture_dir} ({citation_count} citation(s), all five providers "
        "and all eight workflows represented)"
    )


def run_coverage(fixture_dir: Path, addons_dir: Path) -> tuple[int, str]:
    try:
        message = check_coverage(fixture_dir, addons_dir)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


# ---------------------------------------------------------------------------
# Self-test and CLI
# ---------------------------------------------------------------------------


def selftest() -> tuple[int, list[str]]:
    lines = []
    all_ok = True
    for mode, name, expected_code in SELFTEST_CASES:
        case_dir = FIXTURES_DIR / name
        if mode == "inventory":
            code, message = run_inventory(case_dir / "addons")
        elif mode == "packs":
            code, message = run_packs(case_dir / "packs")
        else:
            code, message = run_coverage(case_dir / "fixtures", case_dir / "addons")
        ok = code == expected_code
        all_ok = all_ok and ok
        lines.append(f"{'pass' if ok else 'FAIL'} {mode}/{name}: expected={expected_code} got={code} ({message})")
    return (0 if all_ok else 1), lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument(
        "--inventory",
        type=Path,
        nargs="?",
        const=DEFAULT_ADDONS_DIR,
        default=None,
        metavar="DIR",
        help="check an add-on evidence corpus directory (default: corpus/addons)",
    )
    parser.add_argument(
        "--packs",
        type=Path,
        nargs="?",
        const=DEFAULT_PACKS_DIR,
        default=None,
        metavar="DIR",
        help="check a modpack archive corpus directory (default: corpus/packs)",
    )
    parser.add_argument("--coverage", type=Path, default=None, metavar="FIXTURE_DIR")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        code, lines = selftest()
        for line in lines:
            print(line)
        return code

    ran = False
    overall_code = 0

    if args.inventory is not None:
        ran = True
        code, message = run_inventory(args.inventory)
        print(message)
        overall_code = overall_code or code

    if args.packs is not None:
        ran = True
        code, message = run_packs(args.packs)
        print(message)
        overall_code = overall_code or code

    if args.coverage is not None:
        ran = True
        addons_dir = args.inventory if args.inventory is not None else DEFAULT_ADDONS_DIR
        code, message = run_coverage(args.coverage, addons_dir)
        print(message)
        overall_code = overall_code or code

    if ran:
        return overall_code

    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
