#!/usr/bin/env python3
"""Small completion checker for the P12.29 world-settings release gate.

This is deliberately a repository-evidence check, not another test runner.
It verifies that the bounded release note, served Handbook topics, capability
matrix notes, and the targeted UI/Rust/CLI evidence all still describe the
same ownership boundary. ``--selftest`` also exercises the checker against a
tiny clean and deliberately incomplete fixture so a broken checker is visible
without requiring a workspace-wide test run.
"""

from __future__ import annotations

import csv
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]

HELP_REQUIREMENTS = {
    "content/help/handbook/worlds-backups.md": [
        "saved with the world",
        "force-gamemode",
        "provided by this server/mod",
        "permanently disable Xbox achievements",
        "not sent as though it applied",
    ],
    "content/help/handbook/bedrock.md": [
        "World settings and achievements",
        "permanently disable Xbox achievements",
        "agent-enforced across Settings",
        "Java has different semantics",
        "force-gamemode",
    ],
    "content/help/concept/worlds.md": [
        "versioned profile",
        "saved with the world",
        "not silently presented as MSC controls",
    ],
    "content/help/concept/active-world.md": [
        "saved with the world",
        "applied live",
        "awaits a server restart",
    ],
    "content/help/concept/settings.md": [
        "two homes",
        "force-gamemode",
        "world-scoped confirmation",
        "server-wide confirmation",
    ],
    "content/help/contextual/settings-difficulty.md": [
        "saved with that world slot",
        "not a server-wide setting",
    ],
    "content/help/contextual/settings-gamemode.md": [
        "id: settings.gamemode",
        "saved with the selected world slot",
        "Xbox achievements",
        "force-gamemode",
    ],
}

RELEASE_REQUIREMENTS = [
    "# World-settings release boundary",
    "## Ownership contract",
    "## Native support boundary",
    "## Evidence map",
    "## Cameron's live UI walk",
    "Create Server wizard's first-world path",
    "Duplicate, import, copy, and restore",
    "Activation and restart timing",
    "It is off by default",
    "Safety confirmation is shared across settings",
    "provided by this server/mod",
]

MATRIX_ROWS = {
    ("GET", "/v1/capabilities"),
    ("GET", "/v1/help/catalog"),
    ("GET", "/v1/help/{helpId}"),
    ("GET", "/v1/settings"),
    ("POST", "/v1/settings"),
    ("GET", "/v1/worlds"),
    ("POST", "/v1/worlds/activate"),
    ("POST", "/v1/worlds/create"),
    ("POST", "/v1/worlds/duplicate"),
    ("POST", "/v1/worlds/import"),
    ("POST", "/v1/worlds/replace"),
    ("GET", "/v1/worlds/{slotId}/profile"),
    ("POST", "/v1/worlds/{slotId}/profile"),
    ("POST", "/v1/backups/restore"),
}

EVIDENCE_REQUIREMENTS = {
    "clients/desktop-web/tests/screens/world-settings.test.ts": [
        "one form for the fresh-server and Worlds-tab creation paths",
        "force-gamemode",
        "selected edition or runtime cannot support",
        "two slot profiles distinct",
        "migration actions",
    ],
    "crates/msc-application/tests/world_activation.rs": [
        "world_activation_switches_between_distinct_profiles_and_preserves_server_settings",
        "world_activation_reports_restart_required_bedrock_profile_changes",
    ],
    "crates/msc-agent/tests/cli_worlds_backups.rs": [
        "world_profile_set_help_shows_changes_and_confirmation",
        "--change",
        "--confirm",
    ],
    "crates/msc-application/tests/command_input.rs": [
        "world_safety_confirmation_contract_distinguishes_bedrock_and_server_scope",
    ],
    "crates/msc-agent/tests/bedrock_routes.rs": [
        "safety_confirmation_is_part_of_the_shared_api_contract",
    ],
}


def read(root: Path, relative: str) -> str:
    return (root / relative).read_text(encoding="utf-8")


def missing_fragments(text: str, fragments: list[str]) -> list[str]:
    # Markdown line wrapping is presentation, not a change to the evidence
    # phrase. Compare a whitespace-normalized copy so a reflowed paragraph
    # cannot make the completion gate report a false omission.
    normalized_text = " ".join(text.split())
    return [
        fragment
        for fragment in fragments
        if fragment not in text and " ".join(fragment.split()) not in normalized_text
    ]


def check_help(root: Path) -> list[str]:
    problems: list[str] = []
    for relative, fragments in HELP_REQUIREMENTS.items():
        path = root / relative
        if not path.is_file():
            problems.append(f"missing help topic: {relative}")
            continue
        missing = missing_fragments(read(root, relative), fragments)
        problems.extend(f"{relative}: missing {fragment!r}" for fragment in missing)
    return problems


def check_release_note(root: Path) -> list[str]:
    relative = "docs/msc2/clients/world-settings.md"
    path = root / relative
    if not path.is_file():
        return [f"missing release note: {relative}"]
    return [
        f"{relative}: missing {fragment!r}"
        for fragment in missing_fragments(read(root, relative), RELEASE_REQUIREMENTS)
    ]


def check_matrix(root: Path) -> list[str]:
    relative = "docs/msc2/client-capability-matrix.csv"
    path = root / relative
    if not path.is_file():
        return [f"missing capability matrix: {relative}"]

    with path.open(newline="", encoding="utf-8") as stream:
        rows = {(row["method"], row["path"]): row for row in csv.DictReader(stream)}

    problems: list[str] = []
    for key in sorted(MATRIX_ROWS):
        row = rows.get(key)
        if row is None:
            problems.append(f"matrix missing {key[0]} {key[1]}")
        elif "P12.29" not in row["notes"]:
            problems.append(f"matrix {key[0]} {key[1]} has no P12.29 evidence note")
    return problems


def check_evidence(root: Path) -> list[str]:
    problems: list[str] = []
    for relative, fragments in EVIDENCE_REQUIREMENTS.items():
        path = root / relative
        if not path.is_file():
            problems.append(f"missing evidence file: {relative}")
            continue
        missing = missing_fragments(read(root, relative), fragments)
        problems.extend(f"{relative}: missing {fragment!r}" for fragment in missing)
    return problems


def check_repo(root: Path) -> list[str]:
    return check_help(root) + check_release_note(root) + check_matrix(root) + check_evidence(root)


def selftest() -> tuple[int, list[str]]:
    clean = "World settings are saved with the world. force-gamemode is a server-wide policy."
    dirty = "World settings are saved with the world."
    clean_missing = missing_fragments(clean, ["saved with the world", "force-gamemode"])
    dirty_missing = missing_fragments(dirty, ["saved with the world", "force-gamemode"])
    problems = check_repo(ROOT)
    lines = [
        f"fixture-clean={'pass' if not clean_missing else 'fail'}",
        f"fixture-dirty={'fail-as-expected' if dirty_missing else 'unexpected-pass'}",
    ]
    if clean_missing:
        problems.append("clean fixture was rejected")
    if not dirty_missing:
        problems.append("dirty fixture was accepted")
    if problems:
        lines.extend(f"error: {problem}" for problem in problems)
        return 1, lines
    lines.append("repository: Handbook, release note, matrix, and targeted evidence are complete")
    return 0, lines


def main() -> int:
    if sys.argv[1:] != ["--selftest"]:
        print(__doc__)
        return 2
    code, lines = selftest()
    print("\n".join(lines))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
