#!/usr/bin/env python3
"""Focused contract gate for P12.19a's host-reset boundary.

The checker inspects the public OpenAPI document rather than a private
implementation. P12.19b and P12.19c must therefore agree on the same route,
DTOs, permission boundary, destructive modes, and post-reset states.
"""

import argparse
import json
import sys

CONTRACT_PATH = "docs/msc2/api-contract/openapi.json"
ROUTE = "/v1/host/reset"
REQUEST_SCHEMA = "HostResetRequestDTO"
RESPONSE_SCHEMA = "HostResetAcceptedDTO"
REQUIRED_ERROR_CODES = {
    "400": "invalid_body",
    "403": "forbidden",
    "409": "conflict",
    "500": "internal_error",
}
EXPECTED_MODES = ["configuration", "everything"]
EXPECTED_POST_RESET_STATES = ["restarting", "needs_pairing", "unavailable"]
EXPECTED_CONFIRMATION = "RESET <current-agent-host-id>"
EXPECTED_CONFIGURATION_REMOVED = [
    "config/host.json",
    "config/servers.json",
    "host/identity.json",
    "auth/credentials/**",
    "auth/sessions/**",
    "auth/pairings/**",
    "operations/reset-owned/**",
]


def load(path=CONTRACT_PATH):
    with open(path) as handle:
        return json.load(handle)


def ref_name(schema):
    ref = schema.get("$ref", "")
    return ref.rsplit("/", 1)[-1] if ref else None


def check_contract(doc):
    failures = []
    post = doc.get("paths", {}).get(ROUTE, {}).get("post")
    schemas = doc.get("components", {}).get("schemas", {})

    if post is None:
        return [f"missing POST {ROUTE}"]
    if post.get("operationId") != "resetHost":
        failures.append("operationId must be resetHost")
    if post.get("x-permission-category") != "admin":
        failures.append("route must require x-permission-category=admin")
    if post.get("x-authentication") != "admin-bearer-or-browser-session-csrf":
        failures.append("route must declare admin-bearer-or-browser-session-csrf")

    body_schema = post.get("requestBody", {}).get("content", {}).get("application/json", {}).get("schema", {})
    if ref_name(body_schema) != REQUEST_SCHEMA:
        failures.append(f"request body must reference {REQUEST_SCHEMA}")

    response_schema = post.get("responses", {}).get("202", {}).get("content", {}).get("application/json", {}).get("schema", {})
    if ref_name(response_schema) != RESPONSE_SCHEMA:
        failures.append(f"202 response must reference {RESPONSE_SCHEMA}")

    responses = post.get("responses", {})
    for status, code in REQUIRED_ERROR_CODES.items():
        response = responses.get(status)
        if response is None:
            failures.append(f"missing {status} response")
            continue
        error_schema = response.get("content", {}).get("application/json", {}).get("schema", {})
        if ref_name(error_schema) != "ErrorDTO":
            failures.append(f"{status} response must reference ErrorDTO")
        if response.get("x-error-code") != code:
            failures.append(f"{status} x-error-code must be {code}")

    request = schemas.get(REQUEST_SCHEMA, {})
    request_props = request.get("properties", {})
    if request.get("additionalProperties") is not False:
        failures.append("request must reject additional properties")
    if request.get("required") != ["mode", "confirmation"]:
        failures.append("request required fields must be mode, confirmation")
    if request_props.get("mode", {}).get("enum") != EXPECTED_MODES:
        failures.append("request mode enum must be configuration, everything")
    if request_props.get("confirmation", {}).get("description") != "Must exactly equal RESET <current-agent-host-id>.":
        failures.append("request confirmation must require the host-specific literal")

    response = schemas.get(RESPONSE_SCHEMA, {})
    response_props = response.get("properties", {})
    if response.get("required") != ["operationId", "hostId", "mode", "agentState", "message"]:
        failures.append("response required fields are incomplete or reordered")
    if response_props.get("mode", {}).get("enum") != EXPECTED_MODES:
        failures.append("response mode enum must be configuration, everything")
    if response_props.get("agentState", {}).get("enum") != EXPECTED_POST_RESET_STATES:
        failures.append("response agentState enum must describe all truthful recovery states")

    reset_contract = post.get("x-reset-contract", {})
    if reset_contract.get("hostScoped") is not True:
        failures.append("reset must be host-scoped")
    if reset_contract.get("clientResetTransport") != "local-only":
        failures.append("client reset must remain local-only")
    if reset_contract.get("serviceUninstall") != "local-desktop-only":
        failures.append("service uninstall must remain local-desktop-only")
    if reset_contract.get("confirmationFormat") != EXPECTED_CONFIRMATION:
        failures.append("reset confirmation format drifted")
    if reset_contract.get("runningServerRefusal") != "server_running":
        failures.append("running-server refusal drifted")
    if reset_contract.get("postResetStates") != EXPECTED_POST_RESET_STATES:
        failures.append("post-reset state list drifted")
    if reset_contract.get("remoteRecovery") != "fresh-one-use-host-local-pairing":
        failures.append("remote recovery must require fresh host-local one-use pairing")

    modes = reset_contract.get("modes", {})
    configuration = modes.get("configuration", {})
    everything = modes.get("everything", {})
    if configuration.get("preserves") != ["servers-root/**"]:
        failures.append("configuration mode must preserve servers-root/**")
    if configuration.get("removes") != EXPECTED_CONFIGURATION_REMOVED:
        failures.append("configuration mode deletion allowlist drifted")
    if everything.get("preserves") != ["agent service"]:
        failures.append("everything mode must preserve the installed agent service")
    if everything.get("removes") != EXPECTED_CONFIGURATION_REMOVED + ["servers-root/**"]:
        failures.append("everything mode deletion allowlist drifted")

    return failures


def selftest():
    clean = load()
    failures = check_contract(clean)
    if failures:
        return False

    dirty = json.loads(json.dumps(clean))
    dirty["paths"][ROUTE]["post"]["x-permission-category"] = "settings"
    return bool(check_contract(dirty))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    if args.selftest:
        passed = selftest()
        print(f"pass={int(passed)}")
        print(f"fail={int(not passed)}")
        raise SystemExit(0 if passed else 1)
    failures = check_contract(load())
    if failures:
        for failure in failures:
            print(failure, file=sys.stderr)
        raise SystemExit(1)
    print(f"ok {ROUTE}: reset contract, DTOs, errors, deletion boundary, and recovery states")


if __name__ == "__main__":
    main()
