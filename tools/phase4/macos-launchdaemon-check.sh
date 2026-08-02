#!/usr/bin/env bash
set -euo pipefail

LABEL="com.msc2.phase4.launchdaemon-check"
PLIST_PATH="/Library/LaunchDaemons/${LABEL}.plist"
SYSTEM_KEYCHAIN="/Library/Keychains/System.keychain"
DRY_RUN=0
TARGET_USER="${SUDO_USER:-${USER:-}}"
TCC_DIR=""
KEEP_ARTIFACTS=0

usage() {
  cat <<USAGE
Usage: $0 [--dry-run] [--user <installing-user>] [--tcc-dir <path>] [--keep-artifacts]

Installs a short-lived macOS LaunchDaemon test with UserName set to the
installing user, then records whether that daemon can write/read/delete a
test item in the user's login keychain, write/read/delete a test item in the
System keychain, and touch a user-selected TCC-sensitive directory.

Options:
  --dry-run         Print planned plist, daemon label, and cleanup actions only.
  --user USER       UserName to place in the LaunchDaemon plist.
                    Defaults to SUDO_USER when run through sudo.
  --tcc-dir PATH    Directory the daemon should create/write/delete in real runs.
                    Use a deliberately chosen test directory, for example under
                    Documents or on an external volume.
  --keep-artifacts  Leave the temporary worker/result directory for inspection.
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
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

if [ -z "${TARGET_USER}" ] || [ "${TARGET_USER}" = "root" ]; then
  echo "could not infer the installing user; rerun with --user <name>" >&2
  exit 2
fi

user_home() {
  local user="$1"
  local home
  home="$(/usr/bin/dscl . -read "/Users/${user}" NFSHomeDirectory 2>/dev/null | /usr/bin/awk '{print $2}')"
  if [ -z "${home}" ]; then
    home="$(eval "printf '%s' ~${user}")"
  fi
  printf '%s' "${home}"
}

USER_HOME="$(user_home "${TARGET_USER}")"
LOGIN_KEYCHAIN="${USER_HOME}/Library/Keychains/login.keychain-db"
if [ ! -e "${LOGIN_KEYCHAIN}" ]; then
  LOGIN_KEYCHAIN="${USER_HOME}/Library/Keychains/login.keychain"
fi

if [ -z "${TCC_DIR}" ]; then
  TCC_DIR="${USER_HOME}/Documents/MSC2LaunchDaemonTccCheck"
fi

RUN_ID="$(/bin/date +%Y%m%d%H%M%S)-$$"
RUN_DIR="/private/tmp/msc2-launchdaemon-check.${RUN_ID}"
WORKER_PATH="${RUN_DIR}/worker.sh"
RESULT_PATH="${RUN_DIR}/result.txt"
STDOUT_PATH="${RUN_DIR}/stdout.log"
STDERR_PATH="${RUN_DIR}/stderr.log"
SERVICE_PREFIX="${LABEL}.${RUN_ID}"
ACCOUNT="msc2-phase4-${TARGET_USER}"

print_plan() {
  cat <<PLAN
macOS LaunchDaemon keychain/TCC check
dry-run: ${DRY_RUN}
planned plist path: ${PLIST_PATH}
daemon label: ${LABEL}
LaunchDaemon UserName: ${TARGET_USER}
login keychain path: ${LOGIN_KEYCHAIN}
System keychain path: ${SYSTEM_KEYCHAIN}
TCC test directory: ${TCC_DIR}
temporary worker directory: ${RUN_DIR}
cleanup actions:
- launchctl bootout system ${PLIST_PATH}
- rm -f ${PLIST_PATH}
- delete test keychain items with service prefix ${SERVICE_PREFIX}
- rm -rf ${RUN_DIR}
PLAN
}

if [ "${DRY_RUN}" -eq 1 ]; then
  print_plan
  exit 0
fi

if [ "$(/usr/bin/uname -s)" != "Darwin" ]; then
  echo "this check only runs on macOS" >&2
  exit 2
fi

if [ "${EUID}" -ne 0 ]; then
  echo "real LaunchDaemon checks require sudo because ${PLIST_PATH} is root-owned" >&2
  exit 2
fi

if [ ! -d "${USER_HOME}" ]; then
  echo "home directory for ${TARGET_USER} does not exist: ${USER_HOME}" >&2
  exit 2
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to write the LaunchDaemon plist safely" >&2
  exit 2
fi

TARGET_GROUP="$(/usr/bin/id -gn "${TARGET_USER}")"

cleanup() {
  /bin/launchctl bootout system "${PLIST_PATH}" >/dev/null 2>&1 || true
  /bin/rm -f "${PLIST_PATH}"
  /usr/bin/sudo -u "${TARGET_USER}" /usr/bin/security delete-generic-password \
    -a "${ACCOUNT}" -s "${SERVICE_PREFIX}.login" "${LOGIN_KEYCHAIN}" >/dev/null 2>&1 || true
  /usr/bin/security delete-generic-password \
    -a "${ACCOUNT}" -s "${SERVICE_PREFIX}.system" "${SYSTEM_KEYCHAIN}" >/dev/null 2>&1 || true
  if [ "${KEEP_ARTIFACTS}" -eq 0 ]; then
    /bin/rm -rf "${RUN_DIR}"
  else
    echo "kept artifacts in ${RUN_DIR}" >&2
  fi
}
trap cleanup EXIT

/bin/mkdir -p "${RUN_DIR}"
/usr/sbin/chown "${TARGET_USER}:${TARGET_GROUP}" "${RUN_DIR}"
/bin/chmod 700 "${RUN_DIR}"

cat >"${WORKER_PATH}" <<'WORKER'
#!/usr/bin/env bash
set +e

RESULT_PATH="$1"
LOGIN_KEYCHAIN="$2"
SYSTEM_KEYCHAIN="$3"
TCC_DIR="$4"
SERVICE_PREFIX="$5"
ACCOUNT="$6"

PASSWORD="phase4-launchdaemon-check"

run_cmd() {
  local label="$1"
  shift
  echo "[$label] command: $*"
  "$@" >"/tmp/msc2-launchdaemon-check-cmd.$$" 2>&1
  local status=$?
  echo "[$label] exit: ${status}"
  sed "s/^/[$label] output: /" "/tmp/msc2-launchdaemon-check-cmd.$$"
  rm -f "/tmp/msc2-launchdaemon-check-cmd.$$"
}

check_keychain() {
  local name="$1"
  local path="$2"
  local service="${SERVICE_PREFIX}.${name}"

  echo
  echo "## ${name} keychain"
  echo "path: ${path}"
  if [ ! -e "${path}" ]; then
    echo "exists: no"
    return
  fi
  echo "exists: yes"
  run_cmd "${name}:add" /usr/bin/security add-generic-password -U -a "${ACCOUNT}" -s "${service}" -w "${PASSWORD}" "${path}"
  run_cmd "${name}:find" /usr/bin/security find-generic-password -a "${ACCOUNT}" -s "${service}" -w "${path}"
  run_cmd "${name}:delete" /usr/bin/security delete-generic-password -a "${ACCOUNT}" -s "${service}" "${path}"
}

{
  echo "msc2 Phase 4 macOS LaunchDaemon keychain/TCC check"
  echo "timestamp: $(/bin/date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "whoami: $(/usr/bin/whoami)"
  echo "id: $(/usr/bin/id)"
  echo "HOME: ${HOME:-}"
  echo "result file: ${RESULT_PATH}"

  check_keychain "login" "${LOGIN_KEYCHAIN}"
  check_keychain "system" "${SYSTEM_KEYCHAIN}"

  echo
  echo "## TCC directory"
  echo "path: ${TCC_DIR}"
  run_cmd "tcc:mkdir" /bin/mkdir -p "${TCC_DIR}"
  run_cmd "tcc:write" /bin/sh -c 'printf "%s\n" phase4-launchdaemon-check > "$1"' sh "${TCC_DIR}/msc2-launchdaemon-check.txt"
  run_cmd "tcc:read" /bin/cat "${TCC_DIR}/msc2-launchdaemon-check.txt"
  run_cmd "tcc:delete-file" /bin/rm -f "${TCC_DIR}/msc2-launchdaemon-check.txt"
  run_cmd "tcc:rmdir" /bin/rmdir "${TCC_DIR}"
} >"${RESULT_PATH}" 2>&1
WORKER

/usr/sbin/chown root:wheel "${WORKER_PATH}"
/bin/chmod 755 "${WORKER_PATH}"

python3 - "${PLIST_PATH}" "${LABEL}" "${TARGET_USER}" "${WORKER_PATH}" \
  "${RESULT_PATH}" "${LOGIN_KEYCHAIN}" "${SYSTEM_KEYCHAIN}" "${TCC_DIR}" \
  "${SERVICE_PREFIX}" "${ACCOUNT}" "${STDOUT_PATH}" "${STDERR_PATH}" <<'PY'
import plistlib
import sys

(
    plist_path,
    label,
    user,
    worker,
    result,
    login_keychain,
    system_keychain,
    tcc_dir,
    service_prefix,
    account,
    stdout_path,
    stderr_path,
) = sys.argv[1:]

plist = {
    "Label": label,
    "ProgramArguments": [
        "/bin/bash",
        worker,
        result,
        login_keychain,
        system_keychain,
        tcc_dir,
        service_prefix,
        account,
    ],
    "RunAtLoad": True,
    "UserName": user,
    "StandardOutPath": stdout_path,
    "StandardErrorPath": stderr_path,
}

with open(plist_path, "wb") as handle:
    plistlib.dump(plist, handle, sort_keys=False)
PY

/usr/sbin/chown root:wheel "${PLIST_PATH}"
/bin/chmod 644 "${PLIST_PATH}"

print_plan
echo
echo "bootstrapping LaunchDaemon..."
/bin/launchctl bootout system "${PLIST_PATH}" >/dev/null 2>&1 || true
/bin/launchctl bootstrap system "${PLIST_PATH}"

for _ in $(/usr/bin/seq 1 30); do
  if [ -s "${RESULT_PATH}" ]; then
    break
  fi
  /bin/sleep 1
done

echo
if [ -s "${RESULT_PATH}" ]; then
  /bin/cat "${RESULT_PATH}"
else
  echo "no result file was written within 30 seconds" >&2
  echo "stdout log:" >&2
  /bin/cat "${STDOUT_PATH}" >&2 2>/dev/null || true
  echo "stderr log:" >&2
  /bin/cat "${STDERR_PATH}" >&2 2>/dev/null || true
  exit 1
fi
