#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REAL_CONFIG=""
REAL_TRANSFER=""

usage() {
  cat <<USAGE
Usage: $0 --real-config <server_config_swift.json> --real-transfer <package.msctransfer>

Runs the Phase 5 public-path gate smoke on this host. The real config and
transfer package are read from their original locations but copied/imported
through isolated MSC 2 data roots.
USAGE
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --real-config)
      REAL_CONFIG="${2:-}"
      shift 2
      ;;
    --real-transfer)
      REAL_TRANSFER="${2:-}"
      shift 2
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

if [[ -z "${REAL_CONFIG}" || -z "${REAL_TRANSFER}" ]]; then
  usage >&2
  exit 2
fi
if [[ ! -f "${REAL_CONFIG}" ]]; then
  echo "real config not found: ${REAL_CONFIG}" >&2
  exit 1
fi
if [[ ! -f "${REAL_TRANSFER}" ]]; then
  echo "real transfer package not found: ${REAL_TRANSFER}" >&2
  exit 1
fi

MSC_BIN="${ROOT}/target/debug/msc"
TOKEN="msc2_phase5_gate_bootstrap_secret"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/msc2-phase5-gate.XXXXXX")"
AGENT_PID=""
KEYCHAIN_SERVICE="com.msc2.phase5.gate.$(date +%Y%m%d%H%M%S).$$"

cleanup() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  if [[ "$(uname -s)" == "Darwin" ]]; then
    /usr/bin/security delete-generic-password \
      -s "${KEYCHAIN_SERVICE}" \
      -a "remote-api.token.phase5" >/dev/null 2>&1 || true
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

free_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

wait_for_agent_healthy() {
  local base_url="$1"
  python3 - "${base_url}" <<'PY'
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

start_agent() {
  local port="$1" data_dir="$2" config_path="$3" servers_root="$4" log_path="$5"
  export MSC2_TEST_BOOTSTRAP_TOKEN="${TOKEN}"
  export MSC2_DATA_DIR="${data_dir}"
  export MSC2_APP_CONFIG_PATH="${config_path}"
  export MSC2_AGENT_SERVERS_ROOT="${servers_root}"
  export MSC2_CREDENTIAL_REGISTRY_PATH="${data_dir}/credential-registry.json"
  export MSC2_OPERATION_JOURNAL_DIR="${data_dir}/journal"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    export MSC2_MACOS_USER_KEYCHAIN_SERVICE="${KEYCHAIN_SERVICE}"
  fi
  mkdir -p "${data_dir}" "${servers_root}" "${MSC2_OPERATION_JOURNAL_DIR}"
  "${MSC_BIN}" serve --bind "127.0.0.1:${port}" >"${log_path}" 2>&1 &
  AGENT_PID="$!"
  wait_for_agent_healthy "http://127.0.0.1:${port}"
}

stop_agent() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  AGENT_PID=""
}

require_tool cargo
require_tool python3

(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

tools/phase5/cli-smoke.sh --migration-restart
tools/phase5/cli-smoke.sh --settings --raw --import-lifecycle
tools/phase5/cli-smoke.sh --rescan
tools/phase5/cli-smoke.sh --replace-all

REAL_CONFIG_ROOT="${TMP_DIR}/real-config"
REAL_CONFIG_DATA="${REAL_CONFIG_ROOT}/data"
REAL_CONFIG_SERVERS="${REAL_CONFIG_ROOT}/servers"
REAL_CONFIG_PATH="${REAL_CONFIG_DATA}/server_config_swift.json"
mkdir -p "${REAL_CONFIG_DATA}" "${REAL_CONFIG_SERVERS}"
cp "${REAL_CONFIG}" "${REAL_CONFIG_PATH}"
REAL_CONFIG_PORT="$(free_port)"
start_agent "${REAL_CONFIG_PORT}" "${REAL_CONFIG_DATA}" "${REAL_CONFIG_PATH}" "${REAL_CONFIG_SERVERS}" "${REAL_CONFIG_ROOT}/agent.log"
"${MSC_BIN}" --base-url "http://127.0.0.1:${REAL_CONFIG_PORT}" --token "${TOKEN}" --json status >/dev/null
python3 - "http://127.0.0.1:${REAL_CONFIG_PORT}" "${TOKEN}" <<'PY'
import json
import sys
import urllib.request

base_url, token = sys.argv[1:3]
request = urllib.request.Request(
    base_url + "/v1/servers", headers={"Authorization": f"Bearer {token}"}
)
with urllib.request.urlopen(request, timeout=5) as resp:
    servers = json.load(resp)
if not isinstance(servers, list):
    raise SystemExit(f"expected server list from real config startup, got {servers!r}")
PY
stop_agent

REAL_TRANSFER_ROOT="${TMP_DIR}/real-transfer"
REAL_TRANSFER_DATA="${REAL_TRANSFER_ROOT}/data"
REAL_TRANSFER_SERVERS="${REAL_TRANSFER_ROOT}/servers"
REAL_TRANSFER_CONFIG="${REAL_TRANSFER_DATA}/server_config_swift.json"
REAL_TRANSFER_PORT="$(free_port)"
start_agent "${REAL_TRANSFER_PORT}" "${REAL_TRANSFER_DATA}" "${REAL_TRANSFER_CONFIG}" "${REAL_TRANSFER_SERVERS}" "${REAL_TRANSFER_ROOT}/agent.log"
MSC2_CLI_RESPONSE_TIMEOUT_SECS=300 "${MSC_BIN}" --base-url "http://127.0.0.1:${REAL_TRANSFER_PORT}" --token "${TOKEN}" --json \
  server import "${REAL_TRANSFER}" --kind transfer >/dev/null
python3 - "http://127.0.0.1:${REAL_TRANSFER_PORT}" "${TOKEN}" <<'PY'
import json
import sys
import urllib.request

base_url, token = sys.argv[1:3]
request = urllib.request.Request(
    base_url + "/v1/servers", headers={"Authorization": f"Bearer {token}"}
)
with urllib.request.urlopen(request, timeout=5) as resp:
    servers = json.load(resp)
if len(servers) < 1:
    raise SystemExit("real transfer public import produced no registered servers")
PY
stop_agent

python3 tools/phase5/real-corpus-check.py \
  --exercise \
  --configs-dir "$(dirname "${REAL_CONFIG}")" \
  --transfer-package "${REAL_TRANSFER}" \
  --require-configs 1 \
  --require-transfer

echo "phase5 gate smoke passed"
