#!/usr/bin/env python3
"""Fixture runner and comparison tool (P0.2).

Validates a fixture against the schema in docs/msc2/fixture-format.md
(reproduced machine-readably in schema.json alongside this file) and,
where an `actual` value can be computed, compares it to `expected`.

Four modes, matching docs/msc2/rolling-plan.md P0.2:
  run.py <file>                          full compare (from Phase 1 on)
  run.py --schema-only <file>             shape check only, no actual needed
  run.py --validate-dir <dir> --expect N  schema-only over a whole directory,
                                           plus a file-count assertion
  run.py --selftest                       exercises both self-test fixtures

Stdlib only, on purpose: Phase 0 has no Cargo.toml and no dependency
setup for Cameron to fight, so this has none either.
"""

import argparse
import json
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
SCHEMA_PATH = SCRIPT_DIR / "schema.json"

# Domains with a known way to compute `actual` from `input`, for full-compare
# mode. Real domains gain an entry here once their Rust port exists — until
# then, full-compare on them is a "not implemented yet" error, which is
# expected in Phase 0. `_selftest` is the one domain that's deliberately
# trivial: it exists only to prove this script's own comparison logic works.
ACTUAL_COMPUTERS = {
    "_selftest": lambda fixture_input: fixture_input,
}


def load_schema():
    with open(SCHEMA_PATH) as f:
        return json.load(f)


def check_type(value, expected_type, where):
    if expected_type == "object":
        if not isinstance(value, dict):
            return [f"{where}: expected an object, got {type(value).__name__}"]
    elif expected_type == "string":
        if not isinstance(value, str):
            return [f"{where}: expected a string, got {type(value).__name__}"]
    elif expected_type == "integer":
        # bool is a subclass of int in Python; a fixture line number of
        # `true` should not silently pass as an integer.
        if isinstance(value, bool) or not isinstance(value, int):
            return [f"{where}: expected an integer, got {type(value).__name__}"]
    return []


def validate_against_schema(data, schema, where="fixture"):
    """Minimal hand-rolled subset of JSON Schema: required + type + one
    level of nested properties. Enough for the fixture shape; not a
    general-purpose validator, and deliberately not the `jsonschema`
    package, per the dependency-free rule above."""
    errors = []
    if not isinstance(data, dict):
        return [f"{where}: expected an object, got {type(data).__name__}"]

    for field in schema.get("required", []):
        if field not in data:
            errors.append(f"{where}: missing required field '{field}'")

    for field, subschema in schema.get("properties", {}).items():
        if field not in data:
            continue
        field_type = subschema.get("type")
        if field_type:
            errors.extend(check_type(data[field], field_type, f"{where}.{field}"))
        if field_type == "object":
            errors.extend(
                validate_against_schema(data[field], subschema, f"{where}.{field}")
            )
    return errors


def validate_path_convention(data, path):
    """The directory convention from fixture-format.md: <domain> is the
    parent directory name, <case> is the filename stem."""
    errors = []
    expected_domain = path.parent.name
    expected_case = path.stem
    if data.get("domain") != expected_domain:
        errors.append(
            f"{path}: domain '{data.get('domain')}' does not match "
            f"directory name '{expected_domain}'"
        )
    if data.get("case") != expected_case:
        errors.append(
            f"{path}: case '{data.get('case')}' does not match "
            f"filename '{expected_case}'"
        )
    return errors


def schema_only(path_str):
    """Returns (exit_code, list_of_errors)."""
    path = Path(path_str)
    try:
        with open(path) as f:
            data = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        return 1, [f"{path}: could not read/parse as JSON: {e}"]

    schema = load_schema()
    errors = validate_against_schema(data, schema, where=str(path))
    if not errors:
        # Path-convention checks assume a structurally valid fixture
        # (they read data['domain']/data['case']), so only run them once
        # the shape itself is confirmed.
        errors.extend(validate_path_convention(data, path))
    return (1 if errors else 0), errors


def full_compare(path_str):
    """Returns (exit_code, message)."""
    code, errors = schema_only(path_str)
    if code != 0:
        return 1, "; ".join(errors)

    with open(path_str) as f:
        data = json.load(f)

    domain = data["domain"]
    compute_actual = ACTUAL_COMPUTERS.get(domain)
    if compute_actual is None:
        return (
            1,
            f"no actual-computer registered for domain '{domain}' "
            "(expected until its Rust port lands)",
        )

    actual = compute_actual(data["input"])
    expected = data["expected"]
    if actual == expected:
        return 0, "match"
    return 1, f"mismatch: expected={expected!r} actual={actual!r}"


def validate_dir(dir_str, expect):
    """Returns (exit_code, message)."""
    dir_path = Path(dir_str)
    if not dir_path.is_dir():
        return 1, f"{dir_path}: not a directory"

    files = sorted(dir_path.glob("*.json"))
    if len(files) != expect:
        return 1, f"{dir_path}: found {len(files)} fixture(s), expected {expect}"

    all_errors = []
    for f in files:
        code, errors = schema_only(str(f))
        if code != 0:
            all_errors.extend(errors)

    if all_errors:
        return 1, "; ".join(all_errors)
    return 0, f"ok {len(files)}"


def selftest():
    """Returns (exit_code, [line, line]) — the exit code is the selftest's
    own verdict (did the runner behave as designed), the lines are what
    P0.2's Verify line expects to see."""
    selftest_dir = Path("fixtures/_selftest")
    pass_code, _ = full_compare(str(selftest_dir / "pass.json"))
    fail_code, _ = full_compare(str(selftest_dir / "fail.json"))
    lines = [f"pass={pass_code}", f"fail={fail_code}"]
    ok = pass_code == 0 and fail_code != 0
    return (0 if ok else 1), lines


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("file", nargs="?", help="fixture file")
    parser.add_argument("--schema-only", action="store_true")
    parser.add_argument("--validate-dir", metavar="DIR")
    parser.add_argument("--expect", type=int, metavar="N")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()

    if args.selftest:
        code, lines = selftest()
        for line in lines:
            print(line)
        sys.exit(code)

    if args.validate_dir is not None:
        if args.expect is None:
            parser.error("--validate-dir requires --expect N")
        code, message = validate_dir(args.validate_dir, args.expect)
        print(message)
        sys.exit(code)

    if args.schema_only:
        if args.file is None:
            parser.error("--schema-only requires a fixture file")
        code, errors = schema_only(args.file)
        if errors:
            print("; ".join(errors), file=sys.stderr)
        sys.exit(code)

    if args.file is not None:
        code, message = full_compare(args.file)
        print(message)
        sys.exit(code)

    parser.print_help()
    sys.exit(2)


if __name__ == "__main__":
    main()
