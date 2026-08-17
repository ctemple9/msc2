#!/usr/bin/env bash
set -euo pipefail

# Phase 6 restart-sensitive public-path smoke (P6.25).
#
# Drives a real foreground msc-agent through nothing but the CLI/API
# (the same surface the iOS client and any other public caller uses):
# import a Java multi-folder world, reconcile it into the formal slot
# model (P6.1/P6.4/P6.11), exercise slot CRUD, activate with a
# mandatory safety backup, take manual backups (both the confirmed and
# the timed-out-but-still-best-effort save-coordination paths), inject
# a real archive-creation failure, restore (including its running-
# server and cross-slot guards), then SIGKILL the real agent process
# mid-activation and mid-restore and prove `reconcile_interrupted_activation`/
# `reconcile_interrupted_restore` (P6.13/P6.18) actually recover a real
# on-disk transaction, not just a fixture-shaped one.
#
# `--synthetic` is the only mode this step builds: a committed, no-
# real-data fake Java world so this runs identically on any machine
# (and, once P6.27 wires it in, any CI runner). Exercising the real
# MSC 1 corpus through this same public path is P6.26's own job, per
# this phase's plan text ("Run the real package/world/backup through
# the public Phase 6 smoke where size permits") -- not built here.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MODE=""
PRIVATE_CORPUS_ROOT=""

usage() {
  cat <<USAGE
Usage: $0 --synthetic
       $0 --custom-level-name
       $0 --private-corpus <path>

--synthetic runs the full restart-sensitive public-path gate smoke
against a committed synthetic Java world -- no real MSC 1 data, safe
to run anywhere.

--custom-level-name runs the focused public-path proof with a Java
server whose configured world is not named "world".

--private-corpus <path> runs a smaller, real-data public-path smoke
(bounded server import, world export/import, activation, backup,
restore) against whichever real Java world sorts first under <path>
-- P6.35's own "drive the real private corpus through the public
path, not just direct application-library calls" leg. <path> is
read-only: every real file under the world folder this picks is
hashed before and after, and the run fails loudly if anything
changed. Meant to be invoked by
tools/phase6/corpus-check.py --exercise --private-root <path>, though
it runs standalone the same way --synthetic does.
USAGE
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --synthetic)
      MODE="synthetic"
      shift
      ;;
    --custom-level-name)
      MODE="custom-level-name"
      shift
      ;;
    --private-corpus)
      MODE="private-corpus"
      PRIVATE_CORPUS_ROOT="${2:-}"
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

if [[ "${MODE}" != "synthetic" && "${MODE}" != "custom-level-name" && "${MODE}" != "private-corpus" ]]; then
  usage >&2
  exit 2
fi
if [[ "${MODE}" == "private-corpus" ]]; then
  if [[ -z "${PRIVATE_CORPUS_ROOT}" || ! -d "${PRIVATE_CORPUS_ROOT}" ]]; then
    echo "--private-corpus requires an existing directory" >&2
    exit 2
  fi
  PRIVATE_CORPUS_ROOT="$(cd "${PRIVATE_CORPUS_ROOT}" && pwd)"
fi

MSC_BIN="${ROOT}/target/debug/msc"
TOKEN="msc2_phase6_gate_bootstrap_secret"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/msc2-phase6-gate.XXXXXX")"
DATA_DIR="${TMP_DIR}/data"
SERVERS_ROOT="${TMP_DIR}/servers"
CONFIG_PATH="${DATA_DIR}/server_config_swift.json"
SERVER_DIR="${SERVERS_ROOT}/java/smoke-world"
FAKE_SOURCE_DIR="${TMP_DIR}/fake-source"
AGENT_PID=""
PORT=""
BASE_URL=""
KEYCHAIN_SERVICE="com.msc2.phase6.gate.$(date +%Y%m%d%H%M%S).$$"

cleanup() {
  if [[ -n "${AGENT_PID}" ]] && kill -0 "${AGENT_PID}" 2>/dev/null; then
    kill -9 "${AGENT_PID}" 2>/dev/null || true
    wait "${AGENT_PID}" 2>/dev/null || true
  fi
  if [[ "$(uname -s)" == "Darwin" ]]; then
    /usr/bin/security delete-generic-password \
      -s "${KEYCHAIN_SERVICE}" \
      -a "remote-api.token.phase6" >/dev/null 2>&1 || true
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
require_tool javac
require_tool jar

fail() {
  echo "FAIL: $*" >&2
  exit 1
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
  # Optional first arg "pause-after-world-move" starts this one agent
  # instance with MSC2_TEST_PAUSE_AFTER_WORLD_MOVE set, so the restart
  # races below get a stable, arbitrarily-wide window to catch instead
  # of racing a poll against a real handful-of-rename()-syscalls
  # window. Every other start_agent call (including the restart right
  # after each race kills this one) leaves it unset -- this must never
  # leak into a process serving any other step's operations.
  if [[ "${1:-}" == "pause-after-world-move" ]]; then
    export MSC2_TEST_PAUSE_AFTER_WORLD_MOVE=1
  else
    unset MSC2_TEST_PAUSE_AFTER_WORLD_MOVE
  fi
  export MSC2_TEST_BOOTSTRAP_TOKEN="${TOKEN}"
  export MSC2_DATA_DIR="${DATA_DIR}"
  export MSC2_APP_CONFIG_PATH="${CONFIG_PATH}"
  export MSC2_AGENT_SERVERS_ROOT="${SERVERS_ROOT}"
  export MSC2_CREDENTIAL_REGISTRY_PATH="${DATA_DIR}/credential-registry.json"
  export MSC2_OPERATION_JOURNAL_DIR="${DATA_DIR}/journal"
  export MSC2_AUDIT_LOG_DIR="${DATA_DIR}/audit-log"
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
  # Runs a CLI call that must fail (a refused guard); fails the smoke
  # if it unexpectedly succeeds.
  if run_msc "$@" >/dev/null 2>>"${TMP_DIR}/agent.log"; then
    fail "expected failure but succeeded: $*"
  fi
}

slot_id_by_name() {
  run_msc_json world list | python3 -c '
import json, sys
data = json.load(sys.stdin)
name = sys.argv[1]
matches = [s["id"] for s in data["slots"] if s["name"] == name]
if not matches:
    sys.exit("no slot named " + name)
print(matches[-1])
' "$1"
}

slot_count() {
  run_msc_json world list | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["slots"]))'
}

active_slot_id() {
  run_msc_json world list | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("activeSlotId") or "")'
}

backup_ids_snapshot() {
  run_msc_json backup list | python3 -c '
import json, sys
d = json.load(sys.stdin)
for b in d["backups"]:
    print(b["id"])
'
}

backup_trigger_reason() {
  run_msc_json backup list | python3 -c '
import json, sys
d = json.load(sys.stdin)
target = sys.argv[1]
for b in d["backups"]:
    if b["id"] == target:
        print(b["triggerReason"])
        sys.exit(0)
sys.exit("backup not found: " + target)
' "$1"
}

new_ids_since() {
  # $1 = file with the "before" snapshot. Fails loudly instead of
  # silently handing back a multi-line id if more than one backup
  # appeared -- every call site expects exactly one.
  local new_ids count
  new_ids="$(comm -13 <(sort "$1") <(backup_ids_snapshot | sort))"
  count="$(printf '%s\n' "${new_ids}" | grep -c .)"
  if [[ "${count}" != "1" ]]; then
    fail "expected exactly 1 new backup, got ${count}: ${new_ids}"
  fi
  printf '%s' "${new_ids}"
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

wait_console_contains() {
  local needle="$1" deadline
  deadline=$(( $(date +%s) + 30 ))
  while [[ "$(date +%s)" -lt "${deadline}" ]]; do
    if run_msc_json console tail --lines 100 | python3 -c '
import json, sys
needle = sys.argv[1]
lines = json.load(sys.stdin)
sys.exit(0 if any(needle in line["text"] for line in lines) else 1)
' "${needle}"; then
      return 0
    fi
    sleep 0.25
  done
  fail "console never contained: ${needle}"
}

wait_server_ready() {
  wait_running_state "True"
  wait_console_contains 'Done (0.001s)! For help, type "help"'
  # The ready line lands in the console buffer via the same background
  # process-event pump that resolves the in-flight "java-start"
  # operation (`finish_active_lifecycle_operation_success`,
  # `routes/lifecycle.rs`) -- give it a moment so the very next
  # operation-admitting mutation (a backup/world call) doesn't lose a
  # per-server-target admission race against an operation that's
  # already functionally done but hasn't been marked `succeeded` yet.
  sleep 1
}

read_generation() {
  cat "${SERVER_DIR}/world/GENERATION.txt"
}

write_generation() {
  printf '%s' "$1" > "${SERVER_DIR}/world/GENERATION.txt"
}

operation_json() {
  # Raw `GET /v1/operations/{id}` -- there is no dedicated `msc operations
  # get` CLI verb (only `finish_operation`'s internal poll uses the
  # route), so this hits it directly the same way the script already
  # does for `/v1/servers` in step 2 below.
  local operation_id="$1"
  python3 - "${BASE_URL}" "${TOKEN}" "${operation_id}" <<'PY'
import json
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

operation_cancel() {
  # Raw `POST /v1/operations/{id}/cancel` -- blocks (agent-side, up to
  # 30s) until the operation the worker itself observes and stops, per
  # `routes/operations.rs::cancel`'s own truthful-cancellation doc.
  local operation_id="$1"
  python3 - "${BASE_URL}" "${TOKEN}" "${operation_id}" <<'PY'
import json
import sys
import urllib.request

base_url, token, operation_id = sys.argv[1:4]
req = urllib.request.Request(
    f"{base_url}/v1/operations/{operation_id}/cancel",
    data=b"{}",
    headers={
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    },
    method="POST",
)
with urllib.request.urlopen(req, timeout=35) as resp:
    print(resp.read().decode())
PY
}

assert_operation_interrupted_by_restart() {
  # The durable operation record left behind by a real agent restart
  # mid-transaction: `LifecycleOperations::reconcile_on_startup`
  # (called unconditionally by every `OperationsState::new`, so every
  # `start_agent` call in this script already runs it) reconciles any
  # journaled entry still `running` after a crash to `failed`, with
  # `error.code == "operation_interrupted"` and a message naming the
  # real cause -- not merely inferred from folders/markers looking
  # right, the record itself says why. See
  # `msc-infrastructure::operation_journal::reconcile_on_startup`.
  local operation_id="$1" state code message
  local record
  record="$(operation_json "${operation_id}")"
  state="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])' <<<"${record}")"
  code="$(python3 -c 'import json,sys; d=json.load(sys.stdin); print((d.get("error") or {}).get("code") or "")' <<<"${record}")"
  message="$(python3 -c 'import json,sys; d=json.load(sys.stdin); print((d.get("error") or {}).get("message") or "")' <<<"${record}")"
  [[ "${state}" == "failed" ]] || fail "operation ${operation_id}: expected reconciled state failed, got ${state}"
  [[ "${code}" == "operation_interrupted" ]] || fail "operation ${operation_id}: expected error code operation_interrupted, got ${code}"
  [[ "${message}" == *"restart"* ]] || fail "operation ${operation_id}: reconciled record does not explain the restart (message: ${message})"
  echo "operation ${operation_id} record explains the restart: ${message}"
}

if [[ "${MODE}" == "custom-level-name" ]]; then
  CUSTOM_NAME="family-realm"
  JAVA_SOURCE_DIR="${TMP_DIR}/custom-java-source"
  BEDROCK_SOURCE_DIR="${TMP_DIR}/custom-bedrock-source"
  JAVA_SERVER_DIR="${SERVERS_ROOT}/java/custom-java"
  BEDROCK_SERVER_DIR="${SERVERS_ROOT}/bedrock/custom-bedrock"
  CHUNKER_BUILD_DIR="${TMP_DIR}/fake-chunker"

  echo "== building focused non-default-level-name inputs =="
  mkdir -p \
    "${JAVA_SOURCE_DIR}/${CUSTOM_NAME}" \
    "${JAVA_SOURCE_DIR}/${CUSTOM_NAME}_nether" \
    "${JAVA_SOURCE_DIR}/${CUSTOM_NAME}_the_end" \
    "${BEDROCK_SOURCE_DIR}/worlds/Bedrock level" \
    "${CHUNKER_BUILD_DIR}"
  printf 'java-overworld\n' > "${JAVA_SOURCE_DIR}/${CUSTOM_NAME}/level.dat"
  printf 'java-nether\n' > "${JAVA_SOURCE_DIR}/${CUSTOM_NAME}_nether/DIM.txt"
  printf 'java-end\n' > "${JAVA_SOURCE_DIR}/${CUSTOM_NAME}_the_end/DIM.txt"
  printf 'server-port=25565\nlevel-name=%s\n' "${CUSTOM_NAME}" > "${JAVA_SOURCE_DIR}/server.properties"
  printf 'eula=true\n' > "${JAVA_SOURCE_DIR}/eula.txt"
  printf 'fake jar\n' > "${JAVA_SOURCE_DIR}/paper.jar"
  printf 'bedrock-source\n' > "${BEDROCK_SOURCE_DIR}/worlds/Bedrock level/level.dat"
  printf 'server-port=19132\nlevel-name=Bedrock level\n' > "${BEDROCK_SOURCE_DIR}/server.properties"
  printf 'fake binary\n' > "${BEDROCK_SOURCE_DIR}/bedrock_server"

  cat > "${CHUNKER_BUILD_DIR}/FakeChunker.java" <<'EOF'
import java.nio.file.*;
public class FakeChunker {
  public static void main(String[] args) throws Exception {
    if (args.length == 2 && args[0].equals("-f") && args[1].equals("?")) {
      System.out.println("JAVA_1_21_4 BEDROCK_1_21_0");
      return;
    }
    Path output = null;
    for (int i = 0; i + 1 < args.length; i++) {
      if (args[i].equals("-o")) output = Path.of(args[i + 1]);
    }
    if (output == null) System.exit(2);
    Files.createDirectories(output);
    Files.writeString(output.resolve("level.dat"), "converted-world");
  }
}
EOF
  (
    cd "${CHUNKER_BUILD_DIR}"
    javac FakeChunker.java
    printf 'Main-Class: FakeChunker\n' > manifest.txt
    jar cfm fake-chunker.jar manifest.txt FakeChunker.class >/dev/null
  )
  export MSC2_CHUNKER_JAR_PATH="${CHUNKER_BUILD_DIR}/fake-chunker.jar"

  echo "== importing custom Java target and Bedrock conversion source =="
  (cd "${ROOT}" && cargo build -p msc-agent >/dev/null)
  start_agent
  run_msc server import "${JAVA_SOURCE_DIR}" --name "custom-java" --type java --eula >/dev/null
  restart_agent
  JAVA_SLOT_ID="$(active_slot_id)"
  run_msc server import "${BEDROCK_SOURCE_DIR}" --name "custom-bedrock" --type bedrock >/dev/null
  restart_agent

  SERVER_IDS="$(python3 - "${BASE_URL}" "${TOKEN}" <<'PY'
import json, sys, urllib.request
req = urllib.request.Request(sys.argv[1] + "/v1/servers", headers={"Authorization": "Bearer " + sys.argv[2]})
with urllib.request.urlopen(req, timeout=5) as resp:
    servers = json.load(resp)
for wanted in ("custom-java", "custom-bedrock"):
    matches = [s["id"] for s in servers if s["name"] == wanted]
    if len(matches) != 1: raise SystemExit(f"expected one {wanted}: {servers!r}")
    print(matches[0])
PY
)"
  JAVA_SERVER_ID="$(printf '%s\n' "${SERVER_IDS}" | sed -n '1p')"
  BEDROCK_SERVER_ID="$(printf '%s\n' "${SERVER_IDS}" | sed -n '2p')"
  echo "== proving manual backup and restore capture the configured Java folders =="
  backup_ids_snapshot > "${TMP_DIR}/custom-before-manual.txt"
  run_msc backup now >/dev/null
  CUSTOM_BACKUP_ID="$(new_ids_since "${TMP_DIR}/custom-before-manual.txt")"
  python3 - "${JAVA_SERVER_DIR}/backups/${CUSTOM_BACKUP_ID}" "${CUSTOM_NAME}" <<'PY'
import sys, zipfile
path, base = sys.argv[1:3]
with zipfile.ZipFile(path) as zf:
    names = zf.namelist()
for folder in (base, base + "_nether", base + "_the_end"):
    if not any(name.startswith(folder + "/") for name in names):
        raise SystemExit(f"{path}: missing {folder}: {names!r}")
PY
  run_msc backup restore "${CUSTOM_BACKUP_ID}" >/dev/null

  echo "== proving conversion keeps the Bedrock target's distinct layout =="
  [[ ! -e "${BEDROCK_SERVER_DIR}/backups" ]] || fail "Bedrock target unexpectedly had backups before conversion"
  CUSTOM_BACKUPS_BEFORE_CONVERT=0
  run_msc world convert "${JAVA_SLOT_ID}" \
    --target-server "${BEDROCK_SERVER_ID}" \
    --target-format BEDROCK_1_21_0 \
    --target-name "Converted" >/dev/null
  CUSTOM_BACKUPS_AFTER_CONVERT="$(find "${BEDROCK_SERVER_DIR}/backups" -name '*.zip' -type f | wc -l | tr -d ' ')"
  [[ "${CUSTOM_BACKUPS_AFTER_CONVERT}" -eq $((CUSTOM_BACKUPS_BEFORE_CONVERT + 1)) ]] || fail "conversion did not create its Bedrock target safety backup"
  [[ -d "${BEDROCK_SERVER_DIR}/worlds" ]] || fail "conversion did not preserve Bedrock's worlds/ layout"

  echo "== proving activation and replacement use the configured Java world =="
  backup_ids_snapshot > "${TMP_DIR}/custom-before-activate.txt"
  run_msc world activate "${JAVA_SLOT_ID}" >/dev/null
  new_ids_since "${TMP_DIR}/custom-before-activate.txt" >/dev/null

  mkdir -p "${TMP_DIR}/custom-replacement/replacement"
  printf 'replacement\n' > "${TMP_DIR}/custom-replacement/replacement/level.dat"
  backup_ids_snapshot > "${TMP_DIR}/custom-before-replace.txt"
  run_msc world replace-active replacement --source "${TMP_DIR}/custom-replacement/replacement" >/dev/null
  new_ids_since "${TMP_DIR}/custom-before-replace.txt" >/dev/null
  [[ ! -e "${JAVA_SERVER_DIR}/${CUSTOM_NAME}" ]] || fail "replacement left the old configured Java world in place"
  [[ ! -e "${JAVA_SERVER_DIR}/${CUSTOM_NAME}_nether" ]] || fail "replacement left the old configured Nether folder in place"
  [[ ! -e "${JAVA_SERVER_DIR}/${CUSTOM_NAME}_the_end" ]] || fail "replacement left the old configured End folder in place"
  [[ -f "${JAVA_SERVER_DIR}/replacement/level.dat" ]] || fail "replacement did not install the new world"

  echo "phase6 gate smoke (custom level name) passed"
  exit 0
fi

if [[ "${MODE}" == "private-corpus" ]]; then
  # =====================================================================
  # Private-corpus mode (P6.35): a smaller, real-data run of the same
  # public path, invoked separately from --synthetic's own giant
  # sequence below. See this script's own usage() text for what it
  # covers and why.
  # =====================================================================
  echo "== discovering the primary real Java world under ${PRIVATE_CORPUS_ROOT} =="
  PRIMARY_WORLD_DIR="$(python3 - "${PRIVATE_CORPUS_ROOT}" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
excluded = {"backups", "bedrock", "world_slots"}


def excluded_path(p: Path) -> bool:
    return any(
        part in excluded or part.startswith("bedrock") or part.startswith("_")
        for part in p.parts
    )


candidates = sorted(
    p.parent
    for p in root.rglob("level.dat")
    if not excluded_path(p.relative_to(root))
)
if not candidates:
    raise SystemExit(f"no real Java level.dat found under {root}")
print(candidates[0])
PY
)"
  echo "primary real world: ${PRIMARY_WORLD_DIR}"

  echo "== hashing real source files before the run =="
  HASHES_BEFORE="$(python3 - "${PRIMARY_WORLD_DIR}" <<'PY'
import hashlib
import sys
from pathlib import Path

root = Path(sys.argv[1])
for path in sorted(root.rglob("*")):
    if path.is_file():
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        print(f"{digest}  {path.relative_to(root)}")
PY
)"

  echo "== building msc-agent =="
  (cd "${ROOT}" && cargo build -p msc-agent >/dev/null)

  # `server import` expects a folder laid out like a real server root --
  # `server.properties` (whose `level-name` names the primary world
  # folder) as a sibling of the world folder(s) themselves, exactly the
  # shape --synthetic's own FAKE_SOURCE_DIR builds in step 1. The real
  # server root one level above the discovered world folder already has
  # exactly that shape -- but it also holds the real server's jars,
  # mods, and logs (hundreds of MB, none of it world/backup evidence),
  # so this stages only the two things import actually needs: the real
  # `server.properties` (read-only, defines which sibling is the
  # primary world) and a copy of the real world folder itself -- the
  # actual corpus material this step is proving the public path against.
  PRIMARY_SERVER_ROOT="$(dirname "${PRIMARY_WORLD_DIR}")"
  [[ -f "${PRIMARY_SERVER_ROOT}/server.properties" ]] || fail "${PRIMARY_SERVER_ROOT}: no server.properties next to the discovered real world folder"
  PRIVATE_STAGE_DIR="${TMP_DIR}/private-corpus-source"
  mkdir -p "${PRIVATE_STAGE_DIR}"

  cp "${PRIMARY_SERVER_ROOT}/server.properties" "${PRIVATE_STAGE_DIR}/server.properties"
  PRIMARY_WORLD_NAME="$(basename "${PRIMARY_WORLD_DIR}")"
  cp -R "${PRIMARY_WORLD_DIR}" "${PRIVATE_STAGE_DIR}/${PRIMARY_WORLD_NAME}"
  for suffix in _nether _the_end; do
    if [[ -d "${PRIMARY_SERVER_ROOT}/${PRIMARY_WORLD_NAME}${suffix}" ]]; then
      cp -R "${PRIMARY_SERVER_ROOT}/${PRIMARY_WORLD_NAME}${suffix}" "${PRIVATE_STAGE_DIR}/${PRIMARY_WORLD_NAME}${suffix}"
    fi
  done

  # `server import`'s own `--name` becomes the new server's directory
  # name under `${SERVERS_ROOT}/java/` -- distinct from the shared
  # `${SERVER_DIR}` (`.../java/smoke-world`) --synthetic's own sections
  # use, since this mode never imports under that name.
  PRIVATE_SERVER_DIR="${SERVERS_ROOT}/java/private-corpus"

  echo "== importing the real world folder (server import, then reconciling) =="
  start_agent
  run_msc server import "${PRIVATE_STAGE_DIR}" --name "private-corpus" --type java --eula >/dev/null
  restart_agent
  [[ "$(slot_count)" == "1" ]] || fail "expected exactly 1 slot after real-corpus reconciliation, got $(slot_count)"
  IMPORTED_SLOT_ID="$(active_slot_id)"
  [[ -f "${PRIVATE_SERVER_DIR}/world_slots/${IMPORTED_SLOT_ID}/world.zip" ]] || fail "reconciled real-corpus slot has no archived world.zip"
  echo "reconciled real-corpus slot: ${IMPORTED_SLOT_ID}"

  echo "== round-tripping through the bounded staged-upload export/import path =="
  run_msc world export "${IMPORTED_SLOT_ID}" --output "${TMP_DIR}/private-corpus-export.zip" >/dev/null
  [[ -s "${TMP_DIR}/private-corpus-export.zip" ]] || fail "real-corpus world export produced an empty file"
  run_msc world import "${TMP_DIR}/private-corpus-export.zip" "Private Corpus Import" >/dev/null
  [[ "$(slot_count)" == "2" ]] || fail "expected 2 slots after real-corpus staged-upload import, got $(slot_count)"
  UPLOADED_SLOT_ID="$(slot_id_by_name "Private Corpus Import")"

  echo "== activating the uploaded real-corpus slot with its mandatory safety backup =="
  backup_ids_snapshot > "${TMP_DIR}/private-corpus-backups-before-activate.txt"
  run_msc world activate "${UPLOADED_SLOT_ID}" >/dev/null
  [[ "$(active_slot_id)" == "${UPLOADED_SLOT_ID}" ]] || fail "real-corpus activation did not update the active slot"
  SAFETY_BACKUP_ID="$(new_ids_since "${TMP_DIR}/private-corpus-backups-before-activate.txt")"
  [[ -n "${SAFETY_BACKUP_ID}" ]] || fail "real-corpus activation did not take its mandatory safety backup"

  # No real server jar exists for this mode (unlike --synthetic's own
  # fake one) -- `backup now` and `restore` are both fully real and
  # legitimate against a stopped server (the direct-zip fallback path
  # every other stopped-server backup in this script already takes;
  # `create_backup`'s own `console: None` branch), so this doesn't
  # start the process at all.
  echo "== taking a manual backup and restoring it (real bytes) =="
  backup_ids_snapshot > "${TMP_DIR}/private-corpus-backups-before-manual.txt"
  run_msc backup now >/dev/null
  MANUAL_BACKUP_ID="$(new_ids_since "${TMP_DIR}/private-corpus-backups-before-manual.txt")"
  [[ -n "${MANUAL_BACKUP_ID}" ]] || fail "real-corpus manual backup did not appear"
  run_msc backup restore "${MANUAL_BACKUP_ID}" >/dev/null

  stop_agent

  echo "== hashing real source files after the run =="
  HASHES_AFTER="$(python3 - "${PRIMARY_WORLD_DIR}" <<'PY'
import hashlib
import sys
from pathlib import Path

root = Path(sys.argv[1])
for path in sorted(root.rglob("*")):
    if path.is_file():
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        print(f"{digest}  {path.relative_to(root)}")
PY
)"
  [[ "${HASHES_BEFORE}" == "${HASHES_AFTER}" ]] || fail "real corpus evidence under ${PRIMARY_WORLD_DIR} changed during the private-corpus smoke run"

  echo "private-corpus public-path smoke passed against real data from ${PRIMARY_WORLD_DIR}"
  exit 0
fi

# =====================================================================
# 1. Build the synthetic Java multi-folder world + fake server jar.
#
# The jar answers `save-all flush` with "Saved the game" exactly once
# per process lifetime, then goes silent -- the manual backup test
# below fires it twice on purpose, so the first exercises the normal
# confirmed-pause path and the second exercises `pauseSavesForBackup`'s
# timeout-as-best-effort path with a real ~10s wall-clock wait, not a
# fixture standing in for one.
# =====================================================================
echo "== building synthetic Java world =="
mkdir -p "${FAKE_SOURCE_DIR}"

cat > "${FAKE_SOURCE_DIR}/FakePaper.java" <<'EOF'
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.concurrent.atomic.AtomicBoolean;

public class FakePaper {
    public static void main(String[] args) throws Exception {
        System.out.println("Booting fake Paper");
        System.out.flush();
        Thread.sleep(500);
        System.out.println("Done (0.001s)! For help, type \"help\"");
        System.out.flush();
        // A real join line, parsed by the same `output_reducer.rs`
        // (`parse_java_player_name`/`upsert_online_player`) a live
        // server's own console output would trigger -- P6.35's scheduled-
        // backup smoke needs a genuinely *detected* online player, not a
        // fixture standing in for one, since `BackupScheduler::fire`'s
        // own online-player gate reads this same reducer state.
        System.out.println("smokePlayer joined the game");
        System.out.flush();

        AtomicBoolean respondedToFlushOnce = new AtomicBoolean(false);
        BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
        String line;
        while ((line = reader.readLine()) != null) {
            System.out.println("COMMAND:" + line);
            System.out.flush();
            if (line.equals("stop")) {
                System.out.println("Stopping fake Paper");
                System.out.flush();
                return;
            } else if (line.equals("save-all flush")) {
                if (!respondedToFlushOnce.getAndSet(true)) {
                    System.out.println("Saved the game");
                    System.out.flush();
                }
            } else if (line.equals("save-off")) {
                System.out.println("Turned off world auto-saving");
                System.out.flush();
            } else if (line.equals("save-on")) {
                System.out.println("Turned on world auto-saving");
                System.out.flush();
            }
        }
    }
}
EOF

(
  cd "${FAKE_SOURCE_DIR}"
  javac FakePaper.java
  printf 'Main-Class: FakePaper\n' > manifest.txt
  jar cfm paper.jar manifest.txt FakePaper.class >/dev/null
)

cat > "${FAKE_SOURCE_DIR}/server.properties" <<'EOF'
server-port=25565
max-players=20
level-name=world
EOF

cat > "${FAKE_SOURCE_DIR}/eula.txt" <<'EOF'
eula=true
EOF

python3 - "${FAKE_SOURCE_DIR}/world/level.dat" <<'PY'
import gzip
import os
import sys
import struct

dest = sys.argv[1]
os.makedirs(os.path.dirname(dest), exist_ok=True)

def tag_compound_named(name: str) -> bytes:
    name_bytes = name.encode("utf-8")
    return b"\x0a" + struct.pack(">H", len(name_bytes)) + name_bytes

# Minimal but real gzip'd big-endian NBT: root compound "" containing
# an immediately-closed "Data" compound. Enough for the reader
# (`nbt.rs`, P6.9) to gunzip and parse without error; this fixture
# doesn't assert on any extracted field, so no further tags are needed
# -- the real level.dat corpus (P6.7/P6.26) already covers extraction.
payload = tag_compound_named("") + tag_compound_named("Data") + b"\x00" + b"\x00"
with open(dest, "wb") as f:
    f.write(gzip.compress(payload))
PY

mkdir -p "${FAKE_SOURCE_DIR}/world_nether" "${FAKE_SOURCE_DIR}/world_the_end"
printf 'dimension placeholder\n' > "${FAKE_SOURCE_DIR}/world_nether/DIM.txt"
printf 'dimension placeholder\n' > "${FAKE_SOURCE_DIR}/world_the_end/DIM.txt"
printf 'GEN-IMPORTED' > "${FAKE_SOURCE_DIR}/world/GENERATION.txt"

echo "== building msc-agent =="
(
  cd "${ROOT}"
  cargo build -p msc-agent >/dev/null
)

# =====================================================================
# 2. Import the raw folder, then restart to trigger reconciliation.
#
# `import_raw_server` (P5.20) copies the folder in and makes it active
# immediately, but P6.1's reconciliation only runs at agent startup
# (`reconcile_imported_worlds_at_startup`) -- so the live-folders-only
# state this import produces isn't turned into a formal slot until the
# next restart, exactly like a real upgrade from a pre-Phase-6 agent.
# =====================================================================
echo "== importing raw Java world and reconciling =="
start_agent
run_msc server import "${FAKE_SOURCE_DIR}" --name "smoke-world" --type java --eula >/dev/null

python3 - "${BASE_URL}" "${TOKEN}" <<'PY'
import json
import sys
import urllib.request

base_url, token = sys.argv[1:3]
req = urllib.request.Request(base_url + "/v1/servers", headers={"Authorization": f"Bearer {token}"})
with urllib.request.urlopen(req, timeout=5) as resp:
    servers = json.load(resp)
if len(servers) != 1:
    raise SystemExit(f"expected exactly 1 imported server, got {servers!r}")
PY

restart_agent

[[ "$(slot_count)" == "1" ]] || fail "expected exactly 1 slot after reconciliation, got $(slot_count)"
[[ -f "${SERVER_DIR}/world_slots/$(active_slot_id)/world.zip" ]] || fail "reconciled slot has no archived world.zip"
[[ "$(read_generation)" == "GEN-IMPORTED" ]] || fail "unexpected live generation after import: $(read_generation)"

SLOT_IMPORTED_ID="$(active_slot_id)"
echo "reconciled imported slot: ${SLOT_IMPORTED_ID}"

# =====================================================================
# 3. Slot CRUD: create, rename, duplicate, copy, export, import, delete.
# =====================================================================
echo "== exercising slot CRUD =="
write_generation "GEN-2"
run_msc world create "Slot 2" --seed "smoke-seed" >/dev/null
SLOT2_ID="$(slot_id_by_name "Slot 2")"
[[ "$(slot_count)" == "2" ]] || fail "expected 2 slots after create, got $(slot_count)"

run_msc world rename "${SLOT2_ID}" "Slot 2 Renamed" >/dev/null
[[ "$(slot_id_by_name "Slot 2 Renamed")" == "${SLOT2_ID}" ]] || fail "rename did not stick"

run_msc world duplicate "${SLOT2_ID}" >/dev/null
[[ "$(slot_count)" == "3" ]] || fail "expected 3 slots after duplicate, got $(slot_count)"
DUP_ID="$(run_msc_json world list | python3 -c '
import json, sys
d = json.load(sys.stdin)
known = {sys.argv[1], sys.argv[2]}
extra = [s["id"] for s in d["slots"] if s["id"] not in known]
print(extra[0])
' "${SLOT_IMPORTED_ID}" "${SLOT2_ID}")"

run_msc world copy --into "${DUP_ID}" --from "${SLOT_IMPORTED_ID}" >/dev/null
[[ "$(slot_count)" == "3" ]] || fail "copy changed slot count unexpectedly"

run_msc world export "${SLOT2_ID}" --output "${TMP_DIR}/exported.zip" >/dev/null
[[ -s "${TMP_DIR}/exported.zip" ]] || fail "world export produced an empty file"

run_msc world import "${TMP_DIR}/exported.zip" "Imported Copy" >/dev/null
[[ "$(slot_count)" == "4" ]] || fail "expected 4 slots after import, got $(slot_count)"

run_msc world delete "$(slot_id_by_name "Imported Copy")" >/dev/null
run_msc world delete "${DUP_ID}" >/dev/null
[[ "$(slot_count)" == "2" ]] || fail "expected 2 slots after cleanup, got $(slot_count)"

# =====================================================================
# 4. Running-server guard on activation.
# =====================================================================
echo "== checking running-server guard on activation =="
run_msc server start >/dev/null
wait_server_ready
expect_fail world activate "${SLOT2_ID}"
run_msc server stop >/dev/null
wait_running_state "False"

# =====================================================================
# 5. Injected archive-creation failure: occupy the backups directory
# with a plain file before it exists, forcing `create_dir_all` (and
# therefore the whole backup) to fail cleanly and portably -- no
# platform-specific permission bits needed.
# =====================================================================
echo "== injecting an archive-creation failure =="
[[ -e "${SERVER_DIR}/backups" ]] && fail "backups path already exists before injection"
touch "${SERVER_DIR}/backups"
expect_fail backup now
[[ -f "${SERVER_DIR}/backups" && ! -d "${SERVER_DIR}/backups" ]] || fail "archive-creation-failure injection left an unexpected backups path"
rm -f "${SERVER_DIR}/backups"

# =====================================================================
# 6. Manual backups: confirmed save-pause, then the timeout-as-best-
# effort path (the fake jar only answers `save-all flush` once).
# =====================================================================
echo "== taking manual backups (confirmed, then timed-out-but-best-effort) =="
run_msc server start >/dev/null
wait_server_ready

backup_ids_snapshot > "${TMP_DIR}/backups-before-1.txt"
run_msc backup now >/dev/null
BACKUP_1_ID="$(new_ids_since "${TMP_DIR}/backups-before-1.txt")"
[[ -n "${BACKUP_1_ID}" ]] || fail "manual backup #1 did not appear"
[[ "$(backup_trigger_reason "${BACKUP_1_ID}")" == "manual" ]] || fail "manual backup #1 has unexpected trigger reason"

echo "   (backup #2 exercises the ~10s save-confirmation timeout path -- this is expected to take a while)"
backup_ids_snapshot > "${TMP_DIR}/backups-before-2.txt"
run_msc backup now >/dev/null
BACKUP_2_ID="$(new_ids_since "${TMP_DIR}/backups-before-2.txt")"
[[ -n "${BACKUP_2_ID}" ]] || fail "manual backup #2 (timeout path) did not appear"

run_msc server stop >/dev/null
wait_running_state "False"

# =====================================================================
# 7. Plain activation with its mandatory safety backup.
# =====================================================================
echo "== activating a slot with its mandatory safety backup =="
write_generation "GEN-3"
backup_ids_snapshot > "${TMP_DIR}/backups-before-activate.txt"
run_msc world activate "${SLOT_IMPORTED_ID}" >/dev/null
[[ "$(read_generation)" == "GEN-IMPORTED" ]] || fail "activation did not install the target slot's content"
[[ "$(active_slot_id)" == "${SLOT_IMPORTED_ID}" ]] || fail "active slot id did not update after activation"
SAFETY_1_ID="$(new_ids_since "${TMP_DIR}/backups-before-activate.txt")"
[[ -n "${SAFETY_1_ID}" ]] || fail "activation did not take its mandatory safety backup"
[[ "$(backup_trigger_reason "${SAFETY_1_ID}")" == "pre-mutation" ]] || fail "activation safety backup has unexpected trigger reason"

# =====================================================================
# 8. Restore guards: running-server, cross-slot, and missing-source.
# =====================================================================
echo "== checking restore guards =="
run_msc server start >/dev/null
wait_server_ready
expect_fail backup restore "${BACKUP_1_ID}"
run_msc server stop >/dev/null
wait_running_state "False"

# BACKUP_1/BACKUP_2 are associated with SLOT_IMPORTED (active when they
# were taken). Activate SLOT2 so they become cross-slot, prove the
# guard refuses, then switch back to SLOT_IMPORTED so the real restore
# below is legitimate again.
run_msc world activate "${SLOT2_ID}" >/dev/null
expect_fail backup restore "${BACKUP_1_ID}"
run_msc world activate "${SLOT_IMPORTED_ID}" >/dev/null
[[ "$(active_slot_id)" == "${SLOT_IMPORTED_ID}" ]] || fail "failed to reactivate SLOT_IMPORTED before restore"

expect_fail backup restore "does-not-exist"

# =====================================================================
# 9. A real restore, with its own mandatory safety backup.
# =====================================================================
echo "== restoring a backup =="
write_generation "GEN-4"
backup_ids_snapshot > "${TMP_DIR}/backups-before-restore.txt"
run_msc backup restore "${BACKUP_1_ID}" >/dev/null
[[ "$(read_generation)" == "GEN-2" ]] || fail "restore did not install the backup's captured content"
SAFETY_2_ID="$(new_ids_since "${TMP_DIR}/backups-before-restore.txt")"
[[ -n "${SAFETY_2_ID}" ]] || fail "restore did not take its mandatory safety backup"
[[ "$(backup_trigger_reason "${SAFETY_2_ID}")" == "pre-restore" ]] || fail "restore safety backup has unexpected trigger reason"

# =====================================================================
# 10. Restart mid-activation: SIGKILL the real agent while
# `world_slots/.activation/prior/` exists (the live server directory
# has no complete world at all), then prove a fresh startup's
# `reconcile_interrupted_activation` recovers to a complete world
# either way, with no operator intervention.
#
# SLOT_IMPORTED and SLOT2 have fixed, distinguishable archived
# content (GEN-IMPORTED / GEN-2) that never changes once archived. Once
# the race is past its first attempt, every later attempt's target is
# both the previous attempt's target *and* the slot that was already
# active going in (a fully-completed activation updates both), so
# "prior_moved" always recovers to the *other* fixed slot/generation
# from whichever one is currently winning. The very first attempt is
# the one exception worth naming: the restore two steps up deliberately
# left live content (GEN-2, from the restored backup) drifted from the
# active-slot marker (still SLOT_IMPORTED, since restore never touches
# it) -- so for a first-attempt catch specifically, "prior_moved"'s
# recovered generation/active-slot are whatever was actually live/
# active immediately before the race started, not simply "the other
# slot." `PRE_RACE_LIVE_GEN`/`PRE_RACE_ACTIVE_SLOT` capture that fact
# directly rather than assuming it.
# =====================================================================
echo "== restart-mid-activation race =="
PRE_RACE_LIVE_GEN="$(read_generation)"
PRE_RACE_ACTIVE_SLOT="$(active_slot_id)"
[[ "${PRE_RACE_ACTIVE_SLOT}" == "${SLOT_IMPORTED_ID}" ]] || fail "unexpected active slot before the activation race"

# Restart the agent dedicated to this one call, paused durably between
# "old world moved aside" and "new world installed" -- see
# start_agent's own comment. This process's only job from here is to
# serve this call and then be killed by the race script.
stop_agent
start_agent pause-after-world-move

RACE_RESULT="$(python3 "${ROOT}/tools/phase6/fixtures/gate-smoke/race_transaction.py" \
  --msc "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" \
  --pid "${AGENT_PID}" \
  --marker-dir "${SERVER_DIR}/world_slots/.activation" \
  --cmd-a "world,activate,${SLOT_IMPORTED_ID}" \
  --cmd-b "world,activate,${SLOT2_ID}" \
  --start-with a \
  --max-attempts 300 --max-seconds 45)"
echo "activation race result: ${RACE_RESULT}"

CAUGHT="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["caught"])' <<<"${RACE_RESULT}")"
[[ "${CAUGHT}" == "True" ]] || fail "never caught the agent mid-activation within the attempt/time budget"

WINNING_TARGET="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["winning_target"])' <<<"${RACE_RESULT}")"
PHASE="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["phase"])' <<<"${RACE_RESULT}")"
ATTEMPTS="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["attempts"])' <<<"${RACE_RESULT}")"

if [[ "${WINNING_TARGET}" == "a" ]]; then
  TARGET_GEN="GEN-IMPORTED"; OTHER_GEN="GEN-2"
  TARGET_SLOT="${SLOT_IMPORTED_ID}"; OTHER_SLOT="${SLOT2_ID}"
else
  TARGET_GEN="GEN-2"; OTHER_GEN="GEN-IMPORTED"
  TARGET_SLOT="${SLOT2_ID}"; OTHER_SLOT="${SLOT_IMPORTED_ID}"
fi

if [[ "${PHASE}" == "installed" ]]; then
  EXPECTED_GEN="${TARGET_GEN}"; EXPECTED_ACTIVE="${TARGET_SLOT}"
elif [[ "${ATTEMPTS}" == "1" ]]; then
  EXPECTED_GEN="${PRE_RACE_LIVE_GEN}"; EXPECTED_ACTIVE="${PRE_RACE_ACTIVE_SLOT}"
else
  EXPECTED_GEN="${OTHER_GEN}"; EXPECTED_ACTIVE="${OTHER_SLOT}"
fi

AGENT_PID=""  # already SIGKILLed by the race script
start_agent

[[ ! -d "${SERVER_DIR}/world_slots/.activation" ]] || fail "interrupted activation was not reconciled away"
[[ "$(read_generation)" == "${EXPECTED_GEN}" ]] || fail "recovered generation mismatch: expected ${EXPECTED_GEN}, got $(read_generation)"
[[ "$(active_slot_id)" == "${EXPECTED_ACTIVE}" ]] || fail "recovered active slot id mismatch: expected ${EXPECTED_ACTIVE}, got $(active_slot_id)"
[[ "$(slot_count)" == "2" ]] || fail "slot count changed across the interrupted activation"

echo "activation recovery verified (${PHASE}, target=${WINNING_TARGET})"

# Folders/markers/slots aren't the whole story -- the durable operation
# record the killed CLI call was driving must itself explain what
# happened, not just the on-disk state reconcile_interrupted_activation
# fixed up.
ACTIVATION_OPERATION_ID="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["operation_id"])' <<<"${RACE_RESULT}")"
[[ -n "${ACTIVATION_OPERATION_ID}" && "${ACTIVATION_OPERATION_ID}" != "None" ]] || fail "activation race did not capture a real operation id from the killed CLI call"
assert_operation_interrupted_by_restart "${ACTIVATION_OPERATION_ID}"

# =====================================================================
# 11. Restart mid-restore: same technique, against
# `world_slots/.restore/prior/`.
# =====================================================================
echo "== restart-mid-restore race =="
run_msc world activate "${SLOT_IMPORTED_ID}" >/dev/null
CURRENT_ACTIVE="$(active_slot_id)"

write_generation "GEN-RESTORE-A"
backup_ids_snapshot > "${TMP_DIR}/backups-before-race-a.txt"
run_msc backup now >/dev/null
RACE_BACKUP_A="$(new_ids_since "${TMP_DIR}/backups-before-race-a.txt")"
[[ -n "${RACE_BACKUP_A}" ]] || fail "restore-race backup A did not appear"

write_generation "GEN-RESTORE-B"
backup_ids_snapshot > "${TMP_DIR}/backups-before-race-b.txt"
run_msc backup now >/dev/null
RACE_BACKUP_B="$(new_ids_since "${TMP_DIR}/backups-before-race-b.txt")"
[[ -n "${RACE_BACKUP_B}" ]] || fail "restore-race backup B did not appear"
[[ "${RACE_BACKUP_A}" != "${RACE_BACKUP_B}" ]] || fail "restore-race backups collided"
PRE_RACE_LIVE_GEN="$(read_generation)"
[[ "${PRE_RACE_LIVE_GEN}" == "GEN-RESTORE-B" ]] || fail "unexpected live generation before the restore race"

# Same dedicated-paused-process technique as the activation race above.
stop_agent
start_agent pause-after-world-move

RACE_RESULT="$(python3 "${ROOT}/tools/phase6/fixtures/gate-smoke/race_transaction.py" \
  --msc "${MSC_BIN}" --base-url "${BASE_URL}" --token "${TOKEN}" \
  --pid "${AGENT_PID}" \
  --marker-dir "${SERVER_DIR}/world_slots/.restore" \
  --cmd-a "backup,restore,${RACE_BACKUP_A}" \
  --cmd-b "backup,restore,${RACE_BACKUP_B}" \
  --start-with a \
  --max-attempts 300 --max-seconds 45)"
echo "restore race result: ${RACE_RESULT}"

CAUGHT="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["caught"])' <<<"${RACE_RESULT}")"
[[ "${CAUGHT}" == "True" ]] || fail "never caught the agent mid-restore within the attempt/time budget"

WINNING_TARGET="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["winning_target"])' <<<"${RACE_RESULT}")"
PHASE="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["phase"])' <<<"${RACE_RESULT}")"
ATTEMPTS="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["attempts"])' <<<"${RACE_RESULT}")"

if [[ "${WINNING_TARGET}" == "a" ]]; then
  TARGET_GEN="GEN-RESTORE-A"; OTHER_GEN="GEN-RESTORE-B"
else
  TARGET_GEN="GEN-RESTORE-B"; OTHER_GEN="GEN-RESTORE-A"
fi

# Restore never touches the active-slot marker, only live content, so
# there is no drift to account for here the way the activation race
# had -- but attempt 1's "before" is still the directly-observed
# `PRE_RACE_LIVE_GEN` on principle, not an assumption.
if [[ "${PHASE}" == "installed" ]]; then
  EXPECTED_GEN="${TARGET_GEN}"
elif [[ "${ATTEMPTS}" == "1" ]]; then
  EXPECTED_GEN="${PRE_RACE_LIVE_GEN}"
else
  EXPECTED_GEN="${OTHER_GEN}"
fi

AGENT_PID=""  # already SIGKILLed by the race script
start_agent

[[ ! -d "${SERVER_DIR}/world_slots/.restore" ]] || fail "interrupted restore was not reconciled away"
[[ "$(read_generation)" == "${EXPECTED_GEN}" ]] || fail "recovered restore generation mismatch: expected ${EXPECTED_GEN}, got $(read_generation)"
[[ "$(active_slot_id)" == "${CURRENT_ACTIVE}" ]] || fail "active slot id changed across an interrupted restore (it never should)"

echo "restore recovery verified (${PHASE}, target=${WINNING_TARGET})"

RESTORE_OPERATION_ID="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["operation_id"])' <<<"${RACE_RESULT}")"
[[ -n "${RESTORE_OPERATION_ID}" && "${RESTORE_OPERATION_ID}" != "None" ]] || fail "restore race did not capture a real operation id from the killed CLI call"
assert_operation_interrupted_by_restart "${RESTORE_OPERATION_ID}"

# =====================================================================
# 13. Active-world replacement's own operation record.
#
# P6.33/P6.34's `world replace-active` route shares the exact
# `succeed`/`cancel`/`fail` shape activation/restore already use --
# proven here with a plain (non-crash) run rather than a third restart
# race. `reconcile_interrupted_world_replace` (P6.33) exists at the
# application layer but is not yet reachable from agent startup
# (`routes/lifecycle.rs::reconcile_servers_at_startup` has no call to
# it -- flagged by P6.34's own report, confirmed still true here, and
# `routes/lifecycle.rs` is not in this step's own Files list to fix).
# A restart-mid-replace race built against that gap would prove
# nothing real: the on-disk `world_slots/.replace/` marker would
# simply be left behind forever, unreconciled, on the very next
# `start_agent` below. So this section covers what actually exists --
# replacement's own durable operation record explaining a real
# completed outcome -- rather than faking a recovery path that isn't
# wired up yet; see this step's own rolling-plan write-up for the
# open question this leaves for Cameron.
# =====================================================================
echo "== active-world replacement and its operation record =="
mkdir -p "${TMP_DIR}/replace-source/world"
cp "${SERVER_DIR}/world/level.dat" "${TMP_DIR}/replace-source/world/level.dat"
printf 'GEN-REPLACE' > "${TMP_DIR}/replace-source/world/GENERATION.txt"

PRE_REPLACE_ACTIVE_SLOT="$(active_slot_id)"
REPLACE_RESULT="$(run_msc_json world replace-active world --source "${TMP_DIR}/replace-source/world")"
[[ "$(python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])' <<<"${REPLACE_RESULT}")" == "succeeded" ]] || fail "replace-active operation did not succeed: ${REPLACE_RESULT}"
REPLACE_OPERATION_ID="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])' <<<"${REPLACE_RESULT}")"
REPLACE_STATUS_LINE="$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("statusLine") or "")' <<<"${REPLACE_RESULT}")"
[[ "${REPLACE_STATUS_LINE}" == *"complete"* ]] || fail "replace-active operation record does not explain its outcome (statusLine: ${REPLACE_STATUS_LINE})"
[[ "$(read_generation)" == "GEN-REPLACE" ]] || fail "replace-active did not install the uploaded replacement"
[[ "$(active_slot_id)" == "${PRE_REPLACE_ACTIVE_SLOT}" ]] || fail "replace-active changed the active slot marker (it never should -- it replaces live content directly)"

# The durable record, fetched back independently via GET rather than
# just trusting the CLI's own blocking-wait response, agrees.
FETCHED_REPLACE_RECORD="$(operation_json "${REPLACE_OPERATION_ID}")"
[[ "$(python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])' <<<"${FETCHED_REPLACE_RECORD}")" == "succeeded" ]] || fail "fetched replace-active operation record disagrees with the CLI's own response"

echo "replace-active operation ${REPLACE_OPERATION_ID} record explains its outcome: ${REPLACE_STATUS_LINE}"

# =====================================================================
# 14. Scheduled backup: fires only once a real online player is
# detected, uses the same save pause/resume protocol a manual backup
# does, cannot overlap another mutation, and prunes only down to --
# never below -- one known-good recovery point.
#
# The fake jar (step 1) prints a real "<player> joined the game" line
# on every boot, parsed by the same `output_reducer.rs` a live
# server's own console would trigger -- `BackupScheduler::fire`'s
# online-player gate is genuinely satisfied here, not assumed. The
# scheduler's own minimum interval is one real minute
# (`run_server_loop`'s `interval_minutes.max(1)`) -- no test-only
# fast-forward hook exists for it (this step's own Files list has no
# production `crates/msc-agent/src/*.rs` entry to add one to), so this
# section genuinely waits roughly a minute of wall-clock time for the
# first tick.
# =====================================================================
echo "== scheduled backup =="
run_msc server stop >/dev/null 2>&1 || true
wait_running_state "False"

# Reduce the many backups earlier sections already accumulated down to
# a known single baseline first -- `backup delete` refuses to delete
# the last remaining verified backup, so repeatedly deleting whatever
# is currently listed always converges to exactly one, regardless of
# exactly how many earlier sections left behind or what their trigger
# reasons were. Starting from a known baseline is what makes the
# post-tick count below an exact, non-fragile assertion rather than a
# guess about this script's own accumulated history.
while [[ "$(backup_ids_snapshot | grep -c .)" -gt 1 ]]; do
  run_msc backup delete "$(backup_ids_snapshot | head -n1)" >/dev/null
done
[[ "$(backup_ids_snapshot | grep -c .)" == "1" ]] || fail "failed to reduce backups to a single known baseline before scheduling"

# One more, real, on-disk backup beyond that baseline so the scheduled
# tick's own prune-before-create (`create_backup`'s own ordering: prune
# runs on `is_automatic`, before the new archive is written) has a
# genuine pair of already-valid backups to prune down to one, then adds
# its own new one on top -- the only way to actually observe "prunes
# down to, never below, one known-good recovery point" rather than
# merely "creates one backup".
run_msc server start >/dev/null
wait_server_ready
run_msc backup now >/dev/null
run_msc server stop >/dev/null
wait_running_state "False"
[[ "$(backup_ids_snapshot | grep -c .)" == "2" ]] || fail "expected exactly 2 backups on disk before the scheduled tick prunes them"

run_msc backup config set --enabled true --interval-minutes 1 --max-count 1 >/dev/null

run_msc server start >/dev/null
wait_server_ready
wait_console_contains "smokePlayer joined the game"

echo "   (waiting up to ~90s for the scheduler's first real tick -- this is expected to take a while)"
backup_ids_snapshot > "${TMP_DIR}/backups-before-scheduled-tick.txt"
SCHEDULED_DEADLINE=$(( $(date +%s) + 90 ))
SCHEDULED_BACKUP_ID=""
while [[ "$(date +%s)" -lt "${SCHEDULED_DEADLINE}" ]]; do
  SCHEDULED_NEW_IDS="$(comm -13 <(sort "${TMP_DIR}/backups-before-scheduled-tick.txt") <(backup_ids_snapshot | sort))"
  if [[ -n "${SCHEDULED_NEW_IDS}" ]]; then
    SCHEDULED_BACKUP_ID="${SCHEDULED_NEW_IDS}"
    break
  fi
  sleep 1
done
[[ -n "${SCHEDULED_BACKUP_ID}" ]] || fail "scheduled backup never fired within the wait budget"
[[ "$(backup_trigger_reason "${SCHEDULED_BACKUP_ID}")" == "auto" ]] || fail "scheduled backup has unexpected trigger reason"
echo "scheduled backup fired for real: ${SCHEDULED_BACKUP_ID}"

run_msc server stop >/dev/null
wait_running_state "False"
run_msc backup config set --enabled false >/dev/null

wait_console_contains "COMMAND:save-off"
wait_console_contains "COMMAND:save-all flush"
wait_console_contains "COMMAND:save-on"
echo "scheduled backup used the real save pause/resume protocol"

# `create_backup`'s own ordering prunes *before* writing the new
# archive (see this section's own comment above), so max-count 1
# pruning the pre-tick pair of 2 down to its single newest, then the
# tick's own new backup landing on top, leaves exactly 2 -- not 1. The
# real, load-bearing assertion isn't that literal number, it's that
# pruning demonstrably ran (2 pre-tick backups did not both survive
# alongside the new one, which would have left 3) while never dropping
# below one known-good recovery point at any point in the process.
FINAL_BACKUP_COUNT="$(backup_ids_snapshot | grep -c .)"
[[ "${FINAL_BACKUP_COUNT}" == "2" ]] || fail "expected exactly 2 backups after pruning (1 pre-tick survivor + the new scheduled one), got ${FINAL_BACKUP_COUNT}"
backup_ids_snapshot | grep -qx "${SCHEDULED_BACKUP_ID}" || fail "the scheduled backup itself did not survive its own tick's pruning"

BACKUP_ZIPS_AFTER_PRUNE="$(python3 -c "import glob; print('\n'.join(sorted(glob.glob('${SERVER_DIR}/backups/*.zip'))))")"
[[ -n "${BACKUP_ZIPS_AFTER_PRUNE}" ]] || fail "expected surviving backup zips on disk after pruning"
while IFS= read -r zip_path; do
  python3 - "${zip_path}" <<'PY'
import sys
import zipfile

zf = zipfile.ZipFile(sys.argv[1])
bad = zf.testzip()
if bad is not None:
    raise SystemExit(f"corrupt member in a surviving backup: {bad}")
if not zf.namelist():
    raise SystemExit("a surviving backup has no entries")
PY
done <<<"${BACKUP_ZIPS_AFTER_PRUNE}"
echo "every surviving backup after pruning is a valid, restorable archive -- pruning never dropped below one known-good recovery point"

# =====================================================================
# 15. Cancel an in-flight mutation: its target stays refused to a
# second mutation until rollback/cleanup finishes, and the live world
# is left completely untouched once it does.
#
# `activate_slot`'s own `should_cancel` is polled at exactly two
# boundaries where the live world hasn't been touched yet -- before
# its mandatory safety backup begins, and again once staging is
# complete but before the live folders are moved
# (`crates/msc-application/src/worlds.rs`'s own doc). The safety
# backup runs unconditionally between those two checks and is not
# itself cancellable mid-flight -- so a real, deterministic window
# (not a timing race) needs that backup to take long enough in real
# wall-clock time for an HTTP cancel request to land before the second
# check. A real ~100MB write into the *currently live* world (what the
# safety backup actually zips) does that honestly -- genuinely slower
# than one loopback HTTP round trip, not a fixture standing in for a
# real delay.
# =====================================================================
echo "== cancel an in-flight mutation =="
run_msc server stop >/dev/null 2>&1 || true
wait_running_state "False"

PRE_CANCEL_ACTIVE_SLOT="$(active_slot_id)"
PRE_CANCEL_GENERATION="$(read_generation)"
if [[ "${PRE_CANCEL_ACTIVE_SLOT}" == "${SLOT_IMPORTED_ID}" ]]; then
  CANCEL_TARGET_SLOT_ID="${SLOT2_ID}"
else
  CANCEL_TARGET_SLOT_ID="${SLOT_IMPORTED_ID}"
fi

python3 -c "
import os
with open('${SERVER_DIR}/world/CANCEL_FILLER.bin', 'wb') as f:
    f.write(os.urandom(100_000_000))
"

CANCEL_START_RESULT="$(run_msc_json world activate "${CANCEL_TARGET_SLOT_ID}" --no-wait)"
CANCEL_OPERATION_ID="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["operationId"])' <<<"${CANCEL_START_RESULT}")"
[[ -n "${CANCEL_OPERATION_ID}" && "${CANCEL_OPERATION_ID}" != "None" ]] || fail "activate --no-wait did not return an operation id"

# Cannot overlap another mutation: the in-flight activation already
# holds this server's exclusivity, admitted synchronously before the
# activate call above even returned -- a second mutation is refused
# immediately, no race needed to observe it.
expect_fail backup now
echo "second mutation correctly refused while the in-flight activation holds the server"

CANCELLED_RECORD="$(operation_cancel "${CANCEL_OPERATION_ID}")"
CANCELLED_STATE="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])' <<<"${CANCELLED_RECORD}")"
CANCELLED_STATUS_LINE="$(python3 -c 'import json,sys; print(json.load(sys.stdin).get("statusLine") or "")' <<<"${CANCELLED_RECORD}")"
[[ "${CANCELLED_STATE}" == "cancelled" ]] || fail "in-flight activation did not reach cancelled state (got ${CANCELLED_STATE}): ${CANCELLED_RECORD}"
[[ "${CANCELLED_STATUS_LINE}" == *[Cc]ancel* ]] || fail "cancelled operation record does not explain itself (statusLine: ${CANCELLED_STATUS_LINE})"

[[ "$(active_slot_id)" == "${PRE_CANCEL_ACTIVE_SLOT}" ]] || fail "a cancelled activation changed the active slot (should_cancel's own boundary is before the live world is touched)"
[[ "$(read_generation)" == "${PRE_CANCEL_GENERATION}" ]] || fail "a cancelled activation changed the live world's generation marker"
[[ -f "${SERVER_DIR}/world/CANCEL_FILLER.bin" ]] || fail "a cancelled activation touched the live world it should never have reached"
rm -f "${SERVER_DIR}/world/CANCEL_FILLER.bin"
echo "cancelled activation left the live world completely untouched (${CANCELLED_STATUS_LINE})"

# The target is usable again for a new mutation only now that the
# cancelled operation's own rollback/cleanup (removing its scratch
# `.activation/` directory) has actually finished, not merely been
# requested.
run_msc backup now >/dev/null
echo "target is usable again now that the cancelled operation's rollback/cleanup finished"

# =====================================================================
# 16. Final health check: the recovered agent still serves the full
# public path, and its most recent backup is a real, valid archive.
# =====================================================================
echo "== final health check =="
run_msc server start >/dev/null
wait_server_ready
run_msc command "say final smoke check" >/dev/null
wait_console_contains "COMMAND:say final smoke check"
run_msc server stop >/dev/null
wait_running_state "False"

python3 - "${SERVER_DIR}" <<'PY'
import json
import sys
import urllib.request
import zipfile
from pathlib import Path

server_dir = Path(sys.argv[1])
backups = sorted((server_dir / "backups").glob("*.zip"))
if not backups:
    raise SystemExit("no backups on disk after the full smoke run")
latest = backups[-1]
with zipfile.ZipFile(latest) as zf:
    bad = zf.testzip()
    if bad is not None:
        raise SystemExit(f"corrupt member in {latest}: {bad}")
    if not zf.namelist():
        raise SystemExit(f"{latest} has no entries")
PY

echo "phase6 gate smoke (synthetic) passed"
