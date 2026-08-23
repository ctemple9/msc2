#!/usr/bin/env python3
"""Check the copied iOS client's Phase 10 Bedrock contract surface.

The check is intentionally source- and schema-oriented. It catches a DTO or
route being added only on one side of the contract, and it delegates the
matrix's complete operation coverage check to the established Phase 6 tool.
"""

import csv
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
IOS_SOURCE = ROOT / "clients/ios/MSCRemoteiOS_Swift"
IOS_TESTS = ROOT / "clients/ios/MSCRemoteiOSTests"
PROJECT = ROOT / "clients/ios/MSCRemoteiOS.xcodeproj/project.pbxproj"
OPENAPI = ROOT / "docs/msc2/api-contract/openapi.json"
MATRIX = ROOT / "docs/msc2/client-capability-matrix.csv"


def fail(problems, message):
    problems.append(message)


def load_source(name):
    path = IOS_SOURCE / name
    return path, path.read_text()


def struct_block(source, name):
    marker = f"struct {name}"
    start = source.find(marker)
    if start < 0:
        return ""
    end = len(source)
    for next_marker in ("\nstruct ", "\nenum ", "\nfinal class ", "\n// MARK:"):
        candidate = source.find(next_marker, start + len(marker))
        if candidate >= 0:
            end = min(end, candidate)
    return source[start:end]


def check_openapi(problems):
    document = json.loads(OPENAPI.read_text())
    schemas = document["components"]["schemas"]
    runtime = schemas.get("BedrockRuntimeStateDTO")
    if runtime is None:
        fail(problems, "OpenAPI is missing BedrockRuntimeStateDTO")
    else:
        required = {"state", "backend", "hostOs", "reasonCode", "message", "helpId"}
        actual = set(runtime.get("properties", {}))
        missing = required - actual
        if missing:
            fail(problems, f"BedrockRuntimeStateDTO is missing properties: {sorted(missing)}")

    runtime_schemas = [
        "RemoteAPIStatus",
        "ServerDTO",
        "ServerCreateResultDTO",
        "PerformanceSnapshotDTO",
        "PlayersResponseDTO",
        "AllowlistResponseDTO",
        "AllowlistMutationResultDTO",
        "SettingsResponseDTO",
        "SettingsUpdateResultDTO",
        "VersionsResponseDTO",
    ]
    for name in runtime_schemas:
        if "runtime" not in schemas.get(name, {}).get("properties", {}):
            fail(problems, f"OpenAPI {name} is missing additive runtime")

    setting_field = schemas.get("SettingFieldDTO", {}).get("properties", {})
    if "helpId" not in setting_field or "help" in setting_field:
        fail(problems, "OpenAPI SettingFieldDTO must use helpId without retired inline help")
    if "cancelable" not in schemas.get("OperationDTO", {}).get("properties", {}):
        fail(problems, "OpenAPI OperationDTO is missing cancelable")
    if "details" not in schemas.get("ErrorDTO", {}).get("properties", {}):
        fail(problems, "OpenAPI ErrorDTO is missing structured details")


def check_ios_sources(problems):
    required_files = [
        "RemoteAPIModels.swift",
        "RemoteAPIClient.swift",
        "DashboardViewModel.swift",
        "DashboardViewModel+Performance.swift",
        "PlayersView.swift",
        "AllowlistView.swift",
        "ServerSettingsView.swift",
    ]
    sources = {}
    for name in required_files:
        path, source = load_source(name)
        sources[name] = source
        if not source.strip():
            fail(problems, f"iOS source is empty: {path}")

    models = sources["RemoteAPIModels.swift"]
    for marker in (
        "struct BedrockRuntimeStateDTO",
        "struct BedrockSupportDTO",
        "struct CapabilitiesDTO",
        "let state: String",
        "let backend: String?",
        "let hostOs: String?",
        "let reasonCode: String?",
        "let helpId: String?",
    ):
        if marker not in models:
            fail(problems, f"iOS models are missing {marker}")

    for name in (
        "RemoteAPIStatus",
        "ServerDTO",
        "ServerCreateResultDTO",
        "PerformanceSnapshotDTO",
        "PlayersResponse",
        "AllowlistResponseDTO",
        "AllowlistMutationResultDTO",
        "SettingsResponseDTO",
        "SettingsUpdateResultDTO",
        "VersionsResponseDTO",
    ):
        block = struct_block(models, name)
        if "let runtime:" not in block:
            fail(problems, f"iOS {name} is missing runtime decoding")

    settings_field = struct_block(models, "SettingFieldDTO")
    for marker in ("helpId", "CodingKeys", 'case key, label, type, value, minInt, maxInt, unit, maxLength, options, helpId, help'):
        if marker not in settings_field:
            fail(problems, f"iOS SettingFieldDTO is missing {marker}")
    if "More help:" not in settings_field:
        fail(problems, "iOS SettingFieldDTO has no compatibility display fallback for helpId")

    operation = struct_block(models, "OperationDTO")
    if "cancelable" not in operation:
        fail(problems, "iOS OperationDTO is missing cancelable")
    error = struct_block(models, "ErrorDTO")
    if "details" not in error:
        fail(problems, "iOS ErrorDTO is missing structured details")

    client = sources["RemoteAPIClient.swift"]
    for marker in (
        "func getCapabilities()",
        "func getStatus()",
        "func getPlayers()",
        "func getAllowlist()",
        "func getSettings()",
        "func start()",
        "func stop()",
        "func cancelOperation(id:",
        "func pollOperationToTerminal(id:",
        "apiErrorDetails",
        "details: ErrorDetailsDTO?",
    ):
        if marker not in client:
            fail(problems, f"iOS client is missing {marker}")

    for name in ("PlayersView.swift", "AllowlistView.swift", "ServerSettingsView.swift"):
        if "runtimeNotice" not in sources[name]:
            fail(problems, f"iOS {name} does not present runtime availability")

    test_path = IOS_TESTS / "Phase10BedrockContractTests.swift"
    if not test_path.exists():
        fail(problems, f"missing iOS contract tests: {test_path}")
    if str(test_path.name) not in PROJECT.read_text():
        fail(problems, "Phase10BedrockContractTests.swift is not registered in the Xcode project")


def check_matrix(problems):
    matrix_checker = ROOT / "tools/phase6/capability-matrix-check.py"
    result = subprocess.run(
        [sys.executable, str(matrix_checker), str(MATRIX)],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode:
        detail = (result.stderr or result.stdout).strip().replace("\n", "; ")
        fail(problems, f"capability matrix check failed: {detail}")

    with MATRIX.open(newline="") as handle:
        rows = list(csv.DictReader(handle))
    by_operation = {(row["method"], row["path"]): row for row in rows}
    expected_ios = {
        ("GET", "/v1/capabilities"),
        ("GET", "/v1/status"),
        ("POST", "/v1/start"),
        ("POST", "/v1/stop"),
        ("GET", "/v1/players"),
        ("GET", "/v1/allowlist"),
        ("POST", "/v1/allowlist"),
        ("GET", "/v1/settings"),
        ("POST", "/v1/settings"),
    }
    for operation in expected_ios:
        row = by_operation.get(operation)
        if row is None or row["ios_status"] != "Implemented":
            fail(problems, f"matrix does not mark iOS Bedrock operation implemented: {operation}")

    for row in rows:
        if row["desktop_web_status"] != "Planned":
            fail(problems, f"matrix claims a desktop/web surface during Phase 10: {row['method']} {row['path']}")


def main():
    problems = []
    check_openapi(problems)
    check_ios_sources(problems)
    check_matrix(problems)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    print("ok: iOS Bedrock DTOs, shared routes, presentation disclosures, tests, OpenAPI, and matrix are aligned")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
