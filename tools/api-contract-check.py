#!/usr/bin/env python3
"""v1 API contract checker (P2.8).

Used as the Verify command for docs/msc2/rolling-plan.md's P2.8 step. Checks
the four things that step's own "What" promises about
docs/msc2/api-contract/openapi.json, the file assembled from the P0.23/P0.30/
P0.32 baseline plus P2.1/P2.2/P2.4/P2.5/P2.6's designs:

  1. every route sits under /v1/
  2. every route declares a permission category (x-permission-category)
  3. every field flagged for helpId in P2.2 (helpid-contract.md SS4) actually
     carries one, on the schema this contract ships
  4. the total route count matches baseline (88, per P0.23's --total) plus
     the five routes P2.8 added (POST /v1/operations, GET /v1/operations/{id},
     POST /v1/operations/{id}/cancel, GET /v1/capabilities,
     GET /v1/help/{helpId}) plus the thirteen P6.8 adds (docs/msc2/worlds/
     phase6-api.md SS3): POST /v1/worlds/{update,delete,duplicate,copy,
     import,export,rename-active-world,convert}, GET /v1/worlds/{slotId}/
     thumbnail, POST /v1/backups/delete, POST /v1/staged-uploads,
     PUT /v1/staged-uploads/{id}, GET /v1/staged-downloads/{id}, plus
     P6.34's POST /v1/worlds/replace-active-world, plus P7.9's
     POST /v1/java-runtimes/install (docs/msc2/families/phase7-api.md,
     D-006 addendum -- MSC 2 installs Java itself)

Also checks that every non-2xx response resolves to ErrorDTO (P2.4 SS5-6's
envelope unification), not the baseline's split Error/typed-DTO pattern.

Stdlib only, in the style of tools/api-baseline-check.py (P0.23).
"""

import argparse
import json
import sys

CONTRACT_PATH = "docs/msc2/api-contract/openapi.json"
EXPECTED_TOTAL = 126  # 88 baseline (P0.23 --total) + 5 P2.8 + 12 P6.8 (copyWorldSlotContent removed post-review; folded into /v1/worlds/replace) + 1 P6.34 (POST /v1/worlds/replace-active-world, replaceActiveWorld) + 1 P7.9 (POST /v1/java-runtimes/install, installJavaRuntime, D-006 addendum) + 3 P8.9 (POST /v1/modpacks/inspect, POST /v1/modpacks/import, POST /v1/modpacks/{operationId}/manual-file, docs/msc2/addons/phase8-api.md SS3) + 5 P11.21 (browser/desktop pairing and browser session routes) + 4 P11.24 (content catalog and structured guide routes) + 2 P11.29i (GET/POST /v1/config/servers-root, missed since Phase 11) + 4 P12.2g (player delete, offline-UUID migration, custom-UUID migration, and duplicate routes) + 1 P12.3d (POST /v1/players/identify, Bedrock gamertag assignment)

# helpid-contract.md SS4's table: schema -> field(s) that must carry helpId.
HELPID_FIELDS = {
    "SettingFieldDTO": ["helpId"],
    "HealthCardDTO": ["helpId"],
    "StartupProblemDTO": ["helpId"],
    "ConnectivityResponseDTO": ["helpId"],
    "PerformanceSnapshotDTO": ["tps1m", "cpuPercent", "ramUsedMB", "ramMaxMB", "worldSizeMB"],
}


def load(path=CONTRACT_PATH):
    with open(path) as f:
        return json.load(f)


def check_namespace(doc):
    bad = [p for p in doc.get("paths", {}) if not p.startswith("/v1/")]
    if bad:
        return 1, f"{len(bad)} path(s) not under /v1/: {bad[:5]}"
    return 0, f"ok {len(doc.get('paths', {}))}"


def check_permission_categories(doc):
    missing = []
    for path, methods in doc.get("paths", {}).items():
        for method, operation in methods.items():
            if "x-permission-category" not in operation:
                missing.append(f"{method.upper()} {path}")
    if missing:
        return 1, missing
    return 0, []


def check_error_dto(doc):
    """Every non-2xx response must resolve to #/components/schemas/ErrorDTO."""
    bad = []
    for path, methods in doc.get("paths", {}).items():
        for method, operation in methods.items():
            for status, response in operation.get("responses", {}).items():
                if status.startswith("2"):
                    continue
                schema = response.get("content", {}).get("application/json", {}).get("schema", {})
                if schema.get("$ref") != "#/components/schemas/ErrorDTO":
                    bad.append(f"{method.upper()} {path} {status}")
    if bad:
        return 1, bad
    return 0, []


def check_helpid_fields(doc):
    schemas = doc.get("components", {}).get("schemas", {})
    missing = []
    for schema_name, fields in HELPID_FIELDS.items():
        schema = schemas.get(schema_name)
        if schema is None:
            missing.append(f"{schema_name}: schema not found")
            continue
        props = schema.get("properties", {})
        for field in fields:
            if field not in props:
                missing.append(f"{schema_name}.{field}")
                continue
            # PerformanceSnapshotDTO's fields carry helpId one level down,
            # inside the {value, helpId} wrapper (helpid-contract.md SS4's
            # option b) rather than as a sibling field on the parent schema.
            if schema_name == "PerformanceSnapshotDTO":
                ref = props[field].get("$ref", "")
                metric_schema = schemas.get(ref.rsplit("/", 1)[-1], {})
                if "helpId" not in metric_schema.get("properties", {}):
                    missing.append(f"{schema_name}.{field} (wrapper schema missing helpId)")
    if missing:
        return 1, missing
    return 0, []


def total_operations(doc):
    return sum(len(methods) for methods in doc.get("paths", {}).values())


def v1_summary():
    doc = load()
    lines = []
    code = 0

    ns_code, ns_msg = check_namespace(doc)
    lines.append(f"namespace: {ns_msg}")
    code = code or ns_code

    cat_code, cat_missing = check_permission_categories(doc)
    lines.append(f"missing-category: {len(cat_missing)}")
    if cat_missing:
        lines.extend(f"  {m}" for m in cat_missing[:10])
    code = code or cat_code

    err_code, err_missing = check_error_dto(doc)
    lines.append(f"non-errordto-responses: {len(err_missing)}")
    if err_missing:
        lines.extend(f"  {m}" for m in err_missing[:10])
    code = code or err_code

    help_code, help_missing = check_helpid_fields(doc)
    lines.append(f"missing-helpid: {len(help_missing)}")
    if help_missing:
        lines.extend(f"  {m}" for m in help_missing)
    code = code or help_code

    n = total_operations(doc)
    lines.append(f"routes: {n}")
    if n != EXPECTED_TOTAL:
        lines.append(f"  expected {EXPECTED_TOTAL}")
        code = 1

    return code, lines


def selftest():
    """Two bundled fixtures (one clean, one violating every rule) so this
    script is checkable independent of the real openapi.json's contents."""
    clean = {
        "paths": {
            "/v1/x": {
                "get": {
                    "x-permission-category": "none",
                    "responses": {
                        "200": {"content": {"application/json": {"schema": {"type": "object"}}}},
                        "404": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/ErrorDTO"}}}},
                    },
                }
            }
        },
        "components": {"schemas": {
            "SettingFieldDTO": {"properties": {"helpId": {"type": "string"}}},
            "HealthCardDTO": {"properties": {"helpId": {"type": "string"}}},
            "StartupProblemDTO": {"properties": {"helpId": {"type": "string"}}},
            "ConnectivityResponseDTO": {"properties": {"helpId": {"type": "string"}}},
            "PerformanceSnapshotDTO": {"properties": {
                f: {"$ref": "#/components/schemas/M"} for f in HELPID_FIELDS["PerformanceSnapshotDTO"]
            }},
            "M": {"properties": {"value": {"type": "number"}, "helpId": {"type": "string"}}},
        }},
    }
    dirty = {
        "paths": {"/x": {"get": {"responses": {"404": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/Error"}}}}}}}},
        "components": {"schemas": {}},
    }

    def verdict(doc):
        checks = (
            check_namespace(doc)[0],
            check_permission_categories(doc)[0],
            check_error_dto(doc)[0],
            check_helpid_fields(doc)[0],
        )
        return 1 if any(checks) else 0

    pass_code = verdict(clean)
    fail_code = verdict(dirty)

    lines = [f"pass={pass_code}", f"fail={fail_code}"]
    ok = pass_code == 0 and fail_code != 0
    return (0 if ok else 1), lines


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--v1-summary", action="store_true")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        code, lines = selftest()
        for line in lines:
            print(line)
        sys.exit(code)

    if args.v1_summary or not any((args.v1_summary, args.selftest)):
        code, lines = v1_summary()
        out = sys.stdout if code == 0 else sys.stderr
        for line in lines:
            print(line, file=out)
        sys.exit(code)

    parser.print_help()
    sys.exit(2)


if __name__ == "__main__":
    main()
