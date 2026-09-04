#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_BIN="/usr/lib/msc2/msc"
UNIT_DIR="/etc/systemd/system"
TMPFILES_DIR="/usr/lib/tmpfiles.d"
AGENT_UNIT="com.ctemple.msc2.agent.service"
HELPER_SOCKET_UNIT="msc2-credential-helper.socket"
HELPER_SERVICE_UNIT="msc2-credential-helper.service"
TMPFILES_UNIT="msc2.conf"
DATA_DIR_OVERRIDE="${MSC2_DATA_DIR:-}"

fail() {
  printf 'msc 2 linux installer: %s\n' "$1" >&2
  exit 1
}

usage() {
  cat <<'USAGE'
Usage: install.sh [--data-dir PATH]

Install the Linux headless MSC 2 agent for the invoking user. The installer
requests elevation itself; do not run pairing commands through sudo.
USAGE
}

while (($# > 0)); do
  case "$1" in
    --data-dir)
      (($# >= 2)) || fail "--data-dir needs a path"
      DATA_DIR_OVERRIDE="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      fail "unknown argument: $1"
      ;;
  esac
done

[[ "$(uname -s)" == "Linux" ]] || fail "this package only installs on Linux"
command -v systemctl >/dev/null 2>&1 || fail "systemctl is required"
command -v systemd-tmpfiles >/dev/null 2>&1 || fail "systemd-tmpfiles is required"
command -v getent >/dev/null 2>&1 || fail "getent is required"
command -v install >/dev/null 2>&1 || fail "install is required"

# The script is normally started by the installing user. Re-enter through
# sudo so the root half can write /usr/lib, /etc, /run, and /var/lib while the
# SUDO_* values preserve the user who owns the server files.
if ((EUID != 0)); then
  command -v sudo >/dev/null 2>&1 || fail "sudo is required for installation"
  if [[ -n "$DATA_DIR_OVERRIDE" ]]; then
    exec sudo env "MSC2_DATA_DIR=$DATA_DIR_OVERRIDE" "$SCRIPT_DIR/install.sh" "$@"
  fi
  exec sudo "$SCRIPT_DIR/install.sh" "$@"
fi

[[ -n "${SUDO_UID:-}" && "${SUDO_UID}" != "0" ]] || fail \
  "run install.sh as the user who will own MSC 2; it requests sudo itself"

INSTALLING_UID="$SUDO_UID"
INSTALLING_USER="$(id -nu "$INSTALLING_UID")" || fail "could not resolve installing user"
INSTALLING_GROUP="$(id -ng "$INSTALLING_UID")" || fail "could not resolve installing group"
INSTALLING_HOME="$(getent passwd "$INSTALLING_UID" | cut -d: -f6)"
[[ -n "$INSTALLING_HOME" ]] || fail "could not resolve installing user's home directory"

DATA_DIR="${DATA_DIR_OVERRIDE:-$INSTALLING_HOME/.local/share/msc2}"
[[ "$DATA_DIR" == /* ]] || fail "MSC2_DATA_DIR must be an absolute path"
[[ "$DATA_DIR" != "/" && "$DATA_DIR" != "/usr" && "$DATA_DIR" != "/etc" && "$DATA_DIR" != "/var" && "$DATA_DIR" != "/run" ]] || \
  fail "MSC2_DATA_DIR points at a protected system directory"
[[ "$DATA_DIR" != *$'\n'* && "$DATA_DIR" != *$'\r'* && "$DATA_DIR" != *'"'* && "$DATA_DIR" != *'\\'* ]] || \
  fail "MSC2_DATA_DIR contains a character that cannot be represented safely in a systemd unit"

for required in \
  "$SCRIPT_DIR/msc" \
  "$SCRIPT_DIR/systemd/com.ctemple.msc2.agent.service.in" \
  "$SCRIPT_DIR/systemd/msc2-credential-helper.socket.in" \
  "$SCRIPT_DIR/systemd/msc2-credential-helper.service.in" \
  "$SCRIPT_DIR/systemd/msc2.conf.in"; do
  [[ -f "$required" ]] || fail "package input is missing: $required"
done

render_template() {
  local template="$1"
  local output="$2"
  awk \
    -v installing_user="$INSTALLING_USER" \
    -v installing_group="$INSTALLING_GROUP" \
    -v installing_uid="$INSTALLING_UID" \
    -v data_dir="$DATA_DIR" \
    '
      function replace_all(line, marker, replacement, position, prefix, suffix) {
        while ((position = index(line, marker)) != 0) {
          prefix = substr(line, 1, position - 1)
          suffix = substr(line, position + length(marker))
          line = prefix replacement suffix
        }
        return line
      }
      {
        line = $0
        line = replace_all(line, "@MSC2_USER@", installing_user)
        line = replace_all(line, "@MSC2_GROUP@", installing_group)
        line = replace_all(line, "@MSC2_UID@", installing_uid)
        line = replace_all(line, "@MSC2_DATA_DIR@", data_dir)
        print line
      }
    ' "$template" > "$output"
}

TEMP_DIR="$(mktemp -d /tmp/msc2-linux-install.XXXXXX)"
cleanup() {
  rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

render_template "$SCRIPT_DIR/systemd/com.ctemple.msc2.agent.service.in" \
  "$TEMP_DIR/$AGENT_UNIT"
render_template "$SCRIPT_DIR/systemd/msc2-credential-helper.socket.in" \
  "$TEMP_DIR/$HELPER_SOCKET_UNIT"
render_template "$SCRIPT_DIR/systemd/msc2-credential-helper.service.in" \
  "$TEMP_DIR/$HELPER_SERVICE_UNIT"
render_template "$SCRIPT_DIR/systemd/msc2.conf.in" \
  "$TEMP_DIR/$TMPFILES_UNIT"

# Stop the old definitions before replacing them. This is an upgrade-safe
# boundary: managed server files and the root-owned credential store are not
# touched, while the next start uses the new binary and unit definitions.
systemctl stop "$AGENT_UNIT" "$HELPER_SERVICE_UNIT" "$HELPER_SOCKET_UNIT" >/dev/null 2>&1 || true
systemctl disable "$AGENT_UNIT" "$HELPER_SERVICE_UNIT" "$HELPER_SOCKET_UNIT" >/dev/null 2>&1 || true

install -d -m 0755 -o root -g root "$(dirname "$INSTALL_BIN")"
install -m 0755 -o root -g root "$SCRIPT_DIR/msc" "$INSTALL_BIN"

# These directories belong to the installing user. Do not recursively chown
# an existing data directory: an upgrade must not rewrite ownership inside a
# managed server tree the user has deliberately arranged.
install -d -m 0700 -o "$INSTALLING_USER" -g "$INSTALLING_GROUP" "$DATA_DIR"
install -d -m 0700 -o "$INSTALLING_USER" -g "$INSTALLING_GROUP" "$DATA_DIR/logs"
install -d -m 0700 -o "$INSTALLING_USER" -g "$INSTALLING_GROUP" "$DATA_DIR/servers"

install -d -m 0755 -o root -g root "$UNIT_DIR" "$TMPFILES_DIR"
install -m 0644 -o root -g root "$TEMP_DIR/$AGENT_UNIT" "$UNIT_DIR/$AGENT_UNIT"
install -m 0644 -o root -g root "$TEMP_DIR/$HELPER_SOCKET_UNIT" "$UNIT_DIR/$HELPER_SOCKET_UNIT"
install -m 0644 -o root -g root "$TEMP_DIR/$HELPER_SERVICE_UNIT" "$UNIT_DIR/$HELPER_SERVICE_UNIT"
install -m 0644 -o root -g root "$TEMP_DIR/$TMPFILES_UNIT" "$TMPFILES_DIR/$TMPFILES_UNIT"

install -d -m 0700 -o root -g root /var/lib/msc2/credentials
systemd-tmpfiles --create "$TMPFILES_DIR/$TMPFILES_UNIT"
systemctl daemon-reload
systemctl enable "$HELPER_SOCKET_UNIT" "$AGENT_UNIT" >/dev/null
systemctl start "$HELPER_SOCKET_UNIT"
systemctl start "$AGENT_UNIT"

cat <<MESSAGE
MSC 2 headless agent installed for ${INSTALLING_USER}.

The agent is enabled for boot and is running under ${INSTALLING_USER}.
Routine control:
  systemctl status ${AGENT_UNIT}
  systemctl start ${AGENT_UNIT}
  systemctl stop ${AGENT_UNIT}

Create a desktop pairing code locally as ${INSTALLING_USER}; never run pairing as root:
  msc pairing create --client-kind desktop

The pairing code is one-use and short-lived. It is not written to the unit,
shell history, ordinary configuration, or release metadata.
MESSAGE
