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
import os
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
    elif schema_type == "number":
        if not isinstance(instance, (int, float)) or isinstance(instance, bool):
            raise AssertionError(f"{path}: expected number, got {instance!r}")
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


def request_expect_http_error(base_url, method, path, token, expected_status, body=None):
    url = base_url.rstrip("/") + path
    headers = {"Authorization": f"Bearer {token}"}
    data = None
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            payload = resp.read().decode(errors="replace")
            raise AssertionError(f"expected HTTP {expected_status}, got {resp.status}: {payload}")
    except urllib.error.HTTPError as e:
        payload = json.loads(e.read())
        if e.code != expected_status:
            raise AssertionError(f"expected HTTP {expected_status}, got {e.code}: {payload}")
        return payload


def run_checks(contract, base_url, token, selected_routes=None):
    """Returns a list of (route_name, passed, detail) tuples, one per route
    this phase's agent implements."""
    results = []
    selected = None
    if selected_routes is not None:
        selected = {route.strip() for route in selected_routes.split(",") if route.strip()}

    def check(route_key, name, method, path, schema_name, expected_status, body=None):
        if selected is not None and route_key not in selected:
            return None
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

    check("health", "GET /v1/health", "GET", "/v1/health", "HealthResponseDTO", 200)
    check("status", "GET /v1/status", "GET", "/v1/status", "RemoteAPIStatus", 200)
    check("performance", "GET /v1/performance", "GET", "/v1/performance", "PerformanceSnapshotDTO", 200)
    check("capabilities", "GET /v1/capabilities", "GET", "/v1/capabilities", "CapabilitiesDTO", 200)

    if selected is not None and "operations" not in selected:
        return results

    created = check(
        "operations",
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
        check("operations", "GET /v1/operations/{id}", "GET", f"/v1/operations/{op_id}", "OperationDTO", 200)
        check(
            "operations",
            "POST /v1/operations/{id}/cancel",
            "POST",
            f"/v1/operations/{op_id}/cancel",
            "OperationDTO",
            200,
        )

    return results


def live_check(base_url, token, routes):
    contract = load_contract()
    results = run_checks(contract, base_url, token, routes)
    failed = [(name, detail) for name, passed, detail in results if not passed]

    if failed:
        for name, detail in failed:
            print(f"FAIL {name}: {detail}", file=sys.stderr)
        return 1

    print(f"ok {len(results)}")
    return 0


def expect_auth_store_check(base_url):
    """P4.5 live check: the old MSC_DEV_TOKEN value must not authorize
    protected routes. Token-backed success is covered by msc-agent's
    auth_real_tokens tests, because a running service no longer exposes a
    fixed dev-token shortcut to mint credentials."""
    contract = load_contract()
    dev_token = os.environ.get("MSC_DEV_TOKEN") or "msc2-dev-token"
    checks = [
        ("GET /v1/status", "GET", "/v1/status", None),
        ("GET /v1/capabilities", "GET", "/v1/capabilities", None),
        ("POST /v1/operations", "POST", "/v1/operations", {"type": "demo-install"}),
    ]
    for name, method, path, body in checks:
        try:
            payload = request_expect_http_error(base_url, method, path, dev_token, 401, body=body)
            assert_conforms(contract, {"$ref": "#/components/schemas/ErrorDTO"}, payload, "ErrorDTO")
            if payload.get("code") != "unauthorized":
                raise AssertionError(f"{name}: expected unauthorized error, got {payload}")
        except (urllib.error.URLError, TimeoutError) as e:
            print(f"FAIL {name}: connection error: {e}", file=sys.stderr)
            return 1
        except (AssertionError, json.JSONDecodeError) as e:
            print(f"FAIL {name}: {e}", file=sys.stderr)
            return 1

    print(f"ok auth-store {len(checks)}")
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
    parser.add_argument("--token", help="bearer token for full live route conformance")
    parser.add_argument("--expect-auth-store", action="store_true", help="P4.5: assert MSC_DEV_TOKEN no longer authorizes protected routes")
    parser.add_argument("--routes", help="comma-separated route keys, e.g. status,performance")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        sys.exit(selftest())

    if args.expect_auth_store:
        if not args.base_url:
            parser.error("--base-url is required with --expect-auth-store")
        sys.exit(expect_auth_store_check(args.base_url))

    if not args.base_url or not args.token:
        parser.error("--base-url and --token are required unless --selftest or --expect-auth-store")

    sys.exit(live_check(args.base_url, args.token, args.routes))


if __name__ == "__main__":
    main()
