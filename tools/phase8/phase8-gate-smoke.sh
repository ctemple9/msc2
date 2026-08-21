#!/usr/bin/env bash
set -euo pipefail

# Phase 8's one portable synthetic public-path smoke.  This intentionally
# uses no public network: the local provider below is only a Modrinth-shaped
# loopback server, while the Rust test suites use their own disk-backed state
# and fake transports.  Together they cover the public HTTP/CLI boundary and
# the mutation/rollback branches that cannot safely be duplicated in shell.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODE=""
TMP_DIR=""
PROVIDER_PID=""

usage() {
  echo "Usage: $0 --synthetic"
}

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cleanup() {
  if [[ -n "${PROVIDER_PID}" ]] && kill -0 "${PROVIDER_PID}" 2>/dev/null; then
    kill "${PROVIDER_PID}" 2>/dev/null || true
    wait "${PROVIDER_PID}" 2>/dev/null || true
  fi
  [[ -z "${TMP_DIR}" ]] || rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --synthetic) MODE="synthetic" ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
  shift
done
[[ "${MODE}" == "synthetic" ]] || { usage >&2; exit 2; }

for tool in cargo python3 curl; do
  command -v "${tool}" >/dev/null 2>&1 || fail "missing required tool: ${tool}"
done

free_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/msc2-phase8-gate.XXXXXX")"
PORT="$(free_port)"
BASE_URL="http://127.0.0.1:${PORT}"

echo "== validating synthetic pack fixture =="
python3 - "${ROOT}/tools/phase8/fixtures/tiny-modrinth.index.json" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as f:
    pack = json.load(f)
assert pack["game"] == "minecraft"
assert pack["dependencies"]["fabric-loader"]
PY

echo "== starting local fake provider =="
python3 "${ROOT}/tools/phase8/fake-provider-server.py" --port "${PORT}" >"${TMP_DIR}/provider.log" 2>&1 &
PROVIDER_PID="$!"
for _ in $(seq 1 80); do
  if curl --fail --silent "${BASE_URL}/__ready__" >/dev/null; then break; fi
  sleep 0.1
done
curl --fail --silent "${BASE_URL}/__ready__" >/dev/null || fail "fake provider did not start"
curl --fail --silent "${BASE_URL}/v2/search?query=synthetic" | python3 -c 'import json,sys; assert json.load(sys.stdin)["hits"][0]["project_id"] == "synthetic-project"'
curl --fail --silent "${BASE_URL}/v2/project/synthetic-project/version" | python3 -c 'import json,sys; assert json.load(sys.stdin)[0]["files"][0]["hashes"]["sha1"]'

echo "== exercising synthetic Phase 8 behavior =="
cd "${ROOT}"
# The domain/storage/application cases cover dependency cycles, corrupt hashes,
# hostile archives, manual CurseForge completion, pack guards, export, repair,
# cancellation, restart reconciliation, and residue cleanup.  `phase8_routes`
# and `cli_phase8` drive the same services through real HTTP and CLI calls.
cargo nextest run -p msc-domain --test addon_dependency --test modpack_policy
cargo nextest run -p msc-infrastructure --test addon_provider --test addon_store
cargo nextest run -p msc-application --test addons --test addon_dependencies --test addon_updates --test modpack_inspection --test modpack_server_creation --test client_export --test diagnostics
cargo nextest run -p msc-agent --test phase8_routes --test cli_phase8

echo "PHASE 8 GATE SMOKE PASSED"
