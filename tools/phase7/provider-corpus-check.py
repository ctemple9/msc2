#!/usr/bin/env python3
"""Phase 7 provider corpus checker (P7.2).

Built before any real evidence is collected, so the bar for `corpus/providers/`
is set before it can be bent to fit whatever P7.3 turns up. Two independent
modes:

Inventory mode checks a provider-corpus directory (default `corpus/providers/`)
against its own `manifest.json`:

  - Every recorded provider response or installer-evidence file (anything
    under the directory except `manifest.json`, `README.md`, and dotfiles)
    has a manifest entry recording: which of the six families it belongs to
    (`family`), where it came from (`source_url`), when it was captured
    (`captured`), its SHA-256 (`sha256`), and its byte size (`byte_size`).
    A missing field fails loudly.
  - `family` must be one of the six families this phase's create flow
    offers (`vanilla`, `paper`, `purpur`, `fabric`, `neoforge`, `forge`) --
    a typo or a flavor outside this phase's scope (e.g. `pufferfish`,
    `spigot`) fails loudly rather than silently miscounting coverage later.
  - No two evidence files share a SHA-256 -- a duplicate isn't a second
    sample.
  - Every `.json` file parses as JSON; every `.xml` file (the Forge/NeoForge
    `maven-metadata.xml` shape) parses as XML. Other extensions (args
    files, run scripts, directory-shape listings) aren't parsed -- they
    aren't JSON or XML in the first place.
  - The recomputed SHA-256 of every evidence file matches what its manifest
    entry recorded -- an input that changed after being catalogued fails
    loudly instead of silently drifting from what the manifest claims.

Coverage mode takes a fixture directory (e.g. `fixtures/server-jar-providers/`)
and the provider corpus it should be characterized against:

  - Every fixture `*.json` file may carry an optional top-level
    `corpus_source` field -- a list of paths, relative to the provider
    corpus root, naming which recorded response(s) that case was
    characterized from. This is additive to the six required fields
    `docs/msc2/fixture-format.md` already defines; existing tooling ignores
    unknown fields, so this doesn't touch that spec.
  - Every path a fixture cites must actually have a manifest entry in the
    provider corpus -- a fixture cannot claim a response that was never
    recorded.
  - Across every fixture in the directory, all six families must be cited
    at least once -- a fixture directory that never exercises (say) Forge
    isn't complete coverage of the six-family boundary, even if every
    citation it does have is real.

Evidence mode (P7.28) checks a real-provisioning evidence directory (default
`docs/msc2/families/provisioning-evidence/`) -- the one Phase 7 step that
uses the real internet, proving live providers still return what the fake
providers/tests above were built to simulate:

  - Exactly one `<family>.json` per family in `FAMILIES`, no more, no fewer
    -- a family Cameron's own machine genuinely could not provision must
    show up as a missing/failing file here, not be silently absent.
  - Each file's own `family` field must match its filename -- a copy-paste
    that never got its family field updated fails loudly rather than
    silently miscounting which family that evidence is really for.
  - Required fields, all non-empty: `resolved_minecraft_version`,
    `download_url`, `checksum` (itself an object carrying non-empty
    `algorithm`/`value` and a present, possibly-`null`,
    `matches_provider_published` key -- `null` is how a family whose
    provider publishes no checksum to compare against records that
    honestly), `byte_size`, `launch_argv`, `install_seconds`.
  - `reached_ready` must be the literal boolean `true` -- this is the one
    field this mode refuses to accept any other value for, since a family
    that never reached a ready state is exactly the "stop and report it"
    case this step's own instructions call for, not something a checker
    should wave through.

Passing and deliberately failing self-tests (`--selftest`) prove each
rejection fires, for all three modes, against small synthetic fixtures
under `tools/phase7/fixtures/` -- never against `corpus/providers/` or
`docs/msc2/families/provisioning-evidence/` themselves, so nothing invented
ends up standing in for the real thing. No network access anywhere in this
tool.

  provider-corpus-check.py --inventory [--providers DIR]
                                             check a provider corpus
                                             directory (default:
                                             corpus/providers)
  provider-corpus-check.py --coverage FIXTURE_DIR [--providers DIR]
                                             check a fixture directory's
                                             citations against a provider
                                             corpus
  provider-corpus-check.py --evidence [EVIDENCE_DIR]
                                             check a real-provisioning
                                             evidence directory (default:
                                             docs/msc2/families/
                                             provisioning-evidence)
  provider-corpus-check.py --selftest       run all three modes against the
                                             fixtures in
                                             tools/phase7/fixtures/,
                                             proving the passing case
                                             succeeds and every
                                             deliberately-broken case fails

Stdlib only, on purpose: same reasoning as `tools/phase6/corpus-check.py`
and `tools/phase5/real-corpus-check.py` -- no dependency setup for Cameron
to fight.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

DEFAULT_PROVIDERS_DIR = Path("corpus/providers")
DEFAULT_EVIDENCE_DIR = Path("docs/msc2/families/provisioning-evidence")
FIXTURES_DIR = Path(__file__).resolve().parent / "fixtures"

FAMILIES = ("vanilla", "paper", "purpur", "fabric", "neoforge", "forge")

REQUIRED_ENTRY_FIELDS = ("family", "source_url", "captured", "sha256", "byte_size")

REQUIRED_EVIDENCE_FIELDS = (
    "family",
    "resolved_minecraft_version",
    "download_url",
    "checksum",
    "byte_size",
    "launch_argv",
    "reached_ready",
    "install_seconds",
)

REQUIRED_CHECKSUM_FIELDS = ("algorithm", "value")

IGNORED_NAMES = {"manifest.json", "README.md"}

# (mode, fixture case directory name, expected exit code)
SELFTEST_CASES = [
    ("inventory", "pass", 0),
    ("inventory", "missing-provenance", 1),
    ("inventory", "duplicate-hash", 1),
    ("inventory", "malformed-json", 1),
    ("inventory", "malformed-xml", 1),
    ("inventory", "mutated-input", 1),
    ("inventory", "unknown-family", 1),
    ("coverage", "coverage-pass", 0),
    ("coverage", "coverage-missing-family", 1),
    ("coverage", "coverage-dangling-citation", 1),
    ("evidence", "evidence-pass", 0),
    ("evidence", "evidence-missing-family", 1),
    ("evidence", "evidence-family-mismatch", 1),
    ("evidence", "evidence-not-ready", 1),
    ("evidence", "evidence-missing-field", 1),
]


class CheckError(Exception):
    pass


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def is_ignored(path: Path) -> bool:
    return path.name in IGNORED_NAMES or path.name.startswith(".")


def load_manifest(providers_dir: Path) -> dict[str, dict]:
    manifest_path = providers_dir / "manifest.json"
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
        for field in REQUIRED_ENTRY_FIELDS:
            value = entry.get(field)
            if value is None or value == "":
                raise CheckError(f"{manifest_path}: {path} missing '{field}'")
        if entry["family"] not in FAMILIES:
            raise CheckError(
                f"{manifest_path}: {path} has unknown family {entry['family']!r} "
                f"(expected one of {', '.join(FAMILIES)})"
            )
        by_path[path] = entry
    return by_path


def check_parseable(file_path: Path) -> None:
    """Only `.json`/`.xml` evidence is parsed -- args files, run scripts,
    and directory-shape listings aren't JSON or XML in the first place."""
    if file_path.suffix == ".json":
        try:
            json.loads(file_path.read_text())
        except json.JSONDecodeError as exc:
            raise CheckError(f"{file_path}: malformed JSON ({exc})")
    elif file_path.suffix == ".xml":
        try:
            ET.fromstring(file_path.read_text())
        except ET.ParseError as exc:
            raise CheckError(f"{file_path}: malformed XML ({exc})")


def check_inventory(providers_dir: Path) -> str:
    """Raises CheckError on the first evidence gap found; returns an "ok"
    message describing what passed otherwise."""
    if not providers_dir.is_dir():
        raise CheckError(f"{providers_dir}: provider corpus directory does not exist")

    manifest = load_manifest(providers_dir)

    seen_hashes: dict[str, str] = {}
    families_present: set[str] = set()
    file_count = 0

    for file_path in sorted(providers_dir.rglob("*")):
        if file_path.is_dir() or is_ignored(file_path):
            continue
        rel_str = str(file_path.relative_to(providers_dir))

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

        check_parseable(file_path)

        families_present.add(entry["family"])
        file_count += 1

    unrecorded = set(manifest) - {
        str(p.relative_to(providers_dir)) for p in providers_dir.rglob("*") if not p.is_dir()
    }
    if unrecorded:
        raise CheckError(f"{providers_dir}: manifest cites path(s) with no file on disk: {sorted(unrecorded)}")

    return (
        f"ok {providers_dir} ({file_count} evidence file(s), "
        f"families present: {', '.join(sorted(families_present)) or 'none'})"
    )


def run_inventory(providers_dir: Path) -> tuple[int, str]:
    try:
        message = check_inventory(providers_dir)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


def check_coverage(fixture_dir: Path, providers_dir: Path) -> str:
    """Raises CheckError on the first gap found; returns an "ok" message
    describing what passed otherwise. Trusts the provider corpus is
    already sound -- run inventory mode on it separately; this only checks
    citations against whatever manifest is there."""
    if not fixture_dir.is_dir():
        raise CheckError(f"{fixture_dir}: fixture directory does not exist")
    manifest = load_manifest(providers_dir)

    families_cited: set[str] = set()
    citation_count = 0

    for fixture_path in sorted(fixture_dir.rglob("*.json")):
        try:
            data = json.loads(fixture_path.read_text())
        except json.JSONDecodeError as exc:
            raise CheckError(f"{fixture_path}: malformed JSON ({exc})")
        if not isinstance(data, dict):
            continue
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
                    f"entry in {providers_dir} -- a fixture cannot claim a response "
                    "that was never recorded"
                )
            families_cited.add(entry["family"])
            citation_count += 1

    missing_families = sorted(set(FAMILIES) - families_cited)
    if missing_families:
        raise CheckError(
            f"{fixture_dir}: no fixture cites a corpus response for "
            f"{', '.join(missing_families)} -- coverage of the six-family "
            "boundary is incomplete"
        )

    return f"ok {fixture_dir} ({citation_count} citation(s), all six families represented)"


def run_coverage(fixture_dir: Path, providers_dir: Path) -> tuple[int, str]:
    try:
        message = check_coverage(fixture_dir, providers_dir)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


def check_evidence(evidence_dir: Path) -> str:
    """Raises CheckError on the first gap found; returns an "ok" message
    describing what passed otherwise. Independent of the provider corpus
    (inventory mode already gates that separately) -- this only checks the
    real-provisioning evidence directory's own shape."""
    if not evidence_dir.is_dir():
        raise CheckError(f"{evidence_dir}: evidence directory does not exist")

    present = {
        p.stem: p
        for p in sorted(evidence_dir.glob("*.json"))
        if p.name not in IGNORED_NAMES
    }

    missing = [f for f in FAMILIES if f not in present]
    if missing:
        raise CheckError(
            f"{evidence_dir}: missing evidence for {', '.join(missing)} -- "
            "a family that could not be provisioned must show up as a "
            "missing/failing file, not be silently absent"
        )

    unknown = sorted(set(present) - set(FAMILIES))
    if unknown:
        raise CheckError(
            f"{evidence_dir}: evidence file(s) for unknown family(ies) {unknown} "
            f"(expected one of {', '.join(FAMILIES)})"
        )

    for family in FAMILIES:
        file_path = present[family]
        try:
            entry = json.loads(file_path.read_text())
        except json.JSONDecodeError as exc:
            raise CheckError(f"{file_path}: malformed JSON ({exc})")

        for field in REQUIRED_EVIDENCE_FIELDS:
            if field not in entry or entry[field] in (None, ""):
                raise CheckError(f"{file_path}: missing '{field}'")

        if entry["family"] != family:
            raise CheckError(
                f"{file_path}: 'family' field is {entry['family']!r}, "
                f"expected {family!r} to match its filename"
            )

        checksum = entry["checksum"]
        if not isinstance(checksum, dict):
            raise CheckError(f"{file_path}: 'checksum' must be an object")
        for field in REQUIRED_CHECKSUM_FIELDS:
            if not checksum.get(field):
                raise CheckError(f"{file_path}: checksum missing '{field}'")
        if "matches_provider_published" not in checksum:
            raise CheckError(
                f"{file_path}: checksum missing 'matches_provider_published' "
                "(true/false when the provider publishes one to compare "
                "against, null when it doesn't -- the key must still be "
                "present either way)"
            )

        if entry["reached_ready"] is not True:
            raise CheckError(
                f"{file_path}: reached_ready is {entry['reached_ready']!r}, "
                "not true -- a family that never reached a ready state must "
                "be stopped and reported, not recorded as passing evidence"
            )

    return f"ok {evidence_dir} (all six families present, reached_ready true for each)"


def run_evidence(evidence_dir: Path) -> tuple[int, str]:
    try:
        message = check_evidence(evidence_dir)
    except CheckError as exc:
        return 1, str(exc)
    return 0, message


def selftest() -> tuple[int, list[str]]:
    lines = []
    all_ok = True
    for mode, name, expected_code in SELFTEST_CASES:
        case_dir = FIXTURES_DIR / name
        if mode == "inventory":
            code, message = run_inventory(case_dir / "providers")
        elif mode == "coverage":
            code, message = run_coverage(case_dir / "fixtures", case_dir / "providers")
        else:
            code, message = run_evidence(case_dir / "evidence")
        ok = code == expected_code
        all_ok = all_ok and ok
        lines.append(f"{'pass' if ok else 'FAIL'} {mode}/{name}: expected={expected_code} got={code} ({message})")
    return (0 if all_ok else 1), lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--providers", type=Path, default=DEFAULT_PROVIDERS_DIR)
    parser.add_argument("--inventory", action="store_true")
    parser.add_argument("--coverage", type=Path, default=None, metavar="FIXTURE_DIR")
    parser.add_argument(
        "--evidence",
        type=Path,
        nargs="?",
        const=DEFAULT_EVIDENCE_DIR,
        default=None,
        metavar="EVIDENCE_DIR",
    )
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        code, lines = selftest()
        for line in lines:
            print(line)
        return code

    if args.evidence is not None:
        code, message = run_evidence(args.evidence)
        print(message)
        return code

    if args.coverage is not None:
        code, message = run_coverage(args.coverage, args.providers)
        print(message)
        return code

    if args.inventory:
        code, message = run_inventory(args.providers)
        print(message)
        return code

    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
