#!/usr/bin/env bash
set -euo pipefail

# Phase 5 CLI smoke harness. Owns one temporary application root and one
# running agent for every portion of Phase 5's public CLI/API surface this
# script proves. Each portion is gated behind its own flag so a single step
# can run only what it owns (P5.11: --settings); later steps (transfer
# import, raw import) extend this same script and its shared setup/teardown
# rather than starting a separately-running agent or relying on an
# installed `msc` binary.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

RUN_SETTINGS=0
if [[ $# -eq 0 ]]; then
  RUN_SETTINGS=1
else
  for arg in "$@"; do
    case "$arg" in
      --settings)
        RUN_SETTINGS=1
        ;;
      *)
        echo "unknown flag: ${arg}" >&2
        echo "usage: $0 [--settings]" >&2
        exit 2
        ;;
    esac
  done
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/msc2-phase5-cli-smoke.XXXXXX")"
PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
BASE_URL="http://127.0.0.1:${PORT}"
TOKEN="msc2_phase5_cli_smoke_bootstrap_secret"
MSC_BIN="${ROOT}/target/debug/msc"
AGENT_PID=""

cleanup() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 1
  fi
}

require_tool cargo
require_tool python3

(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

export MSC2_TEST_BOOTSTRAP_TOKEN="${TOKEN}"
export MSC2_OPERATION_JOURNAL_DIR="${TMP_DIR}/journal"
export MSC2_CREDENTIAL_REGISTRY_PATH="${TMP_DIR}/credential-registry.json"
mkdir -p "${MSC2_OPERATION_JOURNAL_DIR}"

"${MSC_BIN}" serve --bind "127.0.0.1:${PORT}" >"${TMP_DIR}/agent.log" 2>&1 &
AGENT_PID="$!"

python3 - "${BASE_URL}" <<'PY'
import sys
import time
import urllib.error
import urllib.request

base_url = sys.argv[1]
deadline = time.time() + 45
while time.time() < deadline:
    try:
        with urllib.request.urlopen(base_url + "/v1/health", timeout=1) as resp:
            if resp.status == 200:
                raise SystemExit(0)
    except (urllib.error.URLError, TimeoutError):
        time.sleep(0.25)
raise SystemExit("agent did not become healthy")
PY

TOKEN_FROM_CLI="$("${MSC_BIN}" token print --test)"

run_settings_smoke() {
  local server_dir="${TMP_DIR}/settings-server"
  local properties_file="${server_dir}/server.properties"
  mkdir -p "${server_dir}"
  : > "${server_dir}/paper.jar"
  cat > "${properties_file}" <<'EOF'
server-port=25565
max-players=20
difficulty=easy
motd=Before
EOF

  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" \
    server import "${server_dir}" --name "Settings Smoke" >/dev/null

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token = sys.argv[1:4]
output = subprocess.check_output(
    [
        msc, "--base-url", base_url, "--token", token, "--json",
        "settings", "get", "--server", "Settings Smoke",
    ],
    text=True,
)
settings = json.loads(output)
if not settings["editable"]:
    raise SystemExit(f"expected settings to be editable, got {settings!r}")
sections = {section["id"]: section for section in settings["sections"]}
if not {"world", "server", "network"} <= sections.keys():
    raise SystemExit(f"expected world/server/network sections, got {list(sections)}")
server_fields = {field["key"]: field["value"] for field in sections["server"]["fields"]}
if server_fields.get("max-players") != "20":
    raise SystemExit(f"expected max-players=20 before the update, got {server_fields!r}")
if server_fields.get("motd") != "Before":
    raise SystemExit(f"expected motd=Before before the update, got {server_fields!r}")
PY

  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" \
    settings set --server "Settings Smoke" "max-players=42" "motd=After" >/dev/null

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token = sys.argv[1:4]
output = subprocess.check_output(
    [msc, "--base-url", base_url, "--token", token, "--json", "settings", "get"],
    text=True,
)
settings = json.loads(output)
sections = {section["id"]: section for section in settings["sections"]}
server_fields = {field["key"]: field["value"] for field in sections["server"]["fields"]}
if server_fields.get("max-players") != "42":
    raise SystemExit(f"expected max-players=42 after the update, got {server_fields!r}")
if server_fields.get("motd") != "After":
    raise SystemExit(f"expected motd=After after the update, got {server_fields!r}")
PY

  if ! grep -q '^max-players=42$' "${properties_file}"; then
    echo "persisted server.properties did not contain max-players=42" >&2
    cat "${properties_file}" >&2
    exit 1
  fi
  if ! grep -q '^motd=After$' "${properties_file}"; then
    echo "persisted server.properties did not contain motd=After" >&2
    cat "${properties_file}" >&2
    exit 1
  fi

  echo "settings cli smoke passed"
}

if [[ "${RUN_SETTINGS}" -eq 1 ]]; then
  run_settings_smoke
fi
