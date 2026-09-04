#!/usr/bin/env bash
set -Eeuo pipefail

WORKSPACE_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_ROOT="${MSC2_HEADLESS_OUTPUT:-$WORKSPACE_ROOT/target/release-headless}"
RUST_TARGET="${MSC2_RUST_TARGET:-x86_64-unknown-linux-gnu}"
AGENT_PACKAGE="${WORKSPACE_ROOT}/crates/msc-agent/Cargo.toml"

fail() {
  printf 'msc 2 linux headless build: %s\n' "$1" >&2
  exit 1
}

[[ "$(uname -s)" == "Linux" ]] || fail "run the Linux headless build on Linux"
[[ "$(uname -m)" == "x86_64" ]] || fail "the beta Linux package target is x86_64"
command -v cargo >/dev/null 2>&1 || fail "cargo is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"

if (($# > 0)); then
  (($# == 1)) || fail "usage: build-linux-headless.sh [output-directory]"
  OUTPUT_ROOT="$1"
fi

VERSION="$(awk -F'"' '/^[[:space:]]*version[[:space:]]*=/ { print $2; exit }' "$AGENT_PACKAGE")"
[[ -n "$VERSION" ]] || fail "could not read the msc-agent version"

cd "$WORKSPACE_ROOT"
cargo build --release --no-default-features --target "$RUST_TARGET" -p msc-agent

SOURCE_BINARY="$WORKSPACE_ROOT/target/$RUST_TARGET/release/msc"
[[ -x "$SOURCE_BINARY" ]] || fail "release binary is missing: $SOURCE_BINARY"

PLATFORM_DIR="$OUTPUT_ROOT/linux"
PACKAGE_ROOT="$OUTPUT_ROOT/.linux-package"
ARCHIVE="$OUTPUT_ROOT/msc2-headless-${VERSION}-linux-x86_64.tar.gz"
mkdir -p "$PLATFORM_DIR" "$PACKAGE_ROOT/systemd"
install -m 0755 "$SOURCE_BINARY" "$PLATFORM_DIR/msc"
install -m 0755 "$SOURCE_BINARY" "$PACKAGE_ROOT/msc"
install -m 0755 "$WORKSPACE_ROOT/packaging/linux/install.sh" "$PACKAGE_ROOT/install.sh"
install -m 0755 "$WORKSPACE_ROOT/packaging/linux/uninstall.sh" "$PACKAGE_ROOT/uninstall.sh"
install -m 0644 "$WORKSPACE_ROOT/packaging/linux/systemd/com.ctemple.msc2.agent.service.in" \
  "$PACKAGE_ROOT/systemd/com.ctemple.msc2.agent.service.in"
install -m 0644 "$WORKSPACE_ROOT/packaging/linux/systemd/msc2-credential-helper.socket.in" \
  "$PACKAGE_ROOT/systemd/msc2-credential-helper.socket.in"
install -m 0644 "$WORKSPACE_ROOT/packaging/linux/systemd/msc2-credential-helper.service.in" \
  "$PACKAGE_ROOT/systemd/msc2-credential-helper.service.in"
install -m 0644 "$WORKSPACE_ROOT/packaging/linux/systemd/msc2.conf.in" \
  "$PACKAGE_ROOT/systemd/msc2.conf.in"

rm -f "$ARCHIVE"
tar -czf "$ARCHIVE" -C "$PACKAGE_ROOT" .
printf 'built %s\n' "$ARCHIVE"
