#!/usr/bin/env bash
set -euo pipefail

# Phase 7 portable six-family provisioning and launch smoke (P7.27).
#
# Drives a real foreground `msc-agent` through nothing but the CLI and
# API -- the same public surface the copied iOS client and any other
# caller uses -- to create all six create-flow families (Vanilla, Paper,
# Purpur, Fabric, NeoForge, Forge), start each one, and prove the thing
# the port plan's own later-audit clause asks for: that Forge/NeoForge
# really launch from `@<args-file> nogui` (the file their installer
# generated) while the other four launch from `-jar <jar> --nogui`, not
# that they were merely classified.
#
# Portable and committed: no real network call happens anywhere in this
# run. `tools/phase7/fixtures/fake-provisioning/fake_provider_server.py`
# serves all six families' catalogs straight from `corpus/providers/`
# (P7.3's real, recorded evidence -- see that script's own doc for which
# parts are byte-for-byte real vs. locally rewritten) over a local
# loopback HTTP server, and a locally built fake server jar / fake
# installer jar (`FakeServer.java`/`FakeInstaller.java`, this directory)
# stand in for a real downloaded jar and a real Forge/NeoForge installer.
# `crates/msc-infrastructure/src/jar_provider.rs`'s per-family
# `MSC2_PROVIDER_*_BASE` env var overrides (added alongside this script)
# are what make a real, unmodified `msc-agent` binary reachable this way
# -- every URL path is otherwise exactly what a real provider would be
# asked for; only the host is redirected.
#
# Also exercises the failure side named in this step's own "What": an
# injected download failure and an injected installer failure each
# leave no server directory behind, and SIGKILLing the agent mid-install
# proves the operation journal reconciles the interrupted "server-create"
# operation to `failed` on restart -- the same generic
# `LifecycleOperations::reconcile_on_startup` mechanism
# `tools/phase6/phase6-gate-smoke.sh` already proved for world
# activation/restore, applied here to provisioning.
#
# Finally, imports a synthetic raw Forge-shaped directory (the P7.27
# working-exit-criteria's "a Phase 5-imported non-Paper server actually
# starts") to prove the new launch-shape dispatch this step's own
# smoke-writing surfaced a real gap in
# (`crates/msc-agent/src/routes/lifecycle.rs::build_launch_request` --
# see this script's own findings note below) works on an *imported*
# server too, not only one this same process just created.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURES_DIR="${ROOT}/tools/phase7/fixtures/fake-provisioning"
CORPUS_DIR="${ROOT}/corpus/providers"
MODE=""

usage() {
  cat <<USAGE
Usage: $0 --synthetic

--synthetic runs the full six-family portable provisioning/launch gate
smoke against a local fake provider and locally built fake jars -- no
real network call, no real MSC 1 data, safe to run anywhere.
USAGE
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --synthetic)
      MODE="synthetic"
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

if [[ "${MODE}" != "synthetic" ]]; then
  usage >&2
  exit 2
fi

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required tool: $1" >&2
    exit 1
  fi
}

require_tool cargo
require_tool python3
require_tool javac
require_tool jar
require_tool java

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

MSC_BIN="${ROOT}/target/debug/msc"
TOKEN="msc2_phase7_gate_bootstrap_secret"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/msc2-phase7-gate.XXXXXX")"
DATA_DIR="${TMP_DIR}/data"
SERVERS_ROOT="${TMP_DIR}/servers"
CONFIG_PATH="${DATA_DIR}/server_config_swift.json"
BUILD_DIR="${TMP_DIR}/build"
CONTROL_DIR="${TMP_DIR}/control"
KEYCHAIN_SERVICE="com.msc2.phase7.gate.$(date +%Y%m%d%H%M%S).$$"

AGENT_PID=""
PORT=""
BASE_URL=""
PROVIDER_PID=""
PROVIDER_PORT=""
PROVIDER_BASE=""

cleanup() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill -9 "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  if [[ -n "${PROVIDER_PID}" ]] && kill -0 "${PROVIDER_PID}" 2>/dev/null; then
    kill -9 "${PROVIDER_PID}" 2>/dev/null || true
    wait "${PROVIDER_PID}" 2>/dev/null || true
  fi
  if [[ "$(uname -s)" == "Darwin" ]]; then
    /usr/bin/security delete-generic-password \
      -s "${KEYCHAIN_SERVICE}" \
      -a "remote-api.token.phase7" >/dev/null 2>&1 || true
  fi
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

free_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

wait_for_url_ok() {
  local url="$1" label="$2" deadline
  deadline=$(( $(date +%s) + 45 ))
  while [[ "$(date +%s)" -lt "${deadline}" ]]; do
    if curl -s -o /dev/null -w '%{http_code}' --max-time 2 "${url}" 2>/dev/null | grep -q '^200$'; then
      return 0
    fi
    sleep 0.25
  done
  fail "${label} never became reachable at ${url}"
}

wait_for_path() {
  local pattern="$1" label="$2" deadline
  deadline=$(( $(date +%s) + 45 ))
  while [[ "$(date +%s)" -lt "${deadline}" ]]; do
    # shellcheck disable=SC2086
    if compgen -G "${pattern}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done
  fail "${label} never appeared: ${pattern}"
}

# =====================================================================
# 1. Build the two fake Java programs P7.13/P7.14's real network/
#    subprocess boundaries talk to: a fake server jar (stands in for a
#    real Vanilla/Paper/Purpur/Fabric download) and a fake Forge/
#    NeoForge installer template jar (one is built per download request
#    by fake_provider_server.py, each embedding which family/version
#    that particular request is for).
# =====================================================================
echo "== building fake server/installer jars =="
mkdir -p "${BUILD_DIR}"
javac -d "${BUILD_DIR}" "${FIXTURES_DIR}/FakeServer.java" "${FIXTURES_DIR}/FakeInstaller.java"
(
  cd "${BUILD_DIR}"
  printf 'Main-Class: FakeServer\n' > server-manifest.txt
  jar cfm fake-server.jar server-manifest.txt FakeServer.class
  printf 'Main-Class: FakeInstaller\n' > installer-manifest.txt
  jar cfm fake-installer-template.jar installer-manifest.txt FakeInstaller.class
)
FAKE_SERVER_JAR="${BUILD_DIR}/fake-server.jar"
FAKE_INSTALLER_TEMPLATE="${BUILD_DIR}/fake-installer-template.jar"

# =====================================================================
# 2. Start the local fake provider (corpus/providers/, over loopback --
#    see this script's own header and fake_provider_server.py's module
#    doc for exactly what's real vs. locally synthesized).
# =====================================================================
echo "== starting fake provider =="
mkdir -p "${CONTROL_DIR}"
PROVIDER_PORT="$(free_port)"
PROVIDER_BASE="http://127.0.0.1:${PROVIDER_PORT}"
python3 "${FIXTURES_DIR}/fake_provider_server.py" \
  --port "${PROVIDER_PORT}" \
  --corpus "${CORPUS_DIR}" \
  --server-jar "${FAKE_SERVER_JAR}" \
  --installer-template "${FAKE_INSTALLER_TEMPLATE}" \
  --control-dir "${CONTROL_DIR}" \
  --install-delay-ms 4000 \
  >>"${TMP_DIR}/provider.log" 2>&1 &
PROVIDER_PID="$!"
wait_for_url_ok "${PROVIDER_BASE}/__ready__" "fake provider"

# =====================================================================
# 3. Agent lifecycle helpers -- same shape as
#    tools/phase6/phase6-gate-smoke.sh's own start_agent/stop_agent/
#    run_msc/operation_json/assert_operation_interrupted_by_restart.
# =====================================================================
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
  export MSC2_TEST_BOOTSTRAP_TOKEN="${TOKEN}"
  export MSC2_DATA_DIR="${DATA_DIR}"
  export MSC2_APP_CONFIG_PATH="${CONFIG_PATH}"
  export MSC2_AGENT_SERVERS_ROOT="${SERVERS_ROOT}"
  export MSC2_CREDENTIAL_REGISTRY_PATH="${DATA_DIR}/credential-registry.json"
  export MSC2_OPERATION_JOURNAL_DIR="${DATA_DIR}/journal"
  export MSC2_AUDIT_LOG_DIR="${DATA_DIR}/audit-log"
  export MSC2_PROVIDER_VANILLA_BASE="${PROVIDER_BASE}"
  export MSC2_PROVIDER_PURPUR_BASE="${PROVIDER_BASE}"
  export MSC2_PROVIDER_PAPER_BASE="${PROVIDER_BASE}"
  export MSC2_PROVIDER_FABRIC_BASE="${PROVIDER_BASE}"
  export MSC2_PROVIDER_NEOFORGE_MAVEN_BASE="${PROVIDER_BASE}"
  export MSC2_PROVIDER_FORGE_MAVEN_BASE="${PROVIDER_BASE}"
  export MSC2_PROVIDER_FORGE_FILES_BASE="${PROVIDER_BASE}"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    export MSC2_MACOS_USER_KEYCHAIN_SERVICE="${KEYCHAIN_SERVICE}"
  elif [[ "$(uname -s)" == "Linux" ]]; then
    export MSC2_LINUX_FOREGROUND_SECRET_STORE_DIR="${DATA_DIR}/linux-secret-store"
  fi
  mkdir -p "${DATA_DIR}" "${SERVERS_ROOT}" "${MSC2_OPERATION_JOURNAL_DIR}" "${MSC2_AUDIT_LOG_DIR}"
  PORT="$(free_port)"
  BASE_URL="http://127.0.0.1:${PORT}"
  "${MSC_BIN}" serve --bind "127.0.0.1:${PORT}" >>"${TMP_DIR}/agent.log" 2>&1 &
  AGENT_PID="$!"
  wait_for_agent_healthy "${BASE_URL}"
}

stop_agent() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  AGENT_PID=""
}

kill_agent_hard() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill -9 "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  AGENT_PID=""
}

restart_agent() {
  stop_agent
  start_agent
}

run_msc() {
  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" "$@"
}

run_msc_json() {
  "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" --json "$@"
}

expect_fail() {
  if run_msc "$@" >/dev/null 2>>"${TMP_DIR}/agent.log"; then
    fail "expected failure but succeeded: $*"
  fi
}

operation_json() {
  local operation_id="$1"
  python3 - "${BASE_URL}" "${TOKEN}" "${operation_id}" <<'PY'
import sys
import urllib.request

base_url, token, operation_id = sys.argv[1:4]
req = urllib.request.Request(
    f"{base_url}/v1/operations/{operation_id}",
    headers={"Authorization": f"Bearer {token}"},
)
with urllib.request.urlopen(req, timeout=5) as resp:
    print(resp.read().decode())
PY
}

assert_operation_interrupted_by_restart() {
  # Same reconciliation this same journal already gives world activation/
  # restore (`tools/phase6/phase6-gate-smoke.sh`'s own
  # `assert_operation_interrupted_by_restart`), applied here to a
  # "server-create" operation: `LifecycleOperations::reconcile_on_startup`
  # marks any operation still `running` after a crash `failed`, with
  # `error.code == "operation_interrupted"`.
  local operation_id="$1" state code message record
  record="$(operation_json "${operation_id}")"
  state="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])' <<<"${record}")"
  code="$(python3 -c 'import json,sys; d=json.load(sys.stdin); print((d.get("error") or {}).get("code") or "")' <<<"${record}")"
  message="$(python3 -c 'import json,sys; d=json.load(sys.stdin); print((d.get("error") or {}).get("message") or "")' <<<"${record}")"
  [[ "${state}" == "failed" ]] || fail "operation ${operation_id}: expected reconciled state failed, got ${state}"
  [[ "${code}" == "operation_interrupted" ]] || fail "operation ${operation_id}: expected error code operation_interrupted, got ${code}"
  [[ "${message}" == *"restart"* ]] || fail "operation ${operation_id}: reconciled record does not explain the restart (message: ${message})"
  echo "operation ${operation_id} record explains the restart: ${message}"
}

wait_running_state() {
  local want="$1" deadline
  deadline=$(( $(date +%s) + 45 ))
  while [[ "$(date +%s)" -lt "${deadline}" ]]; do
    local running
    running="$(run_msc_json status | python3 -c 'import json,sys; print(json.load(sys.stdin)["running"])')"
    if [[ "${running}" == "${want}" ]]; then
      return 0
    fi
    sleep 0.25
  done
  fail "server never reached running=${want}"
}

console_lines_json() {
  run_msc_json console tail --lines 500
}

# The console buffer is cumulative across every server this same agent
# process has started, not cleared between servers -- so after several
# create/start/stop cycles it holds every earlier server's own boot
# lines too. Both console assertions below take the display `name` that
# was just started and scope themselves to lines *after* that server's
# own `"Starting server: <name>"` system line (written by
# `LifecycleService::start_active_server` at the top of every real
# start, before anything else), rather than searching the whole
# cumulative buffer -- otherwise a stale "Done"/`LAUNCH_ARGV:` line from
# an earlier, already-stopped server satisfies the check instantly,
# before the server actually being tested has printed anything of its
# own.
console_lines_since_start() {
  local name="$1"
  console_lines_json | python3 -c '
import json, sys
name = sys.argv[1]
lines = json.load(sys.stdin)
marker = f"Starting server: {name}"
last_start = None
for i, line in enumerate(lines):
    if line["text"] == marker:
        last_start = i
if last_start is None:
    sys.exit("no \"" + marker + "\" console line found")
print(json.dumps(lines[last_start + 1:]))
' "${name}"
}

wait_console_contains() {
  local name="$1" needle="$2" deadline
  deadline=$(( $(date +%s) + 30 ))
  while [[ "$(date +%s)" -lt "${deadline}" ]]; do
    if console_lines_since_start "${name}" | python3 -c '
import json, sys
needle = sys.argv[1]
lines = json.load(sys.stdin)
sys.exit(0 if any(needle in line["text"] for line in lines) else 1)
' "${needle}"; then
      return 0
    fi
    sleep 0.25
  done
  fail "console never contained (since ${name} started): ${needle}"
}

assert_launch_argv_shape() {
  # $1 = server display name that was just started. $2 = "args-file"
  # (Forge/NeoForge: `@<file> nogui`, no `-jar`) or "jar" (the other
  # four: `-jar <jar> --nogui`, no `@`).
  #
  # FakeServer.java/FakeInstaller.java print `sun.java.command`, not the
  # raw OS command line (see their own doc comments: `ProcessHandle`'s
  # Windows implementation never populates it -- JDK-8176725, found by
  # this step's own Windows CI leg). By the time `sun.java.command` is
  # built, `-jar`/`@<args-file>` are already gone -- it reads
  # "<jar> <args>" for a jar launch and "<MainClass> <args>" for an
  # args-file launch -- so the shape check below is on the first token's
  # `.jar` suffix instead of the literal flag.
  local name="$1" want="$2"
  local argv first_token
  argv="$(console_lines_since_start "${name}" | python3 -c '
import json, sys
lines = json.load(sys.stdin)
for line in lines:
    if line["text"].startswith("LAUNCH_ARGV:"):
        print(line["text"][len("LAUNCH_ARGV:"):])
        break
')"
  [[ -n "${argv}" ]] || fail "no LAUNCH_ARGV console line found since ${name} started"
  first_token="${argv%% *}"
  if [[ "${want}" == "args-file" ]]; then
    [[ "${first_token}" != *.jar ]] || fail "expected an args-file (class-name) launch, got a jar launch: ${argv}"
  else
    [[ "${first_token}" == *.jar ]] || fail "expected a jar launch, got: ${argv}"
  fi
  echo "launch argv (${want} shape): ${argv}"
}

config_field() {
  # Reads one field of the server_config_swift.json entry matching
  # $1 = serverId. $2 = field name (the durable on-disk config uses the
  # same snake_case field names as `msc_domain::app_config_schema
  # ::ConfigServer`, e.g. "minecraft_version"/"loader_version" --
  # verified directly against that module rather than assumed).
  local server_id="$1" field="$2"
  python3 -c '
import json, sys
server_id, field, path = sys.argv[1:4]
with open(path) as f:
    cfg = json.load(f)
for server in cfg["servers"]:
    if server["id"] == server_id:
        print(server.get(field) or "")
        sys.exit(0)
sys.exit("server not found: " + server_id)
' "${server_id}" "${field}" "${CONFIG_PATH}"
}

server_id_from_create_json() {
  python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["serverId"])'
}

# =====================================================================
# 4. Start the agent, pointed at the fake provider.
# =====================================================================
echo "== starting agent =="
start_agent

# =====================================================================
# 5. Create, start, and prove the launch shape of all six create-flow
#    families.
# =====================================================================
add_on_dir_for() {
  case "$1" in
    vanilla) echo "" ;;
    paper|purpur) echo "plugins" ;;
    fabric|neoforge|forge) echo "mods" ;;
  esac
}

launch_shape_for() {
  case "$1" in
    neoforge|forge) echo "args-file" ;;
    *) echo "jar" ;;
  esac
}

PORT_COUNTER=25566

for flavor in vanilla paper purpur fabric neoforge forge; do
  name="smoke-${flavor}"
  folder="smoke-${flavor}"
  port=$(( PORT_COUNTER++ ))
  echo "== creating ${flavor} (${name}) =="
  create_json="$(run_msc_json server create "${name}" --flavor "${flavor}" --port "${port}")"
  server_id="$(server_id_from_create_json <<<"${create_json}")"
  [[ -n "${server_id}" ]] || fail "${flavor}: create returned no serverId"
  server_dir="${SERVERS_ROOT}/java/${folder}"
  [[ -d "${server_dir}" ]] || fail "${flavor}: server directory missing after create: ${server_dir}"
  [[ -f "${server_dir}/eula.txt" ]] || fail "${flavor}: eula.txt missing"
  [[ -f "${server_dir}/server.properties" ]] || fail "${flavor}: server.properties missing"

  add_on="$(add_on_dir_for "${flavor}")"
  if [[ -n "${add_on}" ]]; then
    [[ -d "${server_dir}/${add_on}" ]] || fail "${flavor}: add-on folder ${add_on}/ missing"
  fi

  world_slots="$(find "${server_dir}/world_slots" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' ')"
  [[ "${world_slots}" -ge 1 ]] || fail "${flavor}: no initial world slot recorded"

  mc_version="$(config_field "${server_id}" minecraft_version)"
  [[ -n "${mc_version}" ]] || fail "${flavor}: no minecraft_version recorded"
  echo "${flavor}: recorded minecraft_version=${mc_version}"
  if [[ "${flavor}" == "fabric" || "${flavor}" == "neoforge" || "${flavor}" == "forge" ]]; then
    loader_version="$(config_field "${server_id}" loader_version)"
    [[ -n "${loader_version}" ]] || fail "${flavor}: no loader_version recorded"
    echo "${flavor}: recorded loader_version=${loader_version}"
  fi

  if [[ "${flavor}" == "forge" || "${flavor}" == "neoforge" ]]; then
    wait_for_path "${server_dir}/libraries/*/*/*/*/unix_args.txt" "${flavor} args file"
  fi

  echo "== starting ${flavor} =="
  run_msc server start "${name}" >/dev/null
  wait_running_state "True"
  wait_console_contains "${name}" 'Done (0.001s)! For help, type "help"'
  assert_launch_argv_shape "${name}" "$(launch_shape_for "${flavor}")"

  run_msc server stop "${name}" >/dev/null
  wait_running_state "False"
done

echo "all six families created, launched with the correct shape, and stopped"

# =====================================================================
# 6. A Phase-5-imported non-Paper server actually starts -- the port
#    plan's later-audit clause applied to an *imported* directory, not
#    only one this same process just created. Builds a synthetic raw
#    Forge-shaped directory (a real installed args file this run's own
#    fake installer already knows how to produce, dropped in directly
#    rather than run through --installServer again) and imports it.
# =====================================================================
echo "== importing a synthetic raw Forge-shaped server =="
IMPORT_SRC="${TMP_DIR}/raw-import-source"
IMPORT_LIB_DIR="${IMPORT_SRC}/libraries/net/minecraftforge/forge/1.20.1-47.4.5"
mkdir -p "${IMPORT_LIB_DIR}"
cp "${FAKE_INSTALLER_TEMPLATE}" "${IMPORT_LIB_DIR}/fake-loader.jar"
cat > "${IMPORT_LIB_DIR}/unix_args.txt" <<'ARGS'
-cp
libraries/net/minecraftforge/forge/1.20.1-47.4.5/fake-loader.jar
FakeInstaller
--launchTarget
forgeserver
ARGS
cat > "${IMPORT_SRC}/eula.txt" <<'EULA'
eula=true
EULA
cat > "${IMPORT_SRC}/server.properties" <<'PROPS'
server-port=25599
level-name=world
PROPS
mkdir -p "${IMPORT_SRC}/world"
python3 - "${IMPORT_SRC}/world/level.dat" <<'PY'
import gzip
import struct
import sys

dest = sys.argv[1]

def tag_compound_named(name: str) -> bytes:
    name_bytes = name.encode("utf-8")
    return b"\x0a" + struct.pack(">H", len(name_bytes)) + name_bytes

# Minimal but real gzip'd big-endian NBT: root compound "" containing an
# immediately-closed "Data" compound -- same shape
# tools/phase6/phase6-gate-smoke.sh's own level.dat writer uses, enough
# for the reader to gunzip and parse without error.
payload = tag_compound_named("") + tag_compound_named("Data") + b"\x00" + b"\x00"
with open(dest, "wb") as f:
    f.write(gzip.compress(payload))
PY

run_msc server import "${IMPORT_SRC}" --name smoke-imported-forge --type java >/dev/null
run_msc server start smoke-imported-forge >/dev/null
wait_running_state "True"
wait_console_contains "smoke-imported-forge" 'Done (0.001s)! For help, type "help"'
assert_launch_argv_shape "smoke-imported-forge" "args-file"
run_msc server stop smoke-imported-forge >/dev/null
wait_running_state "False"
echo "imported non-Paper (Forge) server launched from the args file its on-disk shape already carried"

# =====================================================================
# 7. Failure side: an injected download failure leaves no directory
#    behind.
# =====================================================================
echo "== injected download failure (vanilla) =="
DL_FAIL_DIR="${SERVERS_ROOT}/java/smoke-fail-download"
touch "${CONTROL_DIR}/fail_download"
expect_fail run_msc server create smoke-fail-download --flavor vanilla --port "$(( PORT_COUNTER++ ))"
rm -f "${CONTROL_DIR}/fail_download"
[[ ! -e "${DL_FAIL_DIR}" ]] || fail "server directory left behind after an injected download failure: ${DL_FAIL_DIR}"
echo "injected download failure left no directory behind"

# =====================================================================
# 8. Failure side: an injected installer failure leaves no directory
#    behind.
# =====================================================================
echo "== injected installer failure (forge) =="
INSTALL_FAIL_DIR="${SERVERS_ROOT}/java/smoke-fail-install"
touch "${CONTROL_DIR}/fail_install"
expect_fail run_msc server create smoke-fail-install --flavor forge --port "$(( PORT_COUNTER++ ))"
rm -f "${CONTROL_DIR}/fail_install"
[[ ! -e "${INSTALL_FAIL_DIR}" ]] || fail "server directory left behind after an injected installer failure: ${INSTALL_FAIL_DIR}"
echo "injected installer failure left no directory behind"

# =====================================================================
# 9. Kill the agent mid-install: the operation journal reconciles the
#    interrupted "server-create" operation on restart.
# =====================================================================
echo "== killing the agent mid-install (neoforge) =="
KILL_DIR="${SERVERS_ROOT}/java/smoke-kill-neoforge"
create_json="$(run_msc_json server create smoke-kill-neoforge --flavor neoforge --port "$(( PORT_COUNTER++ ))" --no-wait)"
operation_id="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["operationId"])' <<<"${create_json}")"
[[ -n "${operation_id}" ]] || fail "no operationId returned for --no-wait create"
wait_for_path "${KILL_DIR}/neoforge-installer.jar" "neoforge installer download (pre-kill signal)"
kill_agent_hard
restart_agent
assert_operation_interrupted_by_restart "${operation_id}"
echo "mid-install kill: the operation journal reconciled the interrupted create to failed on restart"

echo "PHASE 7 GATE SMOKE PASSED"
