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

fail() {
  printf 'msc 2 linux uninstaller: %s\n' "$1" >&2
  exit 1
}

[[ "$(uname -s)" == "Linux" ]] || fail "this package only uninstalls on Linux"
command -v systemctl >/dev/null 2>&1 || fail "systemctl is required"
command -v systemd-tmpfiles >/dev/null 2>&1 || fail "systemd-tmpfiles is required"
command -v sudo >/dev/null 2>&1 || ((EUID == 0)) || fail "sudo is required for uninstallation"

if ((EUID != 0)); then
  exec sudo "$SCRIPT_DIR/uninstall.sh" "$@"
fi

if (($# > 0)); then
  [[ "$1" == "--help" || "$1" == "-h" ]] || fail "unknown argument: $1"
  cat <<'USAGE'
Usage: uninstall.sh

Stop and remove the MSC 2 Linux service definitions and installed binary.
Managed server data, logs, configuration, and credential blobs are retained.
USAGE
  exit 0
fi

# Keep this list explicit. Uninstall only controls the three MSC units and
# never asks systemd to stop an unrelated service on the host.
systemctl stop "$AGENT_UNIT" "$HELPER_SERVICE_UNIT" "$HELPER_SOCKET_UNIT" >/dev/null 2>&1 || true
systemctl disable "$AGENT_UNIT" "$HELPER_SERVICE_UNIT" "$HELPER_SOCKET_UNIT" >/dev/null 2>&1 || true
systemctl daemon-reload

rm -f \
  "$UNIT_DIR/$AGENT_UNIT" \
  "$UNIT_DIR/$HELPER_SOCKET_UNIT" \
  "$UNIT_DIR/$HELPER_SERVICE_UNIT" \
  "$TMPFILES_DIR/$TMPFILES_UNIT" \
  "$INSTALL_BIN"

systemd-tmpfiles --remove "$TMPFILES_DIR/$TMPFILES_UNIT" >/dev/null 2>&1 || true
systemctl daemon-reload
rmdir /run/msc2 >/dev/null 2>&1 || true

cat <<'MESSAGE'
MSC 2 Linux service definitions and binary removed.

Managed server data, logs, configuration, and the root-owned credential store
were retained. Review those paths before removing them manually if a complete
data purge is wanted.
MESSAGE
