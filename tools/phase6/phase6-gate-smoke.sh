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

usage() {
  cat <<USAGE
Usage: $0 --synthetic

Runs the Phase 6 restart-sensitive public-path gate smoke against a
committed synthetic Java world -- no real MSC 1 data, safe to run
anywhere.
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

# =====================================================================
# 12. Final health check: the recovered agent still serves the full
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
