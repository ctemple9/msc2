#!/usr/bin/env python3
"""Live contract-conformance checker for the skeletal `msc-agent` (P2.17).

Calls every route this phase's agent implements (health, status,
capabilities, and the three operation-lifecycle routes) against a running
`msc-agent` and validates each live JSON response against
`docs/msc2/api-contract/openapi.json`'s schema for that route --
P0.23/P2.11's schema-depth discipline (every required field present, every
present field's declared type/enum matches, $ref/items/properties followed
recursively so nested content is checked too), now pointed at a live
server's actual output instead of a static document or a hand-written Rust
example. The depth-check logic itself is a direct Python port of
crates/msc-api/tests/dto_conformance.rs's `assert_conforms`.

Does not check the two WebSocket channels (console, operation-progress) --
those aren't JSON request/response routes this checker's model fits, and
P2.15's own Verify already exercises the one that exists so far.

Stdlib only, in the style of tools/api-baseline-check.py (P0.23) and
tools/api-contract-check.py (P2.8).
"""

import argparse
import json
import sys
import urllib.error
import urllib.request

CONTRACT_PATH = "docs/msc2/api-contract/openapi.json"


def load_contract():
    with open(CONTRACT_PATH) as f:
        return json.load(f)


def resolve(contract, schema):
    ref = schema.get("$ref")
    if ref is not None:
        name = ref.rsplit("/", 1)[-1]
        return contract["components"]["schemas"][name]
    return schema


def assert_conforms(contract, schema, instance, path):
    """A depth-check, not a full JSON Schema validator -- see
    dto_conformance.rs's docstring for the exact contract this mirrors."""
    schema = resolve(contract, schema)

    if instance is None:
        if not schema.get("nullable", False):
            raise AssertionError(f"{path}: null not allowed by schema {schema}")
        return

    enum_values = schema.get("enum")
    if enum_values is not None and instance not in enum_values:
        raise AssertionError(f"{path}: {instance!r} not one of {enum_values}")

    schema_type = schema.get("type")
    if schema_type == "object":
        if not isinstance(instance, dict):
            raise AssertionError(f"{path}: expected object, got {instance!r}")
        for field in schema.get("required", []):
            if field not in instance:
                raise AssertionError(f"{path}: missing required field '{field}'")
        properties = schema.get("properties", {})
        for key, value in instance.items():
            prop_schema = properties.get(key)
            if prop_schema is not None:
                assert_conforms(contract, prop_schema, value, f"{path}.{key}")
    elif schema_type == "array":
        if not isinstance(instance, list):
            raise AssertionError(f"{path}: expected array, got {instance!r}")
        items = schema.get("items")
        if items is not None:
            for i, item in enumerate(instance):
                assert_conforms(contract, items, item, f"{path}[{i}]")
    elif schema_type == "string":
        if not isinstance(instance, str):
            raise AssertionError(f"{path}: expected string, got {instance!r}")
    elif schema_type == "integer":
        if not isinstance(instance, int) or isinstance(instance, bool):
            raise AssertionError(f"{path}: expected integer, got {instance!r}")
    elif schema_type == "boolean":
        if not isinstance(instance, bool):
            raise AssertionError(f"{path}: expected boolean, got {instance!r}")
    elif schema_type is not None:
        raise AssertionError(f"{path}: unhandled schema type '{schema_type}'")
    # else: enum-only/untyped schema -- the enum check above already ran


def request(base_url, method, path, token, body=None):
    url = base_url.rstrip("/") + path
    headers = {"Authorization": f"Bearer {token}"}
    data = None
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    with urllib.request.urlopen(req, timeout=5) as resp:
        return resp.status, json.loads(resp.read())


def run_checks(contract, base_url, token):
    """Returns a list of (route_name, passed, detail) tuples, one per route
    this phase's agent implements."""
    results = []

    def check(name, method, path, schema_name, expected_status, body=None):
        try:
            status, payload = request(base_url, method, path, token, body)
        except urllib.error.HTTPError as e:
            results.append((name, False, f"HTTP {e.code}: {e.read().decode(errors='replace')}"))
            return None
        except (urllib.error.URLError, TimeoutError) as e:
            results.append((name, False, f"connection error: {e}"))
            return None

        if status != expected_status:
            results.append((name, False, f"expected status {expected_status}, got {status}"))
            return payload

        try:
            assert_conforms(contract, {"$ref": f"#/components/schemas/{schema_name}"}, payload, schema_name)
        except AssertionError as e:
            results.append((name, False, str(e)))
            return payload

        results.append((name, True, None))
        return payload

    check("GET /v1/health", "GET", "/v1/health", "HealthResponseDTO", 200)
    check("GET /v1/status", "GET", "/v1/status", "RemoteAPIStatus", 200)
    check("GET /v1/capabilities", "GET", "/v1/capabilities", "CapabilitiesDTO", 200)

    created = check(
        "POST /v1/operations",
        "POST",
        "/v1/operations",
        "OperationDTO",
        202,
        body={"type": "demo-install"},
    )
    op_id = created.get("id") if isinstance(created, dict) else None
    if op_id is None:
        results.append(("GET /v1/operations/{id}", False, "no operation id from create response"))
        results.append(("POST /v1/operations/{id}/cancel", False, "no operation id from create response"))
    else:
        check("GET /v1/operations/{id}", "GET", f"/v1/operations/{op_id}", "OperationDTO", 200)
        check(
            "POST /v1/operations/{id}/cancel",
            "POST",
            f"/v1/operations/{op_id}/cancel",
            "OperationDTO",
            200,
        )

    return results


def live_check(base_url, token):
    contract = load_contract()
    results = run_checks(contract, base_url, token)
    failed = [(name, detail) for name, passed, detail in results if not passed]

    if failed:
        for name, detail in failed:
            print(f"FAIL {name}: {detail}", file=sys.stderr)
        return 1

    print(f"ok {len(results)}")
    return 0


def selftest():
    """A tiny embedded schema/instance pair, independent of the real
    openapi.json or any live server -- checkable on its own, same principle
    as P0.23's --selftest."""
    contract = {
        "components": {
            "schemas": {
                "Widget": {
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": {"type": "string"},
                        "count": {"type": "integer"},
                    },
                }
            }
        }
    }
    schema = {"$ref": "#/components/schemas/Widget"}

    def verdict(instance):
        try:
            assert_conforms(contract, schema, instance, "Widget")
            return 0
        except AssertionError:
            return 1

    pass_code = verdict({"name": "a", "count": 1})
    fail_code = verdict({"count": "not-an-int"})
    print(f"pass={pass_code}")
    print(f"fail={fail_code}")
    return 0 if pass_code == 0 and fail_code != 0 else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base-url", help="e.g. http://127.0.0.1:48400")
    parser.add_argument("--token", help="dev bearer token (MSC_DEV_TOKEN)")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        sys.exit(selftest())

    if not args.base_url or not args.token:
        parser.error("--base-url and --token are required unless --selftest")

    sys.exit(live_check(args.base_url, args.token))


if __name__ == "__main__":
    main()
