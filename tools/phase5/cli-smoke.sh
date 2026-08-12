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
RUN_TRANSFER=0
if [[ $# -eq 0 ]]; then
  RUN_SETTINGS=1
  RUN_TRANSFER=1
else
  for arg in "$@"; do
    case "$arg" in
      --settings)
        RUN_SETTINGS=1
        ;;
      --transfer)
        RUN_TRANSFER=1
        ;;
      *)
        echo "unknown flag: ${arg}" >&2
        echo "usage: $0 [--settings] [--transfer]" >&2
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
export MSC2_TRANSFER_SERVERS_ROOT="${TMP_DIR}/transfer-servers"
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

build_transfer_package() {
  local package_path="$1" server_id="$2" display_name="$3" port="$4"
  python3 - "${package_path}" "${server_id}" "${display_name}" "${port}" <<'PY'
import json
import sys
import zipfile

package_path, server_id, display_name, port = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
folder_name = server_id.lower()

manifest = {
    "formatVersion": 2,
    "appConfigVersion": 1,
    "createdAt": "2026-01-01T00:00:00Z",
    "sourceMachineName": "cli-smoke",
    "servers": [
        {
            "server": {
                "id": server_id,
                "display_name": display_name,
                "server_dir": "",
                "paper_jar_path": "",
                "min_ram_gb": 2,
                "max_ram_gb": 4,
                "server_type": "java",
            },
            "folderName": folder_name,
            "javaPort": port,
            "paperMCVersion": None,
            "paperBuild": None,
            "bundledPaperJar": False,
            "pluginLinks": [],
        }
    ],
}

with zipfile.ZipFile(package_path, "w") as zf:
    zf.writestr("manifest.json", json.dumps(manifest))
    zf.writestr(
        f"servers/{folder_name}/configs/server.properties",
        f"server-port={port}\nmax-players=8\n",
    )
PY
}

assert_servers_present() {
  # Args: base_url token expected-name...
  python3 - "$@" <<'PY'
import json
import sys
import urllib.request

base_url, token = sys.argv[1], sys.argv[2]
expected = set(sys.argv[3:])
request = urllib.request.Request(
    base_url + "/v1/servers", headers={"Authorization": f"Bearer {token}"}
)
with urllib.request.urlopen(request, timeout=5) as resp:
    servers = json.load(resp)
names = {server["name"] for server in servers}
if not expected <= names:
    raise SystemExit(f"expected {expected} to be a subset of {names}")
PY
}

assert_servers_replaced() {
  # Args: base_url token still-present-name removed-name...
  # Doesn't assert the *complete* server list, since `--settings` may have
  # already imported an unrelated Paper server in this same run —
  # replaceAll only replaces the transfer-imported set (see servers.rs's
  # "Transfer-package import" header comment for the flagged gap).
  python3 - "$@" <<'PY'
import json
import sys
import urllib.request

base_url, token, still_present = sys.argv[1], sys.argv[2], sys.argv[3]
removed = set(sys.argv[4:])
request = urllib.request.Request(
    base_url + "/v1/servers", headers={"Authorization": f"Bearer {token}"}
)
with urllib.request.urlopen(request, timeout=5) as resp:
    servers = json.load(resp)
names = {server["name"] for server in servers}
if still_present not in names:
    raise SystemExit(f"expected {still_present!r} present after replaceAll, got {names}")
if names & removed:
    raise SystemExit(f"expected {removed} removed by replaceAll, got {names}")
PY
}

run_transfer_smoke() {
  local package_a="${TMP_DIR}/transfer-a.msctransfer"
  local package_b="${TMP_DIR}/transfer-b.msctransfer"
  local package_c="${TMP_DIR}/transfer-c.msctransfer"
  local backup_path="${TMP_DIR}/before-replace-all.msctransfer"

  build_transfer_package "${package_a}" "TRANSFER-A" "Transfer Smoke A" 25566
  build_transfer_package "${package_b}" "TRANSFER-B" "Transfer Smoke B" 25567
  build_transfer_package "${package_c}" "TRANSFER-C" "Transfer Smoke C" 25568

  # Prove merge: two independent transfer imports both land, neither
  # replacing the other.
  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" "${package_a}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token, package = sys.argv[1:5]
output = subprocess.check_output(
    [msc, "--base-url", base_url, "--token", token, "--json", "server", "import", package],
    text=True,
)
result = json.loads(output)
if not result["success"] or result["imported"] != 1 or result.get("replaced"):
    raise SystemExit(f"expected a successful merge import of 1 server, got {result!r}")
PY

  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json \
    server import "${package_b}" >/dev/null

  assert_servers_present "${BASE_URL}" "${TOKEN_FROM_CLI}" "Transfer Smoke A" "Transfer Smoke B"

  # Prove missing-backup rejection: replaceAll without --backup-path fails
  # before touching anything, and never applies package C.
  set +e
  reject_output=$("${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json \
    server import "${package_c}" --transfer-mode replaceAll 2>&1)
  reject_exit=$?
  set -e
  if [[ "${reject_exit}" -eq 0 ]]; then
    echo "expected replaceAll without --backup-path to fail" >&2
    exit 1
  fi
  if ! grep -q "backup_path_required" <<<"${reject_output}"; then
    echo "expected backup_path_required in rejected output, got: ${reject_output}" >&2
    exit 1
  fi
  assert_servers_present "${BASE_URL}" "${TOKEN_FROM_CLI}" "Transfer Smoke A" "Transfer Smoke B"

  # Prove backup-before-replace-all: a real backup file is written, then
  # the whole transfer-imported set is replaced by package C alone.
  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" "${package_c}" "${backup_path}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token, package, backup_path = sys.argv[1:6]
output = subprocess.check_output(
    [
        msc, "--base-url", base_url, "--token", token, "--json", "server", "import", package,
        "--transfer-mode", "replaceAll", "--backup-path", backup_path,
    ],
    text=True,
)
result = json.loads(output)
if not result["success"] or not result.get("replaced"):
    raise SystemExit(f"expected a successful replaceAll import, got {result!r}")
PY

  if [[ ! -f "${backup_path}" ]]; then
    echo "expected a backup file at ${backup_path} before replaceAll" >&2
    exit 1
  fi
  assert_servers_replaced "${BASE_URL}" "${TOKEN_FROM_CLI}" "Transfer Smoke C" \
    "Transfer Smoke A" "Transfer Smoke B"

  echo "transfer cli smoke passed"
}

if [[ "${RUN_SETTINGS}" -eq 1 ]]; then
  run_settings_smoke
fi

if [[ "${RUN_TRANSFER}" -eq 1 ]]; then
  run_transfer_smoke
fi
