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
import base64
import json
import os
import socket
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
import zipfile
from pathlib import Path
from urllib.parse import urlparse

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


def schema_ref(schema_name):
    return {"$ref": f"#/components/schemas/{schema_name}"}


def array_schema(item_schema_name):
    return {"type": "array", "items": schema_ref(item_schema_name)}


def run_checks(contract, base_url, token, selected_routes=None):
    """Returns a list of (route_name, passed, detail) tuples, one per route
    this phase's agent implements."""
    results = []
    selected = None
    if selected_routes is not None:
        selected = expand_route_selection(selected_routes)

    def check_schema(route_key, name, method, path, schema, expected_status, body=None):
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
            assert_conforms(contract, schema, payload, schema.get("$ref", route_key))
        except AssertionError as e:
            results.append((name, False, str(e)))
            return payload

        results.append((name, True, None))
        return payload

    def check(route_key, name, method, path, schema_name, expected_status, body=None):
        return check_schema(route_key, name, method, path, schema_ref(schema_name), expected_status, body)

    check("health", "GET /v1/health", "GET", "/v1/health", "HealthResponseDTO", 200)
    check("status", "GET /v1/status", "GET", "/v1/status", "RemoteAPIStatus", 200)
    check("performance", "GET /v1/performance", "GET", "/v1/performance", "PerformanceSnapshotDTO", 200)
    check("capabilities", "GET /v1/capabilities", "GET", "/v1/capabilities", "CapabilitiesDTO", 200)
    check_phase4_lifecycle_routes(contract, base_url, token, selected, results, check, check_schema)

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


def expand_route_selection(selected_routes):
    selected = {route.strip() for route in selected_routes.split(",") if route.strip()}
    if "phase4-lifecycle" not in selected:
        return selected
    selected.remove("phase4-lifecycle")
    selected.update(
        {
            "servers",
            "servers-import",
            "active-server",
            "start",
            "stop",
            "command",
            "status",
            "performance",
            "console-tail",
            "console-ws",
        }
    )
    return selected


def check_phase4_lifecycle_routes(contract, base_url, token, selected, results, check, check_schema):
    phase4_keys = {
        "servers",
        "servers-import",
        "active-server",
        "start",
        "stop",
        "command",
        "console-tail",
        "console-ws",
    }
    if selected is not None and not (selected & phase4_keys):
        return

    check_schema("servers", "GET /v1/servers", "GET", "/v1/servers", array_schema("ServerDTO"), 200)

    with tempfile.TemporaryDirectory(prefix="msc2-phase4-lifecycle-") as tmp:
        server_dir = Path(tmp) / "paper"
        server_dir.mkdir()
        build_fake_paper_jar(server_dir / "paper.jar")
        (server_dir / "eula.txt").write_text("eula=true\n")
        (server_dir / "server.properties").write_text(
            "server-port=25565\nmax-players=20\nlevel-name=world\n"
        )

        imported = check(
            "servers-import",
            "POST /v1/servers/import",
            "POST",
            "/v1/servers/import",
            "ServerImportResultDTO",
            200,
            body={
                "action": "importExisting",
                "sourcePath": str(server_dir),
                "displayName": "Contract Paper",
                "serverType": "java",
                "importKind": "paper",
            },
        )
        server_id = imported.get("serverId") if isinstance(imported, dict) else None
        if not server_id:
            results.append(("POST /v1/active-server", False, "no serverId from import response"))
            results.append(("POST /v1/start", False, "no serverId from import response"))
            results.append(("POST /v1/command", False, "no serverId from import response"))
            results.append(("POST /v1/stop", False, "no serverId from import response"))
            return

        check(
            "active-server",
            "POST /v1/active-server",
            "POST",
            "/v1/active-server",
            "SimpleResult",
            200,
            body={"serverId": server_id},
        )
        check("start", "POST /v1/start", "POST", "/v1/start", "SimpleResult", 200)
        wait_for_running_status(base_url, token, contract, results)
        check(
            "command",
            "POST /v1/command",
            "POST",
            "/v1/command",
            "CommandResult",
            200,
            body={"command": "say contract-check"},
        )
        check_schema(
            "console-tail",
            "GET /v1/console/tail",
            "GET",
            "/v1/console/tail?n=20",
            array_schema("ConsoleLineDTO"),
            200,
        )
        check_console_websocket(base_url, token, results)
        check("stop", "POST /v1/stop", "POST", "/v1/stop", "SimpleResult", 200)


def build_fake_paper_jar(jar_path):
    source = jar_path.with_name("FakePaper.java")
    source.write_text(
        """
import java.io.BufferedReader;
import java.io.InputStreamReader;

public class FakePaper {
    public static void main(String[] args) throws Exception {
        System.out.println("Done (0.001s)! For help, type \\"help\\"");
        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = reader.readLine()) != null) {
            System.out.println("command:" + line);
            if (line.trim().equals("stop")) {
                break;
            }
        }
    }
}
""".strip()
    )
    subprocess.run(["javac", str(source)], check=True, cwd=jar_path.parent)
    with zipfile.ZipFile(jar_path, "w") as jar:
        jar.writestr("META-INF/MANIFEST.MF", "Manifest-Version: 1.0\nMain-Class: FakePaper\n\n")
        jar.write(jar_path.with_name("FakePaper.class"), "FakePaper.class")


def wait_for_running_status(base_url, token, contract, results):
    for _ in range(50):
        try:
            status, payload = request(base_url, "GET", "/v1/status", token)
            assert status == 200
            assert_conforms(contract, schema_ref("RemoteAPIStatus"), payload, "RemoteAPIStatus")
            if payload.get("running") and payload.get("pid"):
                return
        except Exception:
            pass
        import time

        time.sleep(0.1)
    results.append(("GET /v1/status after start", False, "server did not report a running pid"))


def check_console_websocket(base_url, token, results):
    parsed = urlparse(base_url)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or (443 if parsed.scheme == "https" else 80)
    path_prefix = parsed.path.rstrip("/")
    key = base64.b64encode(os.urandom(16)).decode()
    request_text = (
        f"GET {path_prefix}/v1/console/stream HTTP/1.1\r\n"
        f"Host: {host}:{port}\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        "Sec-WebSocket-Version: 13\r\n"
        f"Authorization: Bearer {token}\r\n"
        "\r\n"
    )
    try:
        with socket.create_connection((host, port), timeout=5) as sock:
            sock.sendall(request_text.encode())
            response = sock.recv(1024).decode(errors="replace")
    except OSError as e:
        results.append(("GET /v1/console/stream", False, f"connection error: {e}"))
        return
    if response.startswith("HTTP/1.1 101") or response.startswith("HTTP/1.0 101"):
        results.append(("GET /v1/console/stream", True, None))
    else:
        results.append(("GET /v1/console/stream", False, response.split("\r\n", 1)[0]))


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


def phase6_example_instances():
    """One hand-built representative instance per P6.20 world/backup/
    staged-transfer schema -- kept conceptually (not byte-for-byte) in
    sync with `crates/msc-api/tests/world_backup_conformance.rs`'s own
    examples, the Rust-side half of this same round-trip idea."""
    slot = {
        "id": "11111111-1111-1111-1111-111111111111",
        "name": "Survival",
        "isActive": True,
        "createdAt": "2026-08-15T12:00:00Z",
        "zipSizeBytes": 123456,
        "worldSeed": "42",
        "hasThumbnail": True,
    }
    backup_item = {
        "id": "world-manual-20260815-120000.zip",
        "displayName": "world (manual) - Aug 15, 2026 12:00 PM",
        "fileSize": 654321,
        "modificationDate": "2026-08-15T12:00:00Z",
        "isAutomatic": False,
        "slotId": "11111111-1111-1111-1111-111111111111",
        "slotName": "Survival",
        "triggerReason": "manual",
    }
    backup_config = {
        "serverName": "Survival",
        "autoBackupEnabled": True,
        "autoBackupIntervalMinutes": 30,
        "autoBackupMaxCount": 10,
        "intervalOptions": [15, 30, 60, 120],
    }
    return {
        "WorldSlotDTO": slot,
        "WorldSlotsResponseDTO": {
            "slots": [slot],
            "activeSlotId": slot["id"],
            "serverRunning": False,
            "isRepairing": False,
        },
        "WorldCreateRequestDTO": {"name": "New World", "seed": "1234"},
        "WorldRenameRequestDTO": {"slotId": "slot-1", "name": "Renamed"},
        "WorldReplaceRequestDTO": {"slotId": "slot-1", "sourceSlotId": "slot-2"},
        "WorldRepairRequestDTO": {"slotId": "slot-1"},
        "WorldActivateRequestDTO": {"slotId": "slot-1"},
        "WorldActivateResultDTO": {"result": "activation_started", "operationId": "op-1"},
        "WorldMutationResultDTO": {"success": True, "message": "saved"},
        "WorldDeleteRequestDTO": {"slotId": "slot-1"},
        "WorldDuplicateRequestDTO": {"slotId": "slot-1"},
        "WorldImportRequestDTO": {"name": "Imported", "stagedUploadId": "upload-1"},
        "WorldExportRequestDTO": {"slotId": "slot-1"},
        "WorldExportResultDTO": {
            "stagedDownloadId": "download-1",
            "expiresAt": "2026-08-15T12:30:00Z",
            "sizeBytes": 42,
        },
        "WorldRenameActiveWorldRequestDTO": {"name": "new-world-name"},
        "WorldReplaceActiveRequestDTO": {"newLevelName": "restored-world", "stagedUploadId": "upload-1"},
        "WorldReplaceActiveResultDTO": {"result": "replace_started", "operationId": "op-5"},
        "WorldConvertRequestDTO": {
            "sourceSlotId": "slot-1",
            "targetServerId": "server-2",
            "targetFormat": "JAVA_1_21_4",
            "targetName": "Converted",
        },
        "WorldConvertResultDTO": {"result": "conversion_started", "operationId": "op-2"},
        "BackupItemDTO": backup_item,
        "BackupsResponseDTO": {"backups": [backup_item]},
        "BackupConfigResponseDTO": backup_config,
        "BackupConfigUpdateRequestDTO": {"autoBackupEnabled": True, "autoBackupIntervalMinutes": 60},
        "BackupConfigUpdateResultDTO": {"success": True, "message": "saved", "config": backup_config},
        "BackupNowResultDTO": {"result": "backup_started", "operationId": "op-3"},
        "BackupRestoreRequestDTO": {"backupId": "world-manual-20260815-120000.zip"},
        "BackupRestoreResultDTO": {"result": "restore_started", "operationId": "op-4"},
        "BackupDeleteRequestDTO": {"backupId": "world-manual-20260815-120000.zip"},
        "StagedUploadBeginRequestDTO": {"purpose": "world-import", "contentType": "application/zip"},
        "StagedUploadBeginResultDTO": {
            "stagedUploadId": "upload-1",
            "uploadPath": "/v1/staged-uploads/upload-1",
            "expiresAt": "2026-08-15T12:30:00Z",
            "maxBytes": 10737418240,
        },
        "StagedUploadCompleteResultDTO": {
            "stagedUploadId": "upload-1",
            "receivedBytes": 4096,
            "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        },
    }


PHASE6_PATHS = [
    ("get", "/v1/worlds"),
    ("post", "/v1/worlds/create"),
    ("post", "/v1/worlds/rename"),
    ("post", "/v1/worlds/replace"),
    ("post", "/v1/worlds/repair"),
    ("post", "/v1/worlds/update"),
    ("post", "/v1/worlds/delete"),
    ("post", "/v1/worlds/duplicate"),
    ("post", "/v1/worlds/import"),
    ("post", "/v1/worlds/export"),
    ("post", "/v1/worlds/rename-active-world"),
    ("post", "/v1/worlds/replace-active-world"),
    ("post", "/v1/worlds/activate"),
    ("post", "/v1/worlds/convert"),
    ("get", "/v1/worlds/{slotId}/thumbnail"),
    ("get", "/v1/backups"),
    ("get", "/v1/backups/config"),
    ("post", "/v1/backups/config"),
    ("post", "/v1/backups/now"),
    ("post", "/v1/backups/restore"),
    ("post", "/v1/backups/delete"),
    ("post", "/v1/staged-uploads"),
    ("put", "/v1/staged-uploads/{id}"),
    ("get", "/v1/staged-downloads/{id}"),
]


def phase6_check():
    """A self-contained (no live server) conformance check for the P6.20/
    P6.21 world/backup/staged-transfer surface, in the same spirit as
    `--selftest`: `tools/contract-conformance-check.py --phase6` (the
    plan's own P6.21 Verify command) names no `--base-url`/`--token`,
    unlike every other live-server check in this file, so it can't be a
    live-server check the way `run_checks` is -- this is this step's own
    documented interpretation of that otherwise-unrunnable Verify line
    (see the P6.21 report for the reasoning).

    Checks, for the full P6.8 world/backup/staged-* surface: (a) every
    `(method, path)` in `PHASE6_PATHS` exists in `openapi.json`, (b)
    every `$ref` its request/response schemas name resolves to a real
    `components/schemas` entry, (c) every schema's `required` fields are
    all declared in its own `properties`, and (d) a hand-built
    representative instance per schema (`phase6_example_instances`)
    passes `assert_conforms` against that schema."""
    contract = load_contract()
    paths = contract["paths"]
    schemas = contract["components"]["schemas"]
    failures = []
    checked = 0

    def schema_refs_in(node):
        if isinstance(node, dict):
            ref = node.get("$ref")
            if ref is not None:
                yield ref
            for value in node.values():
                yield from schema_refs_in(value)
        elif isinstance(node, list):
            for item in node:
                yield from schema_refs_in(item)

    for method, path in PHASE6_PATHS:
        checked += 1
        if path not in paths or method not in paths[path]:
            failures.append(f"{method.upper()} {path}: missing from openapi.json")
            continue
        for ref in schema_refs_in(paths[path][method]):
            name = ref.rsplit("/", 1)[-1]
            if name not in schemas:
                failures.append(f"{method.upper()} {path}: $ref '{ref}' does not resolve")

    for name, schema in schemas.items():
        if not (name.startswith("World") or name.startswith("Backup") or name.startswith("StagedUpload")):
            continue
        checked += 1
        properties = schema.get("properties", {})
        for field in schema.get("required", []):
            if field not in properties:
                failures.append(f"{name}: required field '{field}' is not declared in properties")

    examples = phase6_example_instances()
    for name, instance in examples.items():
        checked += 1
        if name not in schemas:
            failures.append(f"{name}: no such schema in openapi.json")
            continue
        try:
            assert_conforms(contract, schema_ref(name), instance, name)
        except AssertionError as e:
            failures.append(str(e))

    missing_examples = set(schemas) - set(examples)
    missing_examples = {
        n for n in missing_examples if n.startswith("World") or n.startswith("Backup") or n.startswith("StagedUpload")
    }
    for name in sorted(missing_examples):
        failures.append(f"{name}: no example instance in phase6_example_instances()")

    if failures:
        for failure in failures:
            print(f"FAIL {failure}", file=sys.stderr)
        return 1

    print(f"ok phase6 {checked}")
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
    parser.add_argument("--phase6", action="store_true", help="P6.21: self-contained world/backup/staged-transfer schema check, no live server needed")
    args = parser.parse_args()

    if args.selftest:
        sys.exit(selftest())

    if args.phase6:
        sys.exit(phase6_check())

    if args.expect_auth_store:
        if not args.base_url:
            parser.error("--base-url is required with --expect-auth-store")
        sys.exit(expect_auth_store_check(args.base_url))

    if not args.base_url or not args.token:
        parser.error("--base-url and --token are required unless --selftest or --expect-auth-store")

    sys.exit(live_check(args.base_url, args.token, args.routes))


if __name__ == "__main__":
    main()
