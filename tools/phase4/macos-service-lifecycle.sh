#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="$(/bin/date +%Y%m%d%H%M%S)-$$"
RUN_DIR="/private/tmp/msc2-macos-service-lifecycle.${RUN_ID}"
TARGET_USER="${SUDO_USER:-${USER:-}}"
KEEP_ARTIFACTS=0
TCC_DIR=""
SERVER_DIR=""
LABEL="com.msc2.phase4.agent.${RUN_ID}"
PLIST_PATH="/Library/LaunchDaemons/${LABEL}.plist"
PORT=""
BASE_URL=""
TOKEN="msc2_phase4_launchdaemon_secret"
MSC_BIN="${ROOT}/target/debug/msc"
SERVER_NAME="Phase4 Paper"
SERVER_PORT=""

usage() {
  cat <<USAGE
Usage: $0 --server-dir <path> [--user <installing-user>] [--tcc-dir <path>] [--keep-artifacts]

Builds the Phase 4 agent, installs it as a root-owned LaunchDaemon with
UserName set to the installing user, imports the given Paper server through
the public CLI/API path, starts it, proves the agent and Java server survive
with no clients connected, runs the P4.4 LaunchDaemon keychain/TCC check, then
stops the server and uninstalls the LaunchDaemon cleanly.
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
    --tcc-dir)
      TCC_DIR="${2:-}"
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

user_home() {
  local user="$1"
  local home
  home="$(/usr/bin/dscl . -read "/Users/${user}" NFSHomeDirectory 2>/dev/null | /usr/bin/awk '{print $2}')"
  if [ -z "${home}" ]; then
    home="$(eval "printf '%s' ~${user}")"
  fi
  printf '%s' "${home}"
}

if [ "$(/usr/bin/uname -s)" != "Darwin" ]; then
  echo "this check only runs on macOS" >&2
  exit 2
fi

if [ "${EUID}" -ne 0 ]; then
  echo "this check must run through sudo so it can manage /Library/LaunchDaemons" >&2
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

if [ -z "${TCC_DIR}" ]; then
  TCC_DIR="$(user_home "${TARGET_USER}")/Documents/MSC2LaunchDaemonTccCheck"
fi

require_tool cargo
require_tool python3
require_tool curl

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
  /bin/launchctl bootout system "${PLIST_PATH}" >/dev/null 2>&1 || true
  /bin/rm -f "${PLIST_PATH}"
  if [ "${KEEP_ARTIFACTS}" -eq 0 ]; then
    /bin/rm -rf "${RUN_DIR}"
  else
    echo "kept artifacts in ${RUN_DIR}" >&2
  fi
}
trap cleanup EXIT

/bin/mkdir -p "${RUN_DIR}/logs" "${RUN_DIR}/journal" "${RUN_DIR}/state"
/usr/sbin/chown -R "${TARGET_USER}:$(/usr/bin/id -gn "${TARGET_USER}")" "${RUN_DIR}"
/bin/chmod -R 700 "${RUN_DIR}"

(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

python3 - "${PLIST_PATH}" "${LABEL}" "${TARGET_USER}" "${MSC_BIN}" "${PORT}" "${RUN_DIR}" "${TOKEN}" <<'PY'
import plistlib
import sys

plist_path, label, user, binary, port, run_dir, token = sys.argv[1:8]
plist = {
    "Label": label,
    "ProgramArguments": [binary, "serve", "--bind", f"127.0.0.1:{port}"],
    "WorkingDirectory": run_dir + "/state",
    "UserName": user,
    "EnvironmentVariables": {
        "MSC2_TEST_BOOTSTRAP_TOKEN": token,
        "MSC2_OPERATION_JOURNAL_DIR": run_dir + "/journal",
        "RUST_LOG": "info",
        "MSC2_EXPECTED_PORT": port,
    },
    "StandardOutPath": run_dir + "/logs/agent.log",
    "StandardErrorPath": run_dir + "/logs/agent.log",
    "RunAtLoad": False,
    "KeepAlive": False,
}
with open(plist_path, "wb") as handle:
    plistlib.dump(plist, handle, sort_keys=False)
PY

/usr/sbin/chown root:wheel "${PLIST_PATH}"
/bin/chmod 644 "${PLIST_PATH}"

echo "bootstrapping LaunchDaemon ${LABEL}"
/bin/launchctl bootstrap system "${PLIST_PATH}"

# `bootstrap` can return before launchd has fully committed the job into
# its table, so an immediate `start` can race it and fail with ESRCH ("No
# such process") even though the job was just registered successfully.
# Poll `launchctl print` (which only succeeds once the job is visible)
# before calling `start`, instead of assuming `bootstrap` is synchronous.
for attempt in $(seq 1 20); do
  if /bin/launchctl print "system/${LABEL}" >/dev/null 2>&1; then
    break
  fi
  if [ "${attempt}" -eq 20 ]; then
    echo "LaunchDaemon ${LABEL} never became visible to launchctl after bootstrap" >&2
    exit 1
  fi
  /bin/sleep 0.25
done

/bin/launchctl start "system/${LABEL}"

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
raise SystemExit("agent did not become healthy through LaunchDaemon")
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
raise SystemExit("server never reached running state through LaunchDaemon")
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

"${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" command --server "${SERVER_NAME}" "say launchdaemon smoke test" >/dev/null

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
    if any("launchdaemon smoke test" in line["text"] for line in lines):
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("console tail never observed the sent command")
PY

AGENT_PID="$(/bin/launchctl print "system/${LABEL}" | /usr/bin/awk '/pid = / {print $3; exit}')"
SERVER_PID="$(/usr/sbin/lsof -nP -iTCP:"${SERVER_PORT}" -sTCP:LISTEN -t 2>/dev/null | /usr/bin/head -n 1 || true)"
if [ -z "${AGENT_PID}" ]; then
  echo "could not determine LaunchDaemon agent pid" >&2
  exit 1
fi
if [ -z "${SERVER_PID}" ]; then
  echo "could not determine Paper server pid on port ${SERVER_PORT}" >&2
  exit 1
fi

/bin/sleep 2
/bin/kill -0 "${AGENT_PID}"
/bin/kill -0 "${SERVER_PID}"

curl --fail --silent --show-error -H "Authorization: Bearer ${TOKEN}" "${BASE_URL}/v1/status" >/dev/null

sudo tools/phase4/macos-launchdaemon-check.sh --user "${TARGET_USER}" --tcc-dir "${TCC_DIR}" >/dev/null

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

/bin/launchctl bootout system "${PLIST_PATH}"
/bin/rm -f "${PLIST_PATH}"

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
raise SystemExit("agent still answered health after LaunchDaemon uninstall")
PY

echo "macOS LaunchDaemon lifecycle check passed"
