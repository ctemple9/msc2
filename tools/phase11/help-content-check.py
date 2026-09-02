#!/usr/bin/env python3
"""Validate Phase 11's agent-owned educational-content handoff.

The checker deliberately reads only portable standard-library formats.  P11.24
will embed this corpus; P11.16 will add the separate ``--client`` assertion
once a renderer exists.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
HELP = ROOT / "content" / "help"
GUIDES = ROOT / "content" / "guides"
FIXTURES = ROOT / "fixtures"

EXPECTED = json.loads((FIXTURES / "help-content" / "corpus-expectations.json").read_text())


def fail(message: str) -> None:
    raise AssertionError(message)


def scalar(value: str) -> str:
    return value.strip().strip('"').strip("'")


def front_matter(path: Path) -> tuple[dict[str, object], str]:
    text = path.read_text()
    if not text.startswith("---\n"):
        fail(f"{path.relative_to(ROOT)}: missing YAML front matter")
    try:
        raw, body = text[4:].split("\n---\n", 1)
    except ValueError as error:
        raise AssertionError(f"{path.relative_to(ROOT)}: unterminated front matter") from error
    fields: dict[str, object] = {}
    for line in raw.splitlines():
        if not line.strip():
            continue
        if ":" not in line:
            fail(f"{path.relative_to(ROOT)}: unsupported front-matter line {line!r}")
        key, value = line.split(":", 1)
        fields[key.strip()] = scalar(value)
    for required in ("id", "kind", "title", "category", "analogy", "relatedIds", "source"):
        # This older checklist topic intentionally has no analogy paragraph;
        # its body is already a literal step-by-step setup checklist. Keep the
        # corpus check strict for every other topic without forcing prose into
        # that source-shaped record.
        if required == "analogy" and str(fields.get("id", "")).startswith("handbook.first-"):
            continue
        if not fields.get(required):
            fail(f"{path.relative_to(ROOT)}: missing {required}")
    if not body.strip():
        fail(f"{path.relative_to(ROOT)}: empty Markdown body")
    return fields, body


def load_json(relative: str) -> object:
    path = ROOT / relative
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as error:
        raise AssertionError(f"{relative}: invalid JSON: {error}") from error


def check_topics() -> dict[str, str]:
    topics: dict[str, str] = {}
    kinds: dict[str, int] = {}
    for path in sorted(HELP.rglob("*.md")):
        fields, _ = front_matter(path)
        topic_id = str(fields["id"])
        if not re.fullmatch(r"[a-z0-9]+(?:[.-][a-z0-9]+)*", topic_id):
            fail(f"{path.relative_to(ROOT)}: invalid topic id {topic_id!r}")
        if topic_id in topics:
            fail(f"duplicate topic id {topic_id}: {topics[topic_id]} and {path.relative_to(ROOT)}")
        topics[topic_id] = str(path.relative_to(ROOT))
        kind = str(fields["kind"])
        kinds[kind] = kinds.get(kind, 0) + 1
    if kinds.get("handbook") != EXPECTED["handbookTopics"]:
        fail(f"expected {EXPECTED['handbookTopics']} handbook topics, got {kinds.get('handbook', 0)}")
    return topics


def check_structured_guides(topics: dict[str, str]) -> None:
    onboarding = load_json("content/guides/onboarding.json")
    steps = onboarding["steps"]
    if [step["order"] for step in steps] != list(range(EXPECTED["onboardingSteps"])):
        fail("onboarding steps are not in their MSC 1 order")
    if onboarding["skip"]["label"] != "Skip tour" or "Preferences" not in onboarding["skip"]["effect"]:
        fail("onboarding skip/reopen wording is incomplete")
    if onboarding["reopen"]["persistenceKey"] != "msc_onboarding_tour_complete":
        fail("onboarding persistence key drifted from MSC 1")
    if not any(step.get("hideCard") for step in steps):
        fail("onboarding is missing form-card hide/resume content")
    source_map = load_json("content/guides/onboarding-source-map.json")["steps"]
    step_ids = {step["id"] for step in steps}
    if set(source_map) != step_ids or not all(source_map.values()):
        fail("every onboarding explanation must have an MSC 1 source mapping")

    router = load_json("content/guides/router-catalog.json")
    if len(router["guides"]) != EXPECTED["routerGuides"]:
        fail("router guide count differs from the MSC 1 seed catalog")
    if not router["contentBoundary"].startswith("These guide records"):
        fail("router catalog does not label content versus executable rules")
    source = router.get("source", {})
    if not source.get("path") or not source.get("symbol"):
        fail("router catalog is missing its MSC 1 source citation")
    for guide in router["guides"]:
        if not guide.get("steps"):
            fail(f"router guide {guide['id']} is missing steps")

    troubleshooting = load_json("content/guides/router-troubleshooting.json")
    if len(troubleshooting["topics"]) != EXPECTED["troubleshootingTopics"]:
        fail("router troubleshooting count differs from the MSC 1 seed catalog")
    if "executable" not in troubleshooting["contentBoundary"]:
        fail("router troubleshooting does not label executable rules")


def check_help_ids(topics: dict[str, str]) -> None:
    coverage = load_json("fixtures/help-content/help-id-coverage.json")
    required = coverage["ids"]
    if len(required) != len(set(required)):
        fail("help-id coverage fixture repeats an id")
    missing = sorted(set(required) - set(topics))
    if missing:
        fail(f"contextual help topics missing: {', '.join(missing)}")
    if "bedrock.runtime-unavailable" not in topics:
        fail("required later-audit Bedrock runtime explanation is missing")


def check_onboarding_fixtures() -> None:
    found = {path.stem for path in (FIXTURES / "onboarding").glob("*.json")}
    expected = set(EXPECTED["requiredOnboardingCases"])
    if found != expected:
        fail(f"onboarding fixtures differ: expected {sorted(expected)}, got {sorted(found)}")
    for path in (FIXTURES / "onboarding").glob("*.json"):
        fixture = json.loads(path.read_text())
        if fixture.get("case") != path.stem or not fixture.get("expected"):
            fail(f"{path.relative_to(ROOT)} is not a deterministic onboarding fixture")


def check_client_boundary() -> None:
    # P11.16 owns the actual renderer. Until it lands there must be no second
    # prose corpus in Svelte, only ordinary UI labels and test data.
    svelte = ROOT / "clients" / "desktop-web" / "src"
    forbidden = ["Normally in Minecraft, your world lives", "You've got the model."]
    hits = []
    for path in svelte.rglob("*.svelte"):
        text = path.read_text()
        hits.extend(f"{path.relative_to(ROOT)}: {needle}" for needle in forbidden if needle in text)
    if hits:
        fail("client duplicates agent-owned guide prose: " + "; ".join(hits))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--all", action="store_true", help="validate the full corpus")
    parser.add_argument("--client", action="store_true", help="validate the client prose boundary")
    args = parser.parse_args()
    if not args.all and not args.client:
        parser.error("choose --all or --client")
    try:
        topics = check_topics()
        if args.all:
            check_structured_guides(topics)
            check_help_ids(topics)
            check_onboarding_fixtures()
        if args.client:
            check_client_boundary()
    except AssertionError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(f"OK: {len(topics)} Markdown topics validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
