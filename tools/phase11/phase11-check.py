#!/usr/bin/env python3
"""Run the final shared-client consistency checks used by P12.LAST.

This is a focused evidence gate. It checks repository structure and delegates
contract/help/DTO checks to the existing narrowly-scoped checkers; it does not
run the client or workspace test suites, which the declared Verify command
runs separately.
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CLIENT = ROOT / "clients" / "desktop-web"
MATRIX = ROOT / "docs/msc2/client-capability-matrix.csv"
OPENAPI = ROOT / "docs/msc2/api-contract/openapi.json"
WEBSOCKET = ROOT / "docs/msc2/api-contract/websocket-v1.json"
EVIDENCE = ROOT / "docs/msc2/clients/phase12-gate.md"

ALLOWED_DESKTOP_PLANNED = {
    ("POST", "/v1/broadcast/download-jar"),
    ("GET", "/v1/broadcast/jar-status"),
    ("POST", "/v1/broadcast/restart"),
    ("WS", "/v1/console/stream"),
    ("WS", "/v1/notifications/stream"),
    ("POST", "/v1/operations"),
    ("WS", "/v1/operations/{id}/stream"),
    ("GET", "/v1/players/{profileId}/skin"),
    ("POST", "/v1/resourcepacks/activate"),
    ("POST", "/v1/resourcepacks/remove"),
    ("POST", "/v1/resourcepacks/seturl"),
    ("POST", "/v1/templates"),
    ("POST", "/v1/watchdog/disable"),
    ("POST", "/v1/watchdog/enable"),
    ("GET", "/v1/watchdog/status"),
}


def fail(message: str) -> None:
    raise AssertionError(message)


def read(relative: str) -> str:
    path = ROOT / relative
    if not path.is_file():
        fail(f"missing {relative}")
    return path.read_text(encoding="utf-8")


def contract_routes() -> set[tuple[str, str]]:
    http = json.loads(OPENAPI.read_text(encoding="utf-8"))
    routes = {
        (method.upper(), path)
        for path, operations in http["paths"].items()
        for method in operations
        if method in {"get", "post", "put", "patch", "delete"}
    }
    websocket = json.loads(WEBSOCKET.read_text(encoding="utf-8"))
    routes.update(("WS", channel["path"]) for channel in websocket["channels"])
    return routes


def matrix_rows() -> list[dict[str, str]]:
    with MATRIX.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def check_matrix() -> list[str]:
    rows = matrix_rows()
    if not rows:
        fail("capability matrix is empty")
    actual = {(row["method"], row["path"]) for row in rows}
    expected = contract_routes()
    if actual != expected:
        fail(
            "matrix/contract mismatch: "
            f"missing={sorted(expected - actual)} extra={sorted(actual - expected)}"
        )
    if len(actual) != len(rows):
        fail("capability matrix contains duplicate route rows")
    required_columns = {
        "method",
        "path",
        "agent_status",
        "desktop_web_status",
        "ios_status",
        "cli_status",
        "notes",
    }
    missing_columns = required_columns - set(rows[0])
    if missing_columns:
        fail(f"capability matrix header is missing {sorted(missing_columns)}")
    for row in rows:
        for column in (
            "method",
            "path",
            "agent_status",
            "desktop_web_status",
            "ios_status",
            "cli_status",
        ):
            if not row.get(column, "").strip():
                fail(f"blank matrix field: {row.get('method')} {row.get('path')} {column}")
        if row["desktop_web_status"] not in {"Implemented", "Planned"}:
            fail(f"invalid Desktop/Web status: {row['desktop_web_status']}")
    planned = {
        (row["method"], row["path"])
        for row in rows
        if row["desktop_web_status"] == "Planned"
    }
    if planned != ALLOWED_DESKTOP_PLANNED:
        fail(
            "Desktop/Web Planned set drifted: "
            f"missing={sorted(ALLOWED_DESKTOP_PLANNED - planned)} "
            f"unexpected={sorted(planned - ALLOWED_DESKTOP_PLANNED)}"
        )
    return [f"{len(rows)} contract operations reconciled; {len(planned)} explicit Desktop/Web future rows"]


def require_fragments(relative: str, fragments: tuple[str, ...]) -> None:
    text = read(relative)
    missing = [fragment for fragment in fragments if fragment not in text]
    if missing:
        fail(f"{relative}: missing {missing}")


def run_checker(relative: str, *args: str) -> None:
    command = [sys.executable, str(ROOT / relative), *args]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if result.returncode:
        output = (result.stdout + result.stderr).strip()
        fail(f"{relative} {' '.join(args)} failed: {output}")


def check_shared_boundaries() -> list[str]:
    require_fragments(
        "docs/msc2/clients/phase12-gate.md",
        (
            "ContentView.swift",
            "DetailsView.swift",
            "AddServerWizardView.swift",
            "ServerEditorView.swift",
            "MSCSettingsView.swift",
            "SetupWizardView.swift",
            "OnboardingManager.swift",
            "ServerHandbookView.swift",
            "Packs is the named Phase 12 screen exception",
            "Pending Cameron visual review",
        ),
    )
    require_fragments(
        "clients/desktop-web/src/App.svelte",
        ("ApplicationShell", "PRIMARY_TABS", "FirstLaunchGate", "SplashGate", "getPlatform"),
    )
    require_fragments(
        "clients/desktop-web/src/lib/navigation/registry.ts",
        ("requiredPermissions", "isAvailable", "scope === 'host'", "context.serverId"),
    )
    require_fragments(
        "clients/desktop-web/src/lib/navigation/layout.ts",
        ("DEFAULT_NARROW_BREAKPOINT", "layoutForWidth"),
    )
    if list((CLIENT / "src-tauri").rglob("*.svelte")):
        fail("Tauri boundary contains a Svelte screen; D-003 requires shared screens")
    return ["shared Svelte/Tauri screen boundary and responsive registry present"]


def check_evidence_paths() -> list[str]:
    require_fragments(
        "docs/msc2/clients/phase12-gate.md",
        (
            "phase12-gate.md",
            "antiAIslop.md",
            "Pending Cameron screen-by-screen pass",
            "Release signing/notarization",
        ),
    )
    require_fragments(
        "clients/desktop-web/tests/hosts/host-state.test.ts",
        ("host-scoped", "isolated"),
    )
    require_fragments(
        "clients/desktop-web/tests/navigation/navigation.test.ts",
        ("permission", "capability", "host"),
    )
    require_fragments(
        "clients/desktop-web/tests/auth/browser.test.ts",
        ("CSRF", "cookie"),
    )
    require_fragments(
        "clients/desktop-web/tests/auth/desktop/desktop.test.ts",
        ("bearer", "host ID"),
    )
    require_fragments(
        "clients/desktop-web/tests/screens/first-launch-reset.test.ts",
        ("first launch", "never creates a server"),
    )
    require_fragments(
        "clients/desktop-web/tests/screens/settings-reset.test.ts",
        ("Reset this client", "RESET AGENT", "host/reset"),
    )
    require_fragments(
        "clients/desktop-web/tests/screens/help.test.ts",
        ("agent Markdown", "guided tour", "Handbook"),
    )
    require_fragments(
        ".github/workflows/ci.yml",
        (
            "platform: linux",
            "platform: macos",
            "platform: windows",
            "headless-link",
            "linux-webkitgtk-smoke.sh --native",
        ),
    )
    require_fragments(
        "docs/msc2/clients/evidence/tri-platform-ci.md",
        ("one production Svelte bundle", "native WebKitGTK", "Playwright"),
    )
    return ["host/auth/help/onboarding/reset and tri-platform/headless evidence paths present"]


def check_renderings() -> list[str]:
    require_fragments(
        "docs/msc2/renderings/README.md",
        (
            "status-card.html",
            "buttons-and-type.html",
            "primitives.html",
            "shell.html",
            "card language",
            "Spacing scale (4pt)",
        ),
    )
    require_fragments(
        "docs/msc2/antiAIslop.md",
        (
            "Color budget 70/20/10",
            "No glassmorphism",
            "Flat containment",
            "Motion with purpose only",
        ),
    )
    require_fragments(
        "clients/desktop-web/tests/visual/shell.test.ts",
        ("no glass", "no gradient fills", "per-card accent rails"),
    )
    return ["locked renderings and anti-slop review boundary present"]


def check_gate() -> list[str]:
    results = []
    results.extend(check_matrix())
    results.extend(check_shared_boundaries())
    results.extend(check_evidence_paths())
    results.extend(check_renderings())
    run_checker("tools/phase11/generated-types-check.py")
    run_checker("tools/phase11/help-content-check.py", "--all")
    run_checker("tools/phase11/help-content-check.py", "--client")
    run_checker("tools/phase12/reset-contract-check.py", "--selftest")
    results.append("generated DTOs, served help, and reset contract checks passed")
    return results


def selftest() -> int:
    try:
        results = check_gate()
    except (AssertionError, OSError, json.JSONDecodeError, KeyError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print("fixture-clean=pass")
    for result in results:
        print(f"OK: {result}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", action="store_true", help="check the final Phase 12 evidence boundary")
    args = parser.parse_args()
    if not args.gate:
        parser.error("choose --gate")
    return selftest()


if __name__ == "__main__":
    raise SystemExit(main())
