#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="$(date +%Y%m%d%H%M%S)-$$"
RUN_DIR="/tmp/msc2-linux-service-lifecycle.${RUN_ID}"
TARGET_USER="${SUDO_USER:-${USER:-}}"
TARGET_GROUP=""
SERVER_DIR=""
KEEP_ARTIFACTS=0
LABEL="msc2-phase4-agent-${RUN_ID}"
HELPER_SOCKET_UNIT="msc2-credential-helper.socket"
HELPER_SERVICE_UNIT="msc2-credential-helper.service"
AGENT_UNIT_PATH="/etc/systemd/system/${LABEL}.service"
HELPER_SOCKET_PATH="/etc/systemd/system/${HELPER_SOCKET_UNIT}"
HELPER_SERVICE_PATH="/etc/systemd/system/${HELPER_SERVICE_UNIT}"
HELPER_RUNTIME_SOCKET="/run/msc2/credential-helper.sock"
HELPER_STORE_DIR="/var/lib/msc2/credentials"
PORT=""
BASE_URL=""
TOKEN="msc2_phase4_systemd_secret"
MSC_BIN="${ROOT}/target/debug/msc"
SERVER_NAME="Phase4 Paper"
SERVER_PORT=""

usage() {
  cat <<USAGE
Usage: $0 --server-dir <path> [--user <installing-user>] [--keep-artifacts]

Builds the Phase 4 agent, installs it as a systemd service running as the
installing user, imports the given Paper server through the public CLI/API
path, starts it, proves the agent and Java server survive with no clients
connected, installs the Phase 4 credential-helper socket/service definitions,
checks their ownership and permissions, then stops the server and uninstalls
everything cleanly.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --server-dir)
      SERVER_DIR="${2:-}"
      shift 2
      ;;
    --user)
      TARGET_USER="${2:-}"
      shift 2
      ;;
    --keep-artifacts)
      KEEP_ARTIFACTS=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 2
  fi
}

if [ "$(uname -s)" != "Linux" ]; then
  echo "this check only runs on Linux" >&2
  exit 2
fi

if [ "${EUID}" -ne 0 ]; then
  echo "this check must run through sudo so it can manage /etc/systemd/system" >&2
  exit 2
fi

if [ -z "${TARGET_USER}" ] || [ "${TARGET_USER}" = "root" ]; then
  echo "could not infer the installing user; rerun with --user <name>" >&2
  exit 2
fi

if [ -z "${SERVER_DIR}" ]; then
  echo "--server-dir is required" >&2
  exit 2
fi

if [ ! -d "${SERVER_DIR}" ]; then
  echo "server directory does not exist: ${SERVER_DIR}" >&2
  exit 2
fi

if [ ! -f "${SERVER_DIR}/server.properties" ]; then
  echo "server directory is missing server.properties: ${SERVER_DIR}" >&2
  exit 2
fi

if ! command -v systemctl >/dev/null 2>&1; then
  echo "systemctl is required" >&2
  exit 2
fi

if [ "$(systemctl is-system-running 2>/dev/null || true)" = "offline" ]; then
  echo "systemd is not running on this host" >&2
  exit 2
fi

TARGET_GROUP="$(id -gn "${TARGET_USER}")"

require_tool cargo
require_tool python3
require_tool curl
require_tool stat

PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"
BASE_URL="http://127.0.0.1:${PORT}"
SERVER_PORT="$(python3 - "${SERVER_DIR}/server.properties" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
port = "25565"
for line in path.read_text().splitlines():
    if line.startswith("server-port="):
        value = line.split("=", 1)[1].strip()
        if value:
            port = value
        break
print(port)
PY
)"

cleanup() {
  set +e
  if [ -x "${MSC_BIN}" ]; then
    "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" server stop "${SERVER_NAME}" >/dev/null 2>&1 || true
  fi
  systemctl stop "${LABEL}.service" >/dev/null 2>&1 || true
  systemctl disable "${LABEL}.service" >/dev/null 2>&1 || true
  rm -f "${AGENT_UNIT_PATH}"
  systemctl stop "${HELPER_SOCKET_UNIT}" >/dev/null 2>&1 || true
  systemctl disable "${HELPER_SOCKET_UNIT}" >/dev/null 2>&1 || true
  rm -f "${HELPER_SOCKET_PATH}" "${HELPER_SERVICE_PATH}"
  systemctl daemon-reload >/dev/null 2>&1 || true
  rm -rf /run/msc2 >/dev/null 2>&1 || true
  if [ "${KEEP_ARTIFACTS}" -eq 0 ]; then
    rm -rf "${RUN_DIR}"
  else
    echo "kept artifacts in ${RUN_DIR}" >&2
  fi
}
trap cleanup EXIT

mkdir -p "${RUN_DIR}/logs" "${RUN_DIR}/journal" "${RUN_DIR}/state"
chown -R "${TARGET_USER}:${TARGET_GROUP}" "${RUN_DIR}"
chmod -R 700 "${RUN_DIR}"

# On SELinux-enforcing hosts, a directory created here under an interactive
# sudo shell is type-transitioned to user_tmp_t, but the systemd-spawned
# agent process runs in init_t regardless of its configured User=. init_t
# cannot write user_tmp_t, so relabel to the generic tmp_t the unit needs.
if command -v selinuxenabled >/dev/null 2>&1 && selinuxenabled; then
  chcon -R -t tmp_t "${RUN_DIR}"
fi

(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

cat > "${AGENT_UNIT_PATH}" <<UNIT
[Unit]
Description=MSC 2 Phase 4 agent (${LABEL})
After=network.target

[Service]
Type=simple
User=${TARGET_USER}
Group=${TARGET_GROUP}
WorkingDirectory=${RUN_DIR}/state
ExecStart=${MSC_BIN} serve --bind 127.0.0.1:${PORT}
Environment=MSC2_TEST_BOOTSTRAP_TOKEN=${TOKEN}
Environment=MSC2_OPERATION_JOURNAL_DIR=${RUN_DIR}/journal
Environment=MSC2_EXPECTED_PORT=${PORT}
StandardOutput=append:${RUN_DIR}/logs/agent.log
StandardError=append:${RUN_DIR}/logs/agent.log
Restart=no

[Install]
WantedBy=multi-user.target
UNIT

chmod 644 "${AGENT_UNIT_PATH}"

mkdir -p /run/msc2 "${HELPER_STORE_DIR}"
chown root:root /run/msc2 "${HELPER_STORE_DIR}"
chmod 755 /run/msc2
chmod 700 "${HELPER_STORE_DIR}"

cat > "${HELPER_SOCKET_PATH}" <<UNIT
[Unit]
Description=MSC 2 credential helper socket

[Socket]
ListenStream=${HELPER_RUNTIME_SOCKET}
SocketUser=${TARGET_USER}
SocketGroup=${TARGET_GROUP}
SocketMode=0600
RemoveOnStop=yes

[Install]
WantedBy=sockets.target
UNIT

cat > "${HELPER_SERVICE_PATH}" <<UNIT
[Unit]
Description=MSC 2 credential helper
Requires=${HELPER_SOCKET_UNIT}
After=network.target

[Service]
Type=simple
User=root
Group=root
ExecStart=${MSC_BIN} credential-helper serve --allowed-uid $(id -u "${TARGET_USER}") --store-dir ${HELPER_STORE_DIR}
StandardInput=socket
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=${HELPER_STORE_DIR}
RuntimeDirectory=msc2
UNIT

chmod 644 "${HELPER_SOCKET_PATH}" "${HELPER_SERVICE_PATH}"

systemctl daemon-reload
systemctl enable "${LABEL}.service" >/dev/null
systemctl enable "${HELPER_SOCKET_UNIT}" >/dev/null
systemctl start "${HELPER_SOCKET_UNIT}"
systemctl start "${LABEL}.service"

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
raise SystemExit("agent did not become healthy through systemd")
PY

"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" server import "${SERVER_DIR}" --name "${SERVER_NAME}" >/dev/null
"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" server start "${SERVER_NAME}" >/dev/null

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 60
while time.time() < deadline:
    output = subprocess.check_output(
        [msc, "--base-url", base_url, "--token", token, "--json", "status"],
        text=True,
    )
    if json.loads(output)["running"]:
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("server never reached running state through systemd")
PY

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 60
while time.time() < deadline:
    output = subprocess.check_output(
        [
            msc,
            "--base-url",
            base_url,
            "--token",
            token,
            "--json",
            "console",
            "tail",
            "--server",
            "Phase4 Paper",
            "--lines",
            "200",
        ],
        text=True,
    )
    lines = json.loads(output)
    if any("Done (" in line["text"] and "For help, type" in line["text"] for line in lines):
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("server never emitted a Paper ready line")
PY

"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" command --server "${SERVER_NAME}" "say systemd smoke test" >/dev/null

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 30
while time.time() < deadline:
    output = subprocess.check_output(
        [
            msc,
            "--base-url",
            base_url,
            "--token",
            token,
            "--json",
            "console",
            "tail",
            "--server",
            "Phase4 Paper",
            "--lines",
            "100",
        ],
        text=True,
    )
    lines = json.loads(output)
    if any("systemd smoke test" in line["text"] for line in lines):
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("console tail never observed the sent command")
PY

AGENT_PID="$(systemctl show "${LABEL}.service" --property MainPID --value)"
SERVER_PID="$(ss -ltnp "sport = :${SERVER_PORT}" 2>/dev/null | awk -F'pid=' 'NR>1 {split($2,a,","); print a[1]; exit}' || true)"
if [ -z "${AGENT_PID}" ] || [ "${AGENT_PID}" = "0" ]; then
  echo "could not determine systemd agent pid" >&2
  exit 1
fi
if [ -z "${SERVER_PID}" ]; then
  echo "could not determine Paper server pid on port ${SERVER_PORT}" >&2
  exit 1
fi

sleep 2
kill -0 "${AGENT_PID}"
kill -0 "${SERVER_PID}"

curl --fail --silent --show-error -H "Authorization: Bearer ${TOKEN}" "${BASE_URL}/v1/status" >/dev/null

if [ ! -S "${HELPER_RUNTIME_SOCKET}" ]; then
  echo "credential-helper socket does not exist: ${HELPER_RUNTIME_SOCKET}" >&2
  exit 1
fi
if [ "$(stat -c %U "${HELPER_RUNTIME_SOCKET}")" != "${TARGET_USER}" ]; then
  echo "credential-helper socket owner mismatch" >&2
  exit 1
fi
if [ "$(stat -c %G "${HELPER_RUNTIME_SOCKET}")" != "${TARGET_GROUP}" ]; then
  echo "credential-helper socket group mismatch" >&2
  exit 1
fi
if [ "$(stat -c %a "${HELPER_RUNTIME_SOCKET}")" != "600" ]; then
  echo "credential-helper socket mode is not 0600" >&2
  exit 1
fi
if [ "$(stat -c %a "${HELPER_STORE_DIR}")" != "700" ]; then
  echo "credential-helper store dir mode is not 0700" >&2
  exit 1
fi

"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" server stop "${SERVER_NAME}" >/dev/null

python3 - "${MSC_BIN}" "${BASE_URL}" "${TOKEN}" <<'PY'
import json
import subprocess
import sys
import time

msc, base_url, token = sys.argv[1:4]
deadline = time.time() + 45
while time.time() < deadline:
    output = subprocess.check_output(
        [msc, "--base-url", base_url, "--token", token, "--json", "status"],
        text=True,
    )
    if not json.loads(output)["running"]:
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("server never reached stopped state")
PY

systemctl stop "${LABEL}.service"
systemctl disable "${LABEL}.service" >/dev/null
rm -f "${AGENT_UNIT_PATH}"
systemctl stop "${HELPER_SOCKET_UNIT}"
systemctl disable "${HELPER_SOCKET_UNIT}" >/dev/null
rm -f "${HELPER_SOCKET_PATH}" "${HELPER_SERVICE_PATH}"
systemctl daemon-reload

python3 - "${BASE_URL}" <<'PY'
import sys
import time
import urllib.error
import urllib.request

base_url = sys.argv[1]
deadline = time.time() + 20
while time.time() < deadline:
    try:
        urllib.request.urlopen(base_url + "/v1/health", timeout=1)
    except (urllib.error.URLError, TimeoutError):
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("agent still answered health after systemd uninstall")
PY

echo "Linux systemd lifecycle check passed"
