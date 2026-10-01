#!/usr/bin/env bash
set -Eeuo pipefail

PATH_DIR="/usr/local/bin"
PATH_LINK="$PATH_DIR/msc"
INSTALL_BASE="/usr/local/lib/msc2"

fail() {
  printf 'msc 2 macOS headless installer: %s\n' "$1" >&2
  exit 1
}

usage() {
  cat <<'USAGE'
Usage: install.sh

Install the macOS headless MSC 2 command and launchd agent service.
USAGE
}

while (($# > 0)); do
  case "$1" in
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

[[ "$(uname -s)" == "Darwin" ]] || fail "this package only installs on macOS"
command -v install >/dev/null 2>&1 || fail "install is required"
command -v ln >/dev/null 2>&1 || fail "ln is required"
command -v readlink >/dev/null 2>&1 || fail "readlink is required"

case "$(uname -m)" in
  x86_64)
    ARCHITECTURE="x86_64"
    ;;
  arm64|aarch64)
    ARCHITECTURE="aarch64"
    ;;
  *)
    fail "unsupported macOS architecture: $(uname -m)"
    ;;
esac

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"
SOURCE_BINARY="$SCRIPT_DIR/msc"
SOURCE_VANTAGE="$SCRIPT_DIR/vantage"
SOURCE_VANTAGE_LICENSE="$SCRIPT_DIR/VANTAGE-LICENSE.txt"
VERSION_FILE="$SCRIPT_DIR/MSC2-VERSION"
[[ -x "$SOURCE_BINARY" ]] || fail "package binary is missing: $SOURCE_BINARY"
[[ -x "$SOURCE_VANTAGE" ]] || fail "Vantage renderer is missing: $SOURCE_VANTAGE"
[[ -f "$SOURCE_VANTAGE_LICENSE" ]] || fail "Vantage license is missing: $SOURCE_VANTAGE_LICENSE"
[[ -f "$VERSION_FILE" ]] || fail "package version file is missing: $VERSION_FILE"
VERSION="$(sed -n '1p' "$VERSION_FILE")"
[[ -n "$VERSION" && "$VERSION" != */* && "$VERSION" != *$'\n'* && "$VERSION" != *$'\r'* ]] || \
  fail "package version is invalid"

# /usr/local is intentionally used for a command installed outside the app
# bundle. Re-enter through sudo so a normal headless user gets one clear
# elevation boundary instead of needing to start the installer as root.
if ((EUID != 0)); then
  command -v sudo >/dev/null 2>&1 || fail "sudo is required for installation"
  exec sudo env "MSC2_INSTALL_HOME=$HOME" "MSC2_INSTALL_USER=$(id -un)" "$SCRIPT_DIR/install.sh" "$@"
fi

[[ -n "${MSC2_INSTALL_HOME:-}" && "${MSC2_INSTALL_HOME}" == /* ]] || fail \
  "run install.sh as the account that will own MSC 2; it requests sudo itself"
[[ -n "${MSC2_INSTALL_USER:-}" && "${MSC2_INSTALL_USER}" != "root" ]] || fail \
  "run install.sh as the account that will own MSC 2"
INSTALLING_GROUP="$(id -gn "$MSC2_INSTALL_USER")" || fail "could not resolve installing group"
DATA_DIR="$MSC2_INSTALL_HOME/Library/Application Support/MSC 2"
AGENT_PLIST="/Library/LaunchDaemons/com.ctemple.msc2.agent.plist"

ARCH_ROOT="$INSTALL_BASE/$ARCHITECTURE"
VERSION_ROOT="$ARCH_ROOT/$VERSION"
INSTALL_BINARY="$VERSION_ROOT/msc"
VANTAGE_BINARY="$VERSION_ROOT/vantage"
VANTAGE_LICENSE="$VERSION_ROOT/VANTAGE-LICENSE.txt"
OWNERSHIP_MARKER="$VERSION_ROOT/.msc2-owned"

if [[ -L "$PATH_DIR" || ( -e "$PATH_DIR" && ! -d "$PATH_DIR" ) ]]; then
  fail "command directory is not a real directory: $PATH_DIR"
fi
if [[ -L "$PATH_LINK" ]]; then
  CURRENT_LINK_TARGET="$(readlink "$PATH_LINK")"
  if [[ "$CURRENT_LINK_TARGET" != "$INSTALL_BINARY" ]]; then
    ARCH_PREFIX="$ARCH_ROOT/"
    CURRENT_VERSION_ROOT="$(dirname -- "$CURRENT_LINK_TARGET")"
    [[ "$CURRENT_LINK_TARGET" == "$ARCH_PREFIX"*/msc ]] &&
      [[ -f "$CURRENT_VERSION_ROOT/.msc2-owned" ]] &&
      [[ "$(sed -n '1p' "$CURRENT_VERSION_ROOT/.msc2-owned")" == "msc2-headless-archive" ]] || \
      fail "existing non-MSC command target at $PATH_LINK; move it before installing"
  fi
elif [[ -e "$PATH_LINK" ]]; then
  fail "existing non-MSC command target at $PATH_LINK; move it before installing"
fi

if [[ -e "$VERSION_ROOT" && ! -f "$OWNERSHIP_MARKER" ]]; then
  fail "existing non-MSC installation directory: $VERSION_ROOT"
fi
if [[ -f "$AGENT_PLIST" ]]; then
  SERVICE_BINARY="$(plutil -extract ProgramArguments.0 raw -o - "$AGENT_PLIST" 2>/dev/null)" || \
    fail "could not inspect the existing agent service"
  [[ "$SERVICE_BINARY" == "$INSTALL_BASE/"*/msc ]] || fail \
    "the installed agent service belongs to another MSC installation"
fi

install -d -m 0755 "$PATH_DIR" "$VERSION_ROOT"
install -m 0755 "$SOURCE_BINARY" "$INSTALL_BINARY"
install -m 0755 "$SOURCE_VANTAGE" "$VANTAGE_BINARY"
install -m 0644 "$SOURCE_VANTAGE_LICENSE" "$VANTAGE_LICENSE"
if [[ -f "$SCRIPT_DIR/BEDROCK-HELPER-PLIST.in" ]]; then
  install -m 0644 "$SCRIPT_DIR/BEDROCK-HELPER-PLIST.in" "$VERSION_ROOT/BEDROCK-HELPER-PLIST.in"
fi
if [[ -d "$SCRIPT_DIR/sidecar" ]]; then
  install -d -m 0755 "$VERSION_ROOT/sidecar"
  for sidecar_file in BedrockSidecar vmlinuz-kata appliance-initramfs.gz; do
    [[ -e "$SCRIPT_DIR/sidecar/$sidecar_file" ]] || fail \
      "package sidecar file is missing: $sidecar_file"
    install -m 0755 "$SCRIPT_DIR/sidecar/$sidecar_file" "$VERSION_ROOT/sidecar/$sidecar_file"
  done
fi
printf 'msc2-headless-archive\n' > "$OWNERSHIP_MARKER"
chmod 0644 "$OWNERSHIP_MARKER"

if [[ -L "$PATH_LINK" ]]; then
  ln -sfn "$INSTALL_BINARY" "$PATH_LINK"
else
  ln -s "$INSTALL_BINARY" "$PATH_LINK"
fi

install -d -m 0700 -o "$MSC2_INSTALL_USER" -g "$INSTALLING_GROUP" "$DATA_DIR" "$DATA_DIR/logs" "$DATA_DIR/servers"
"$INSTALL_BINARY" service install \
  --service-name com.ctemple.msc2.agent \
  --binary-path "$INSTALL_BINARY" \
  --working-directory "$DATA_DIR" \
  --log-path "$DATA_DIR/logs/agent.log" \
  --run-user "$MSC2_INSTALL_USER" \
  --expected-port 48001 \
  --arg serve --arg=--bind --arg 127.0.0.1:48001 \
  --env "HOME=$MSC2_INSTALL_HOME" \
  --env "MSC2_DATA_DIR=$DATA_DIR"
/bin/launchctl kickstart -k system/com.ctemple.msc2.agent

cat <<MESSAGE
MSC 2 macOS headless command installed.

The command is installed as $PATH_LINK and is available as msc.
$(if [[ ":$PATH:" == *":$PATH_DIR:"* ]]; then
    printf 'This shell already includes %s on PATH.\n' "$PATH_DIR"
  else
    printf 'Refresh PATH or open a new shell before using it from this shell.\n'
  fi)

The launchd agent is enabled at boot and running as $MSC2_INSTALL_USER.
Use msc status agent, msc stop agent, and msc start agent for local control.
MESSAGE
