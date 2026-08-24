#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != "--native" || $# -ne 1 ]]; then
  echo "usage: bash tools/phase11/linux-webkitgtk-smoke.sh --native" >&2
  exit 64
fi

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "This smoke test must run on Linux: it exercises Tauri through system WebKitGTK." >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
client_dir="$repo_root/clients/desktop-web"
evidence_dir="$repo_root/docs/msc2/clients/evidence"
driver_bin="${MSC_TAURI_DRIVER:-$(command -v tauri-driver || true)}"
webkit_driver="$(command -v WebKitWebDriver || command -v webkit2gtk-driver || true)"

for required in "$driver_bin" "$webkit_driver" "$(command -v Xvfb || true)" "$(command -v node || true)" "$(command -v npm || true)"; do
  if [[ -z "$required" ]]; then
    echo "missing required native-renderer dependency (tauri-driver, WebKitWebDriver/webkit2gtk-driver, Xvfb, node, or npm)" >&2
    exit 1
  fi
done

mkdir -p "$evidence_dir"
work_dir="$(mktemp -d)"
cleanup() {
  [[ -n "${harness_pid:-}" ]] && kill "$harness_pid" 2>/dev/null || true
  [[ -n "${xvfb_pid:-}" ]] && kill "$xvfb_pid" 2>/dev/null || true
  rm -rf "$work_dir"
}
trap cleanup EXIT

cd "$client_dir"
npm run build
PORT=4173 node ./tests/e2e/browser/contract-harness.mjs >"$work_dir/contract-harness.log" 2>&1 &
harness_pid=$!
for _ in $(seq 1 30); do
  curl --fail --silent http://127.0.0.1:4173/v1/capabilities >/dev/null && break
  sleep 1
done
curl --fail --silent http://127.0.0.1:4173/v1/capabilities >/dev/null

VITE_MSC_API_BASE_URL=http://127.0.0.1:4173 npm exec tauri build -- --debug --no-bundle
tauri_binary="$client_dir/src-tauri/target/debug/msc2-desktop-web"
if [[ ! -x "$tauri_binary" ]]; then
  echo "expected built Tauri binary at $tauri_binary" >&2
  exit 1
fi

Xvfb :99 -screen 0 1280x960x24 >"$work_dir/xvfb.log" 2>&1 &
xvfb_pid=$!
export DISPLAY=:99
export MSC_TAURI_BINARY="$tauri_binary"
export MSC_TAURI_DRIVER="$driver_bin"
export MSC_WEBKITGTK_SCREENSHOT="$evidence_dir/linux-webkitgtk-native.png"
MSC_EXPECT_MOTION=fallback npm run test:e2e-tauri-linux

if command -v gsettings >/dev/null 2>&1 && command -v dbus-run-session >/dev/null 2>&1; then
  reduced_motion_config="$work_dir/reduced-motion-config"
  mkdir -p "$reduced_motion_config/gtk-3.0"
  printf '[Settings]\ngtk-enable-animations=false\n' >"$reduced_motion_config/gtk-3.0/settings.ini"
  dbus-run-session -- env XDG_CONFIG_HOME="$reduced_motion_config" bash -c '
    gsettings set org.gnome.desktop.interface enable-animations false
    MSC_EXPECT_MOTION=reduced npm run test:e2e-tauri-linux
  '
else
  echo "missing gsettings or dbus-run-session required for the WebKitGTK reduced-motion branch" >&2
  exit 1
fi

webkit_version="$($webkit_driver --version 2>&1 | head -n 1 || true)"
if command -v dpkg-query >/dev/null 2>&1; then
  package_version="$(dpkg-query -W -f='${Version}' libwebkit2gtk-4.1-0 2>/dev/null || dpkg-query -W -f='${Version}' libwebkit2gtk-4.0-37 2>/dev/null || true)"
else
  package_version="unavailable (non-Debian package query)"
fi
cat >"$evidence_dir/linux-webkitgtk-native.md" <<EOF
# Linux WebKitGTK native-renderer evidence

- Runner: $(. /etc/os-release && printf '%s %s' "$NAME" "$VERSION_ID")
- WebKitGTK package version: ${package_version:-not reported}
- Native driver: $webkit_driver${webkit_version:+ ($webkit_version)}
- Tauri driver: $driver_bin
- Display server: Xvfb :99
- Screenshot: [linux-webkitgtk-native.png](linux-webkitgtk-native.png)

The production Svelte bundle was built into the debug Tauri binary and driven
through Tauri's Linux WebDriver bridge to the system WebKitGTK renderer. The
run covers the visible shell, navigation, CSS layout, dialog, deterministic
mutation, console view, deep link, fresh-profile entry, and reduced-motion
fallback.
EOF

echo "Linux WebKitGTK native Tauri smoke passed"
