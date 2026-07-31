#!/usr/bin/env python3
"""API baseline schema-depth checker (P0.23).

Used as the Verify command for every P0.23a-P0.23s route-family step in
docs/msc2/rolling-plan.md. Checks two things a flat path count can't:
  1. only the paths that actually belong to a given route family are counted
  2. every matched operation's `responses` nests down to a real
     content -> application/json -> schema, not a stub {}

Five modes:
  api-baseline-check.py <family>   check one route family, print `ok <n>`
  api-baseline-check.py --total    sum every path in the file (P0.23s's
                                    final sanity check against 87)
  api-baseline-check.py --depth-all check every operation in the whole file
                                    for real schema depth, not just one
                                    family (P0.30's Verify, against 88)
  api-baseline-check.py --typed-failures  check that the (path, method,
                                    status) pairs P0.32 corrected are still
                                    the route's typed result DTO, not the
                                    generic Error schema
  api-baseline-check.py --selftest exercise the checker against two bundled
                                    fixtures (one deep, one stub) so it's
                                    checkable before openapi.json exists

Stdlib only, on purpose: Phase 0 has no Cargo.toml and no dependency setup
for Cameron to fight, so this has none either.
"""

import argparse
import json
import sys

OPENAPI_PATH = "docs/msc2/api-baseline/openapi.json"

# Families whose sub-route count MSC 1's own route-family list
# (msc2-engineering.md §5) gives exactly. The five `*`-wildcard families
# (playit, broadcast, resourcepacks, watchdog, players) are deliberately
# absent here -- for those, only "count > 0" is checked, since MSC 1's docs
# don't state a sub-route count and asserting a guessed one would repeat
# the exact problem P0.25/P0.27 were fixed to avoid.
KNOWN_COUNTS = {
    # servers/settings/worlds/components/backups/health bumped by P0.30, which
    # added bare-resource GET routes (and, for health, GET /health itself and
    # GET /health/problems) that share the same family prefix as routes these
    # counts already covered. See the P0.30 Amendments log entry.
    "servers": 6,
    "settings": 1,
    "worlds": 6,
    "components": 6,
    "backups": 4,
    "config": 3,
    "users": 3,
    "health": 3,
    "command": 1,
    "start": 1,
    "stop": 1,
    "allowlist": 1,
    "duckdns": 1,
    "templates": 1,
}
WILDCARD_FAMILIES = {"playit", "broadcast", "resourcepacks", "watchdog", "players"}

# P0.32: (path, method, status) pairs that must resolve to the route's own
# typed result DTO, not the generic Error schema. MSC 1's handlers send the
# same encodable(result) for every post-provider status code (win or lose);
# only the synchronous pre-provider guards (missing_body, invalid_json, and
# field-required checks before the async Task) are genuinely {"error": ...}.
# Verified per-route against RemoteAPIServer+ComponentRoutes.swift,
# +UserRoutes.swift, and the /allowlist case in +HTTP.swift -- not guessed.
TYPED_FAILURES = [
    ("/allowlist", "post", "409"),
    ("/allowlist", "post", "500"),
    ("/components/install", "post", "409"),
    ("/components/remove", "post", "404"),
    ("/components/remove", "post", "409"),
    ("/components/remove", "post", "500"),
    ("/components/update", "post", "404"),
    ("/components/update", "post", "409"),
    ("/components/update", "post", "500"),
    ("/components/version", "post", "409"),
    ("/components/version", "post", "429"),
    ("/components/version", "post", "500"),
    ("/config/geyser", "post", "409"),
    ("/config/geyser", "post", "500"),
    ("/config/ram", "post", "409"),
    ("/config/ram", "post", "500"),
    ("/health/repair", "post", "404"),
    ("/health/repair", "post", "409"),
    ("/health/repair", "post", "500"),
    ("/players/hidden", "post", "404"),
    ("/players/hidden", "post", "409"),
    ("/players/hidden", "post", "500"),
    ("/players/skin-override", "post", "404"),
    ("/players/skin-override", "post", "409"),
    ("/players/skin-override", "post", "500"),
    ("/resourcepacks/activate", "post", "409"),
    ("/resourcepacks/activate", "post", "500"),
    ("/resourcepacks/remove", "post", "404"),
    ("/resourcepacks/remove", "post", "500"),
    ("/resourcepacks/seturl", "post", "409"),
    ("/resourcepacks/seturl", "post", "500"),
    ("/resourcepacks/toggle", "post", "404"),
    ("/resourcepacks/toggle", "post", "500"),
    ("/servers/create", "post", "409"),
    ("/servers/create", "post", "500"),
    ("/servers/delete", "post", "404"),
    ("/servers/delete", "post", "409"),
    ("/servers/delete", "post", "500"),
    ("/servers/eula", "post", "404"),
    ("/servers/eula", "post", "500"),
    ("/servers/import", "post", "400"),
    ("/servers/import", "post", "404"),
    ("/servers/import", "post", "409"),
    ("/servers/import", "post", "500"),
    ("/servers/rename", "post", "404"),
    ("/servers/rename", "post", "409"),
    ("/servers/rename", "post", "500"),
    ("/templates", "post", "400"),
    ("/templates", "post", "404"),
    ("/templates", "post", "409"),
    ("/templates", "post", "500"),
    ("/users", "post", "422"),
    ("/users/revoke", "post", "404"),
    ("/users/revoke", "post", "500"),
    ("/users/update", "post", "404"),
    ("/users/update", "post", "422"),
    ("/worlds/create", "post", "404"),
    ("/worlds/create", "post", "409"),
    ("/worlds/create", "post", "500"),
    ("/worlds/rename", "post", "404"),
    ("/worlds/rename", "post", "409"),
    ("/worlds/rename", "post", "500"),
    ("/worlds/repair", "post", "404"),
    ("/worlds/repair", "post", "409"),
    ("/worlds/repair", "post", "500"),
    ("/worlds/replace", "post", "404"),
    ("/worlds/replace", "post", "409"),
    ("/worlds/replace", "post", "500"),
]


def load_openapi(path=OPENAPI_PATH):
    with open(path) as f:
        return json.load(f)


def matched_paths(doc, family):
    prefix = "/" + family
    return {
        p: methods
        for p, methods in doc.get("paths", {}).items()
        if p == prefix or p.startswith(prefix + "/")
    }


def has_real_schema(operation):
    """True if at least one response nests down to a real JSON schema,
    rather than an empty stub like {"200": {}}."""
    for response in operation.get("responses", {}).values():
        schema = response.get("content", {}).get("application/json", {}).get("schema")
        if schema:
            return True
    return False


def check_family(doc, family, expect):
    """Returns (exit_code, message). `expect` is an int for known families,
    or None for wildcard families (only checks count > 0)."""
    paths = matched_paths(doc, family)
    n = len(paths)

    if expect is not None:
        if n != expect:
            return 1, f"{family}: found {n} path(s), expected {expect}"
    elif n == 0:
        return 1, f"{family}: found 0 paths, expected > 0"

    for path, methods in paths.items():
        for method, operation in methods.items():
            if not has_real_schema(operation):
                return 1, (
                    f"{family}: {method.upper()} {path} has no real "
                    "content/application/json/schema (stub response?)"
                )

    return 0, f"ok {n}"


def total_count(doc):
    return sum(len(methods) for methods in doc.get("paths", {}).values())


def check_typed_failures(doc):
    """P0.32: every (path, method, status) in TYPED_FAILURES must NOT resolve
    to the generic Error schema. Returns (exit_code, message)."""
    for path, method, status in TYPED_FAILURES:
        operation = doc.get("paths", {}).get(path, {}).get(method)
        if operation is None:
            return 1, f"{method.upper()} {path} not found in openapi.json"
        response = operation.get("responses", {}).get(status)
        if response is None:
            return 1, f"{method.upper()} {path} has no {status} response"
        schema = response.get("content", {}).get("application/json", {}).get("schema", {})
        ref = schema.get("$ref", "")
        if ref.endswith("/Error"):
            return 1, f"{method.upper()} {path} {status} is still the generic Error schema"
    return 0, f"ok {len(TYPED_FAILURES)}"


def check_all_depth(doc):
    """P0.30: whole-document version of check_family's schema-depth assertion --
    every operation in the file, not just one family, must have a real
    content -> application/json -> schema. Returns (exit_code, message)."""
    n = 0
    for path, methods in doc.get("paths", {}).items():
        for method, operation in methods.items():
            n += 1
            if not has_real_schema(operation):
                return 1, (
                    f"{method.upper()} {path} has no real "
                    "content/application/json/schema (stub response?)"
                )
    return 0, f"ok {n}"


def selftest():
    """Returns (exit_code, [line, line]) -- the exit code is the selftest's
    own verdict; the lines are what P0.23's Verify line expects to see."""
    deep_doc = {
        "paths": {
            "/deep": {
                "get": {
                    "responses": {
                        "200": {"content": {"application/json": {"schema": {"type": "object"}}}}
                    }
                }
            }
        }
    }
    shallow_doc = {"paths": {"/shallow": {"get": {"responses": {"200": {}}}}}}

    pass_code, _ = check_family(deep_doc, "deep", expect=1)
    fail_code, _ = check_family(shallow_doc, "shallow", expect=1)
    lines = [f"pass={pass_code}", f"fail={fail_code}"]
    ok = pass_code == 0 and fail_code != 0
    return (0 if ok else 1), lines


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("family", nargs="?", help="route family to check, e.g. 'backups'")
    parser.add_argument("--total", action="store_true")
    parser.add_argument("--depth-all", action="store_true")
    parser.add_argument("--typed-failures", action="store_true")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        code, lines = selftest()
        for line in lines:
            print(line)
        sys.exit(code)

    if args.total:
        doc = load_openapi()
        print(f"total {total_count(doc)}")
        sys.exit(0)

    if args.depth_all:
        doc = load_openapi()
        code, message = check_all_depth(doc)
        if code == 0:
            print(message)
        else:
            print(message, file=sys.stderr)
        sys.exit(code)

    if args.typed_failures:
        doc = load_openapi()
        code, message = check_typed_failures(doc)
        if code == 0:
            print(message)
        else:
            print(message, file=sys.stderr)
        sys.exit(code)

    if args.family:
        if args.family not in KNOWN_COUNTS and args.family not in WILDCARD_FAMILIES:
            parser.error(f"unknown family '{args.family}'")
        expect = KNOWN_COUNTS.get(args.family)
        doc = load_openapi()
        code, message = check_family(doc, args.family, expect)
        if code == 0:
            print(message)
        else:
            print(message, file=sys.stderr)
        sys.exit(code)

    parser.print_help()
    sys.exit(2)


if __name__ == "__main__":
    main()
