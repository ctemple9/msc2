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
MACOS_ROOT_SERVICE="com.msc2.agent.root.${RUN_ID}"
MACOS_ROOT_ACCOUNT="credential-root-v1"
MACOS_SECRET_STORE_DIR="${RUN_DIR}/state/secrets"
MACOS_DATA_DIR="${RUN_DIR}/state/data"
EVIDENCE_DIR="${ROOT}/docs/msc2/lifecycle/credential-evidence"
EVIDENCE_FILE="${EVIDENCE_DIR}/macos-${RUN_ID}.json"

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
  /usr/bin/security delete-generic-password -s "${MACOS_ROOT_SERVICE}" -a "${MACOS_ROOT_ACCOUNT}" /Library/Keychains/System.keychain >/dev/null 2>&1 || true
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
/bin/mkdir -p "${MACOS_SECRET_STORE_DIR}" "${MACOS_DATA_DIR}"
/usr/sbin/chown -R "${TARGET_USER}:$(/usr/bin/id -gn "${TARGET_USER}")" "${MACOS_SECRET_STORE_DIR}" "${MACOS_DATA_DIR}"
/bin/chmod 700 "${MACOS_SECRET_STORE_DIR}" "${MACOS_DATA_DIR}"

(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

# `cargo build` produces an unsigned binary (re-linking strips any
# signature applied to a previous build of the same file, even on a
# cached/no-op build), and launchd refuses to actually spawn an unsigned
# executable as a daemon — confirmed directly: `sudo msc serve ...` from
# a shell runs an unsigned binary fine, but the same binary registered
# and started through `launchctl` fails silently with exit code 3 (ESRCH,
# "no such process" — launchd never gets far enough to report a clearer
# reason). An ad-hoc signature (no paid developer account needed) is
# enough; apply one every run, right after the build that could have
# invalidated it.
/usr/bin/codesign -s - --force "${MSC_BIN}"

ROOT_KEY="$(python3 - <<'PY'
import secrets
print(secrets.token_hex(32))
PY
)"

/usr/bin/security add-generic-password \
  -U \
  -s "${MACOS_ROOT_SERVICE}" \
  -a "${MACOS_ROOT_ACCOUNT}" \
  -w "${ROOT_KEY}" \
  -T "${MSC_BIN}" \
  /Library/Keychains/System.keychain >/dev/null
unset ROOT_KEY

python3 - "${PLIST_PATH}" "${LABEL}" "${TARGET_USER}" "${MSC_BIN}" "${PORT}" "${RUN_DIR}" "${TOKEN}" "${MACOS_ROOT_SERVICE}" "${MACOS_ROOT_ACCOUNT}" "${MACOS_SECRET_STORE_DIR}" "${MACOS_DATA_DIR}" <<'PY'
import plistlib
import sys

(
    plist_path,
    label,
    user,
    binary,
    port,
    run_dir,
    token,
    root_service,
    root_account,
    secret_store_dir,
    data_dir,
) = sys.argv[1:12]
plist = {
    "Label": label,
    "ProgramArguments": [binary, "serve", "--bind", f"127.0.0.1:{port}"],
    "WorkingDirectory": run_dir + "/state",
    "UserName": user,
    "EnvironmentVariables": {
        "MSC2_TEST_BOOTSTRAP_TOKEN": token,
        "MSC2_OPERATION_JOURNAL_DIR": run_dir + "/journal",
        "MSC2_MACOS_SECRET_ROOT_SERVICE": root_service,
        "MSC2_MACOS_SECRET_ROOT_ACCOUNT": root_account,
        "MSC2_MACOS_SECRET_STORE_DIR": secret_store_dir,
        "MSC2_DATA_DIR": data_dir,
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

# Confirm the job is visible before starting it — cheap, and a clearer
# failure than whatever `start` would do against a job that never loaded.
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

# `start`/`stop` are the legacy launchctl subcommand family and take a
# bare label, unlike `bootstrap`/`bootout`/`print`'s `<domain>/<label>`
# target syntax — confirmed directly against real launchd: `launchctl
# start system/<label>` fails silently with exit 3 (ESRCH, "no such
# process") even though the identical job responds fine to `print
# system/<label>`; `launchctl start <label>` on that same job succeeds.
/bin/launchctl start "${LABEL}"

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

curl --fail --silent --show-error -H "Authorization: Bearer ${TOKEN}" "${BASE_URL}/v1/status" >/dev/null
AGENT_PID_BEFORE_RESTART="$(/bin/launchctl print "system/${LABEL}" | /usr/bin/awk '/pid = / {print $3; exit}')"
if [ -z "${AGENT_PID_BEFORE_RESTART}" ]; then
  echo "could not determine LaunchDaemon agent pid before credential restart proof" >&2
  exit 1
fi

python3 - "${PLIST_PATH}" <<'PY'
import plistlib
import sys

path = sys.argv[1]
with open(path, "rb") as handle:
    plist = plistlib.load(handle)
env = plist.get("EnvironmentVariables", {})
env.pop("MSC2_TEST_BOOTSTRAP_TOKEN", None)
plist["EnvironmentVariables"] = env
with open(path, "wb") as handle:
    plistlib.dump(plist, handle, sort_keys=False)
PY
/usr/sbin/chown root:wheel "${PLIST_PATH}"
/bin/chmod 644 "${PLIST_PATH}"

/bin/launchctl bootout system "${PLIST_PATH}"
/bin/launchctl bootstrap system "${PLIST_PATH}"
for attempt in $(seq 1 20); do
  if /bin/launchctl print "system/${LABEL}" >/dev/null 2>&1; then
    break
  fi
  if [ "${attempt}" -eq 20 ]; then
    echo "LaunchDaemon ${LABEL} never became visible after credential restart bootstrap" >&2
    exit 1
  fi
  /bin/sleep 0.25
done
/bin/launchctl start "${LABEL}"

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
raise SystemExit("agent did not become healthy after LaunchDaemon credential restart")
PY
curl --fail --silent --show-error -H "Authorization: Bearer ${TOKEN}" "${BASE_URL}/v1/status" >/dev/null
AGENT_PID_AFTER_RESTART="$(/bin/launchctl print "system/${LABEL}" | /usr/bin/awk '/pid = / {print $3; exit}')"
if [ -z "${AGENT_PID_AFTER_RESTART}" ] || [ "${AGENT_PID_AFTER_RESTART}" = "${AGENT_PID_BEFORE_RESTART}" ]; then
  echo "LaunchDaemon restart did not produce a new agent pid" >&2
  exit 1
fi

/bin/mkdir -p "${EVIDENCE_DIR}"
python3 - "${EVIDENCE_FILE}" "${RUN_DIR}" "${RUN_ID}" "${LABEL}" "${AGENT_PID_BEFORE_RESTART}" "${AGENT_PID_AFTER_RESTART}" <<'PY'
import datetime
import json
import sys

path, run_dir, run_id, label, before_pid, after_pid = sys.argv[1:7]
record = {
    "artifactDir": run_dir,
    "bootstrapTokenRemovedBeforeRestart": True,
    "credentialPath": "test bootstrap token registered through production service startup",
    "credentialStoredInProductionStore": True,
    "platform": "macos",
    "processEvidence": {
        "beforeRestartPid": before_pid,
        "afterRestartPid": after_pid,
    },
    "protectedRequestAfterRestart": True,
    "protectedRequestBeforeRestart": True,
    "recordedAt": datetime.datetime.now(datetime.UTC).isoformat(),
    "restartedActualServiceProcess": True,
    "result": "passed",
    "runId": run_id,
    "schema": "msc2.phase4.credential-evidence.v1",
    "script": "tools/phase4/macos-service-lifecycle.sh",
    "serviceManager": "launchd",
    "serviceName": label,
    "tokenMaterialRecorded": False,
}
with open(path, "w", encoding="utf-8") as handle:
    json.dump(record, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
/usr/sbin/chown "${TARGET_USER}" "${EVIDENCE_FILE}" >/dev/null 2>&1 || true

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
