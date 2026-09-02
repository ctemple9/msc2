#!/usr/bin/env python3
"""Check that the generated client surface covers the frozen OpenAPI contract."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "docs/msc2/api-contract/openapi.json"
GENERATED_PATH = ROOT / "clients/desktop-web/src/lib/api/generated.ts"
SCHEMA_START = re.compile(r"^    ([A-Za-z_$][A-Za-z0-9_$]*): ", re.MULTILINE)
PROPERTY = re.compile(r"^      ([A-Za-z_$][A-Za-z0-9_$]*)(\?)?:", re.MULTILINE)


class CheckError(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise CheckError(message)


def schema_blocks(generated: str) -> dict[str, str]:
    marker = "export interface components"
    start = generated.index(marker)
    section = generated[start:]
    matches = list(SCHEMA_START.finditer(section))
    blocks: dict[str, str] = {}
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(section)
        blocks[match.group(1)] = section[match.start() : end]
    return blocks


def check() -> list[str]:
    try:
        contract = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))
        generated = GENERATED_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        raise CheckError(f"cannot read contract or generated output: {error}") from error

    schemas = contract["components"]["schemas"]
    blocks = schema_blocks(generated)
    missing_schemas = sorted(set(schemas) - set(blocks))
    require(not missing_schemas, f"generated output is missing schemas: {missing_schemas}")
    require(
        "BedrockRuntimeStateDTO" in blocks,
        "generated output must include BedrockRuntimeStateDTO for additive capability skew",
    )

    for name, schema in schemas.items():
        properties = {match.group(1): match.group(2) == "?" for match in PROPERTY.finditer(blocks[name])}
        missing_properties = sorted(set(schema.get("properties", {})) - properties.keys())
        require(not missing_properties, f"{name}: generated output is missing {missing_properties}")
        required = set(schema.get("required", []))
        wrong_requiredness = sorted(
            field
            for field in schema.get("properties", {})
            if properties[field] != (field not in required)
        )
        require(not wrong_requiredness, f"{name}: required/optional fields drifted: {wrong_requiredness}")
        # openapi-typescript represents an intentionally empty object schema as
        # Record<string, never>; there are no declared fields to drift in that
        # shape, so it is the only valid exception to the usual additive-field
        # intersection emitted for populated schemas.
        require(
            "[key: string]: unknown;" in blocks[name]
            or (not schema.get("properties") and "Record<string, never>" in blocks[name]),
            f"{name}: additive unknown fields are not accepted",
        )

    paths = contract["paths"]
    require("export interface paths" in generated, "generated output has no HTTP paths interface")
    # openapi-typescript uses single-quoted keys in the generated paths
    # interface; the contract JSON uses double-quoted keys.
    missing_paths = sorted(path for path in paths if f"'{path}':" not in generated)
    require(not missing_paths, f"generated output is missing paths: {missing_paths}")

    operations = {
        operation.get("operationId")
        for methods in paths.values()
        for operation in methods.values()
        if operation.get("operationId")
    }
    require("export interface operations" in generated, "generated output has no operation interface")
    missing_operations = sorted(operation for operation in operations if f"  {operation}: {{" not in generated)
    require(not missing_operations, f"generated output is missing operations: {missing_operations}")

    return [
        f"schemas: {len(schemas)} generated",
        f"paths: {len(paths)} generated",
        f"operations: {len(operations)} generated",
        "additive fields: unknown properties accepted",
    ]


def main() -> int:
    try:
        for line in check():
            print(f"OK: {line}")
    except (CheckError, ValueError, KeyError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
