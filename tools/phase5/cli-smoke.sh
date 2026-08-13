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
RUN_RAW=0
RUN_IMPORT_LIFECYCLE=0
RUN_RESCAN=0
RUN_MIGRATION_RESTART=0
RUN_REPLACE_ALL=0
if [[ $# -eq 0 ]]; then
  RUN_SETTINGS=1
  RUN_TRANSFER=1
  RUN_RAW=1
  RUN_IMPORT_LIFECYCLE=1
  RUN_RESCAN=1
  RUN_MIGRATION_RESTART=1
else
  for arg in "$@"; do
    case "$arg" in
      --settings)
        RUN_SETTINGS=1
        ;;
      --transfer)
        RUN_TRANSFER=1
        ;;
      --raw)
        RUN_RAW=1
        ;;
      --import-lifecycle)
        RUN_IMPORT_LIFECYCLE=1
        ;;
      --rescan)
        RUN_RESCAN=1
        ;;
      --migration-restart)
        RUN_MIGRATION_RESTART=1
        ;;
      --replace-all)
        RUN_REPLACE_ALL=1
        ;;
      *)
        echo "unknown flag: ${arg}" >&2
        echo "usage: $0 [--settings] [--transfer] [--raw] [--import-lifecycle] [--rescan] [--migration-restart] [--replace-all]" >&2
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
KEYCHAIN_SERVICE="com.msc2.phase5.cli-smoke.$(date +%Y%m%d%H%M%S).$$"
MIGRATED_CREDENTIAL_ID=""

cleanup() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  if [[ "$(uname -s)" == "Darwin" ]]; then
    /usr/bin/security delete-generic-password \
      -s "${KEYCHAIN_SERVICE}" \
      -a "remote-api.token.phase5" >/dev/null 2>&1 || true
    /usr/bin/security delete-generic-password \
      -s "${KEYCHAIN_SERVICE}" \
      -a "remote-api.owner-token" >/dev/null 2>&1 || true
    /usr/bin/security delete-generic-password \
      -s "${KEYCHAIN_SERVICE}" \
      -a "xbox-broadcast.alt-password.11111111-1111-1111-1111-111111111111" >/dev/null 2>&1 || true
    if [[ -n "${MIGRATED_CREDENTIAL_ID}" ]]; then
      /usr/bin/security delete-generic-password \
        -s "${KEYCHAIN_SERVICE}" \
        -a "remote-api.token.${MIGRATED_CREDENTIAL_ID}" >/dev/null 2>&1 || true
    fi
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

export MSC2_JAVA_PATH="${TMP_DIR}/fake-java"
cat > "${MSC2_JAVA_PATH}" <<'EOF'
#!/usr/bin/env bash
echo "Booting fake Paper"
sleep 1
echo 'Done (0.001s)! For help, type "help"'
while IFS= read -r line; do
  echo "COMMAND:${line}"
  if [[ "${line}" == "stop" ]]; then
    echo "Stopping fake Paper"
    exit 0
  fi
done
EOF
chmod 755 "${MSC2_JAVA_PATH}"

(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

export MSC2_TEST_BOOTSTRAP_TOKEN="${TOKEN}"
export MSC2_OPERATION_JOURNAL_DIR="${TMP_DIR}/journal"
export MSC2_CREDENTIAL_REGISTRY_PATH="${TMP_DIR}/credential-registry.json"
export MSC2_DATA_DIR="${TMP_DIR}/data"
export MSC2_APP_CONFIG_PATH="${MSC2_DATA_DIR}/server_config_swift.json"
if [[ "$(uname -s)" == "Darwin" ]]; then
  export MSC2_MACOS_USER_KEYCHAIN_SERVICE="${KEYCHAIN_SERVICE}"
elif [[ "$(uname -s)" == "Linux" ]]; then
  export MSC2_LINUX_FOREGROUND_SECRET_STORE_DIR="${TMP_DIR}/linux-secret-store"
fi
# Shared by transfer, raw, and no-serverType importExisting routes.
export MSC2_AGENT_SERVERS_ROOT="${TMP_DIR}/agent-servers"
mkdir -p "${MSC2_OPERATION_JOURNAL_DIR}" "${MSC2_DATA_DIR}"

if [[ "${RUN_MIGRATION_RESTART}" -eq 1 ]]; then
  mkdir -p "${MSC2_AGENT_SERVERS_ROOT}/java/legacy_migration"
  : > "${MSC2_AGENT_SERVERS_ROOT}/java/legacy_migration/paper.jar"
  cat > "${MSC2_APP_CONFIG_PATH}" <<EOF
{
  "config_version": 1,
  "servers_root": "${MSC2_AGENT_SERVERS_ROOT}",
  "remote_api_token": "legacy-owner-secret-xyz",
  "servers": [
    {
      "id": "11111111-1111-1111-1111-111111111111",
      "display_name": "Legacy Migration",
      "server_dir": "${MSC2_AGENT_SERVERS_ROOT}/java/legacy_migration",
      "paper_jar_path": "${MSC2_AGENT_SERVERS_ROOT}/java/legacy_migration/paper.jar",
      "min_ram_gb": 2,
      "max_ram_gb": 4,
      "server_type": "java",
      "xbox_broadcast_alt_password": "legacy-alt-password"
    }
  ]
}
EOF
fi

"${MSC_BIN}" serve --bind "127.0.0.1:${PORT}" >"${TMP_DIR}/agent.log" 2>&1 &
AGENT_PID="$!"

wait_for_agent_healthy() {
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
}

wait_for_agent_healthy

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
  properties_file="${MSC2_AGENT_SERVERS_ROOT}/java/settings_smoke/server.properties"

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

  set +e
  rejected_after_replace=$("${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json status 2>&1)
  rejected_after_replace_status=$?
  set -e
  if [[ "${rejected_after_replace_status}" -eq 0 ]]; then
    echo "expected replaceAll to invalidate the token used for the transfer smoke" >&2
    exit 1
  fi
  if ! grep -q "unauthorized" <<<"${rejected_after_replace}"; then
    echo "expected unauthorized after transfer replaceAll, got: ${rejected_after_replace}" >&2
    exit 1
  fi

  echo "transfer cli smoke passed"
}

build_raw_java_folder() {
  local dir="$1" port="$2"
  mkdir -p "${dir}"
  : > "${dir}/paper-1.21.1-131.jar"
  cat > "${dir}/server.properties" <<EOF
server-port=${port}
max-players=20
level-name=world
EOF
  echo "eula=true" > "${dir}/eula.txt"
}

build_raw_bedrock_folder() {
  local dir="$1" port="$2"
  mkdir -p "${dir}"
  : > "${dir}/bedrock_server"
  cat > "${dir}/server.properties" <<EOF
server-port=${port}
max-players=10
EOF
}

build_raw_java_zip() {
  local zip_path="$1" port="$2"
  python3 - "${zip_path}" "${port}" <<'PY'
import sys
import zipfile

zip_path, port = sys.argv[1], int(sys.argv[2])
with zipfile.ZipFile(zip_path, "w") as zf:
    zf.writestr("paper-1.21.1-131.jar", b"")
    zf.writestr(
        "server.properties",
        f"server-port={port}\nmax-players=20\nlevel-name=world\n",
    )
    zf.writestr("eula.txt", "eula=true\n")
PY
}

build_traversal_zip() {
  local zip_path="$1"
  python3 - "${zip_path}" <<'PY'
import sys
import zipfile

zip_path = sys.argv[1]
with zipfile.ZipFile(zip_path, "w") as zf:
    zf.writestr("paper-1.21.1-131.jar", b"")
    zf.writestr("../evil.txt", b"escaped")
PY
}

run_raw_smoke() {
  local java_folder="${TMP_DIR}/raw-java-folder"
  local java_zip="${TMP_DIR}/raw-java.zip"
  local bedrock_folder="${TMP_DIR}/raw-bedrock-folder"
  local traversal_zip="${TMP_DIR}/raw-traversal.zip"

  build_raw_java_folder "${java_folder}" 25610
  build_raw_java_zip "${java_zip}" 25611
  build_raw_bedrock_folder "${bedrock_folder}" 19140
  build_traversal_zip "${traversal_zip}"

  # Scan all three: prove the scan response is labelled correctly and
  # never mutates anything (no destination should exist yet).
  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" "${java_folder}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token, path = sys.argv[1:5]
output = subprocess.check_output(
    [msc, "--base-url", base_url, "--token", token, "--json",
     "server", "import", path, "--scan", "--kind", "folder"],
    text=True,
)
result = json.loads(output)
if not result["success"] or result["serverType"] != "java" or result["javaFlavor"] != "paper":
    raise SystemExit(f"expected a paper java scan, got {result!r}")
if result["port"] != 25610 or result["maxPlayers"] != 20 or not result["eulaAccepted"]:
    raise SystemExit(f"unexpected scanned properties: {result!r}")
PY

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" "${java_zip}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token, path = sys.argv[1:5]
output = subprocess.check_output(
    [msc, "--base-url", base_url, "--token", token, "--json",
     "server", "import", path, "--scan"],
    text=True,
)
result = json.loads(output)
if not result["success"] or not result["isZip"] or result["serverType"] != "java":
    raise SystemExit(f"expected a zip java scan, got {result!r}")
if result["port"] != 25611:
    raise SystemExit(f"unexpected scanned port: {result!r}")
PY

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" "${bedrock_folder}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token, path = sys.argv[1:5]
output = subprocess.check_output(
    [msc, "--base-url", base_url, "--token", token, "--json",
     "server", "import", path, "--scan", "--kind", "folder"],
    text=True,
)
result = json.loads(output)
if not result["success"] or result["serverType"] != "bedrock" or result.get("javaFlavor"):
    raise SystemExit(f"expected a bedrock scan, got {result!r}")
if result["port"] != 19140:
    raise SystemExit(f"unexpected scanned port: {result!r}")
PY

  # Import all three, each with an override, and verify the copied
  # destination on disk carries it.
  local java_dest="${MSC2_AGENT_SERVERS_ROOT}/java/raw_smoke_java_folder"
  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json \
    server import "${java_folder}" --kind folder --type java \
    --name "Raw Smoke Java Folder" --game-port 25620 --max-players 9 >/dev/null
  if ! grep -q '^server-port=25620$' "${java_dest}/server.properties"; then
    echo "expected the copied java folder's server.properties to carry the port override" >&2
    exit 1
  fi
  if ! grep -q '^max-players=9$' "${java_dest}/server.properties"; then
    echo "expected the copied java folder's server.properties to carry the max-players override" >&2
    exit 1
  fi
  if [[ ! -f "${java_folder}/paper-1.21.1-131.jar" ]]; then
    echo "expected the original java folder source to remain untouched (a copy, not a move)" >&2
    exit 1
  fi

  local zip_dest="${MSC2_AGENT_SERVERS_ROOT}/java/raw_smoke_java_zip"
  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json \
    server import "${java_zip}" --type java --name "Raw Smoke Java Zip" --game-port 25621 >/dev/null
  if [[ ! -f "${zip_dest}/paper-1.21.1-131.jar" ]]; then
    echo "expected the zip source to have been extracted to ${zip_dest}" >&2
    exit 1
  fi
  if ! grep -q '^server-port=25621$' "${zip_dest}/server.properties"; then
    echo "expected the extracted zip's server.properties to carry the port override" >&2
    exit 1
  fi

  local bedrock_dest="${MSC2_AGENT_SERVERS_ROOT}/bedrock/raw_smoke_bedrock_folder"
  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json \
    server import "${bedrock_folder}" --kind folder --type bedrock \
    --name "Raw Smoke Bedrock Folder" --game-port 19150 >/dev/null
  if [[ ! -f "${bedrock_dest}/bedrock_server" ]]; then
    echo "expected the copied bedrock folder at ${bedrock_dest}" >&2
    exit 1
  fi

  assert_servers_present "${BASE_URL}" "${TOKEN_FROM_CLI}" \
    "Raw Smoke Java Folder" "Raw Smoke Java Zip" "Raw Smoke Bedrock Folder"

  # A traversal zip is rejected, and leaves no destination behind.
  local traversal_dest="${MSC2_AGENT_SERVERS_ROOT}/java/raw_smoke_traversal"
  set +e
  traversal_output=$("${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json \
    server import "${traversal_zip}" --type java --name "Raw Smoke Traversal" 2>&1)
  traversal_exit=$?
  set -e
  if [[ "${traversal_exit}" -eq 0 ]]; then
    echo "expected a traversal zip import to fail, got: ${traversal_output}" >&2
    exit 1
  fi
  if [[ -e "${traversal_dest}" ]]; then
    echo "expected no destination left behind after a rejected traversal zip import" >&2
    exit 1
  fi

  echo "raw cli smoke passed"
}

run_import_lifecycle_smoke() {
  local source="${TMP_DIR}/import-lifecycle-java-source"
  local managed="${MSC2_AGENT_SERVERS_ROOT}/java/import_lifecycle_java"
  build_raw_java_folder "${source}" 25630

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" "${source}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token, path = sys.argv[1:5]
output = subprocess.check_output(
    [
        msc, "--base-url", base_url, "--token", token, "--json",
        "server", "import", path, "--name", "Import Lifecycle Java",
    ],
    text=True,
)
result = json.loads(output)
if not result["success"] or result.get("serverId") is None:
    raise SystemExit(f"expected no-type importExisting to infer and import a Java server, got {result!r}")
PY

  if [[ ! -f "${managed}/paper-1.21.1-131.jar" ]]; then
    echo "expected no-type importExisting to copy the Paper server into ${managed}" >&2
    exit 1
  fi

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token = sys.argv[1:4]
output = subprocess.check_output(
    [
        msc, "--base-url", base_url, "--token", token, "--json",
        "settings", "get", "--server", "Import Lifecycle Java",
    ],
    text=True,
)
settings = json.loads(output)
if not settings["editable"] or settings["serverName"] != "Import Lifecycle Java":
    raise SystemExit(f"expected imported Java server to be settings-capable, got {settings!r}")
PY

  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" \
    server start "Import Lifecycle Java" >/dev/null

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 30
while time.time() < deadline:
    output = subprocess.check_output(
        [msc, "--base-url", base_url, "--token", token, "--json", "status"],
        text=True,
    )
    status = json.loads(output)
    if status["running"] and status.get("activeServerId"):
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("imported Java server never reached running state")
PY

  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" \
    server stop "Import Lifecycle Java" >/dev/null

  echo "import lifecycle cli smoke passed"
}

run_rescan_smoke() {
  local source="${MSC2_AGENT_SERVERS_ROOT}/java/rescan_smoke_java"
  build_raw_java_folder "${source}" 25640

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token = sys.argv[1:4]
output = subprocess.check_output(
    [msc, "--base-url", base_url, "--token", token, "--json", "server", "rescan"],
    text=True,
)
result = json.loads(output)
if not result["success"] or result.get("imported", 0) < 1:
    raise SystemExit(f"expected rescan to import at least one untracked managed server, got {result!r}")
if result.get("serverName") != "rescan smoke java":
    raise SystemExit(f"expected display name from folder name, got {result!r}")
PY

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token = sys.argv[1:4]
output = subprocess.check_output(
    [
        msc, "--base-url", base_url, "--token", token, "--json",
        "settings", "get", "--server", "rescan smoke java",
    ],
    text=True,
)
settings = json.loads(output)
if not settings["editable"] or settings["serverName"] != "rescan smoke java":
    raise SystemExit(f"expected rescanned Java server to be settings-capable, got {settings!r}")
PY

  kill "${AGENT_PID}"
  wait "${AGENT_PID}" 2>/dev/null || true
  AGENT_PID=""
  "${MSC_BIN}" serve --bind "127.0.0.1:${PORT}" >"${TMP_DIR}/agent-restarted.log" 2>&1 &
  AGENT_PID="$!"
  wait_for_agent_healthy

  assert_servers_present "${BASE_URL}" "${TOKEN_FROM_CLI}" "rescan smoke java"

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token = sys.argv[1:4]
output = subprocess.check_output(
    [msc, "--base-url", base_url, "--token", token, "--json", "server", "rescan"],
    text=True,
)
result = json.loads(output)
if result.get("imported") != 0:
    raise SystemExit(f"expected second rescan after restart to avoid duplicates, got {result!r}")
PY

  echo "rescan cli smoke passed"
}

run_migration_restart_smoke() {
  local migrated_token
  migrated_token="$(python3 - "${TMP_DIR}/agent.log" <<'PY'
import pathlib
import sys
import time

path = pathlib.Path(sys.argv[1])
marker = "new bearer token (shown once): "
deadline = time.time() + 20
while time.time() < deadline:
    text = path.read_text(errors="replace") if path.exists() else ""
    start = text.find(marker)
    if start != -1:
        token = text[start + len(marker):].splitlines()[0].strip()
        if token.startswith("msc2_"):
            print(token)
            raise SystemExit(0)
    time.sleep(0.1)
raise SystemExit("agent did not print migrated bearer token")
PY
)"
  local token_tail="${migrated_token#msc2_}"
  MIGRATED_CREDENTIAL_ID="${token_tail%%_*}"

  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${migrated_token}" --json status >/dev/null

  if grep -q 'remote_api_token\|xbox_broadcast_alt_password' "${MSC2_APP_CONFIG_PATH}"; then
    echo "legacy plaintext secrets remained in ${MSC2_APP_CONFIG_PATH}" >&2
    cat "${MSC2_APP_CONFIG_PATH}" >&2
    exit 1
  fi

  kill "${AGENT_PID}"
  wait "${AGENT_PID}" 2>/dev/null || true
  AGENT_PID=""
  "${MSC_BIN}" serve --bind "127.0.0.1:${PORT}" >"${TMP_DIR}/agent-migration-restarted.log" 2>&1 &
  AGENT_PID="$!"
  wait_for_agent_healthy

  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${migrated_token}" --json status >/dev/null

  echo "migration restart cli smoke passed"
}

run_replace_all_smoke() {
  local old_source="${TMP_DIR}/replace-all-old-source"
  local package="${TMP_DIR}/replace-all-new.msctransfer"
  local backup_path="${TMP_DIR}/replace-all-backup.msctransfer"
  build_raw_java_folder "${old_source}" 25650
  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json \
    server import "${old_source}" --name "ReplaceAll Old" >/dev/null
  build_transfer_package "${package}" "REPLACE-NEW" "ReplaceAll New" 25651

  python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN_FROM_CLI}" "${package}" "${backup_path}" <<'PY'
import json
import subprocess
import sys

msc, base_url, token, package, backup_path = sys.argv[1:6]
output = subprocess.check_output(
    [
        msc, "--base-url", base_url, "--token", token, "--json",
        "server", "import", package, "--transfer-mode", "replaceAll",
        "--backup-path", backup_path,
    ],
    text=True,
)
result = json.loads(output)
if not result["success"] or result.get("replaced") is not True:
    raise SystemExit(f"expected replaceAll success, got {result!r}")
PY

  if [[ ! -f "${backup_path}" ]]; then
    echo "expected replaceAll backup at ${backup_path}" >&2
    exit 1
  fi

  set +e
  rejected="$("${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN_FROM_CLI}" --json status 2>&1)"
  rejected_status=$?
  set -e
  if [[ "${rejected_status}" -eq 0 ]]; then
    echo "expected the calling token to be invalid after replaceAll" >&2
    exit 1
  fi
  if ! grep -q "unauthorized" <<<"${rejected}"; then
    echo "expected unauthorized after replaceAll token wipe, got: ${rejected}" >&2
    exit 1
  fi

  echo "replaceAll cli smoke passed"
}

if [[ "${RUN_MIGRATION_RESTART}" -eq 1 ]]; then
  run_migration_restart_smoke
fi

if [[ "${RUN_SETTINGS}" -eq 1 ]]; then
  run_settings_smoke
fi

if [[ "${RUN_RAW}" -eq 1 ]]; then
  run_raw_smoke
fi

if [[ "${RUN_IMPORT_LIFECYCLE}" -eq 1 ]]; then
  run_import_lifecycle_smoke
fi

if [[ "${RUN_RESCAN}" -eq 1 ]]; then
  run_rescan_smoke
fi

if [[ "${RUN_REPLACE_ALL}" -eq 1 ]]; then
  run_replace_all_smoke
fi

if [[ "${RUN_TRANSFER}" -eq 1 ]]; then
  run_transfer_smoke
fi
