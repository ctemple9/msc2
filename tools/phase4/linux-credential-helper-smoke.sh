#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_USER="${SUDO_USER:-${USER}}"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required" >&2
    exit 2
  fi
}

if [ "$(uname -s)" != "Linux" ]; then
  echo "linux-credential-helper-smoke.sh must run on Linux" >&2
  exit 2
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "run with sudo so the helper can exercise root-only systemd-creds storage" >&2
  exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
  target_cargo_bin="$(getent passwd "${TARGET_USER}" | cut -d: -f6)/.cargo/bin"
  if [ -x "${target_cargo_bin}/cargo" ]; then
    PATH="${PATH}:${target_cargo_bin}"
  fi
fi

require_tool cargo
require_tool python3
require_tool stat
require_tool sudo
require_tool systemd-creds

RUN_DIR="$(mktemp -d /tmp/msc2-linux-credential-helper.XXXXXX)"
# mktemp -d makes RUN_DIR mode 0700 owned by root (this whole script runs
# under sudo). The unprivileged TARGET_USER needs to traverse into it to
# reach the socket below; only the STORE_DIR subdirectory needs to stay
# root-only, and the helper backend already locks that down to 0700 itself.
chmod 755 "${RUN_DIR}"
STORE_DIR="${RUN_DIR}/credentials"
SOCKET_PATH="${RUN_DIR}/credential-helper.sock"
HELPER_LOG="${RUN_DIR}/helper.log"
HELPER_PID=""
ALLOWED_UID="$(id -u "${TARGET_USER}")"

cleanup() {
  local exit_code="$?"
  set +e
  if [ "${exit_code}" -ne 0 ] && [ -f "${HELPER_LOG}" ]; then
    echo "--- helper log (exit ${exit_code}) ---" >&2
    cat "${HELPER_LOG}" >&2
  fi
  if [ -n "${HELPER_PID}" ]; then
    kill "${HELPER_PID}" >/dev/null 2>&1 || true
    wait "${HELPER_PID}" >/dev/null 2>&1 || true
  fi
  rm -rf "${RUN_DIR}"
}
trap cleanup EXIT

(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

MSC_BIN="${ROOT}/target/debug/msc"

start_helper() {
  rm -f "${SOCKET_PATH}"
  "${MSC_BIN}" credential-helper serve \
    --allowed-uid "${ALLOWED_UID}" \
    --store-dir "${STORE_DIR}" \
    --socket-path "${SOCKET_PATH}" >"${HELPER_LOG}" 2>&1 &
  HELPER_PID="$!"

  python3 - "${SOCKET_PATH}" "${HELPER_LOG}" <<'PY'
import pathlib
import sys
import time

socket_path = pathlib.Path(sys.argv[1])
log_path = pathlib.Path(sys.argv[2])
deadline = time.time() + 10
while time.time() < deadline:
    if socket_path.exists():
        raise SystemExit(0)
    if log_path.exists() and "error" in log_path.read_text(errors="ignore").lower():
        raise SystemExit(log_path.read_text(errors="ignore"))
    time.sleep(0.05)
raise SystemExit(f"credential helper socket did not appear: {socket_path}")
PY
}

stop_helper() {
  if [ -n "${HELPER_PID}" ]; then
    kill "${HELPER_PID}" >/dev/null 2>&1 || true
    wait "${HELPER_PID}" >/dev/null 2>&1 || true
    HELPER_PID=""
  fi
  rm -f "${SOCKET_PATH}"
}

start_helper

if [ "$(stat -c %U "${SOCKET_PATH}")" != "${TARGET_USER}" ]; then
  echo "temporary helper socket owner mismatch" >&2
  exit 1
fi
if [ "$(stat -c %a "${SOCKET_PATH}")" != "600" ]; then
  echo "temporary helper socket mode is not 0600" >&2
  exit 1
fi

sudo -u "${TARGET_USER}" python3 - "${SOCKET_PATH}" <<'PY'
import json
import socket
import sys

socket_path = sys.argv[1]

def request(payload):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.connect(socket_path)
        sock.sendall(json.dumps(payload, separators=(",", ":")).encode() + b"\n")
        data = b""
        while not data.endswith(b"\n"):
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
    return json.loads(data.decode())

def check(actual, expected):
    assert actual == expected, f"expected {expected!r}, got {actual!r}"

check(request({"version": 1, "op": "ping"}), {"ok": True})
check(request({"version": 1, "op": "set", "key": "remote-api.token.smoke", "value": "first-value"}), {"ok": True})
check(request({"version": 1, "op": "get", "key": "remote-api.token.smoke"}), {"ok": True, "value": "first-value"})
check(request({"version": 1, "op": "set", "key": "remote-api.token.smoke", "value": "second-value"}), {"ok": True})
check(request({"version": 1, "op": "get", "key": "remote-api.token.smoke"}), {"ok": True, "value": "second-value"})
PY

if [ ! -f "${STORE_DIR}/remote-api.token.smoke.cred" ]; then
  echo "encrypted credential blob was not written" >&2
  exit 1
fi
if grep -a "second-value" "${STORE_DIR}/remote-api.token.smoke.cred" >/dev/null; then
  echo "encrypted credential blob contains plaintext" >&2
  exit 1
fi

python3 - "${SOCKET_PATH}" <<'PY'
import json
import socket
import sys

with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
    sock.connect(sys.argv[1])
    sock.sendall(b'{"version":1,"op":"ping"}\n')
    data = sock.recv(4096)

response = json.loads(data.decode())
if response.get("ok") is not False or response.get("error", {}).get("code") != "forbidden_uid":
    raise SystemExit(f"root peer was not rejected by credential helper: {response}")
PY

stop_helper
start_helper

sudo -u "${TARGET_USER}" python3 - "${SOCKET_PATH}" <<'PY'
import json
import socket
import sys

socket_path = sys.argv[1]

def request(payload):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.connect(socket_path)
        sock.sendall(json.dumps(payload, separators=(",", ":")).encode() + b"\n")
        data = b""
        while not data.endswith(b"\n"):
            chunk = sock.recv(4096)
            if not chunk:
                break
            data += chunk
    return json.loads(data.decode())

def check(actual, expected):
    assert actual == expected, f"expected {expected!r}, got {actual!r}"

check(request({"version": 1, "op": "get", "key": "remote-api.token.smoke"}), {"ok": True, "value": "second-value"})
check(request({"version": 1, "op": "delete", "key": "remote-api.token.smoke"}), {"ok": True})
check(request({"version": 1, "op": "get", "key": "remote-api.token.smoke"}), {"ok": True})
PY

if [ -e "${STORE_DIR}/remote-api.token.smoke.cred" ]; then
  echo "credential-helper delete left a stored blob behind" >&2
  exit 1
fi

echo "linux credential helper smoke passed"
