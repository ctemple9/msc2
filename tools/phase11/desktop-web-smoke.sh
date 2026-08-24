#!/usr/bin/env bash
set -euo pipefail

# P11.28's offline guard for the CI topology.  It deliberately checks the
# candidate recipe rather than pretending a local Mac can prove the Windows
# renderer, or vice versa.  Each runner records its actual versions separately.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  cat >&2 <<'EOF'
usage:
  bash tools/phase11/desktop-web-smoke.sh --synthetic --all-surfaces
  bash tools/phase11/desktop-web-smoke.sh --record-platform <linux|macos|windows> <output.md>
EOF
}

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

require_text() {
  local file="$1"
  local text="$2"
  rg -F --quiet "$text" "$file" || fail "missing '$text' in $file"
}

synthetic() {
  local ci="$ROOT/.github/workflows/ci.yml"
  local package="$ROOT/clients/desktop-web/package.json"
  local evidence="$ROOT/docs/msc2/clients/evidence/tri-platform-ci.md"

  [[ -f "$ci" ]] || fail "CI workflow is missing"
  [[ -f "$package" ]] || fail "client package manifest is missing"
  [[ -f "$evidence" ]] || fail "tri-platform evidence policy is missing"

  for platform in linux macos windows; do
    require_text "$ci" "platform: $platform"
  done
  require_text "$ci" "desktop-web-evidence-\${{ matrix.platform }}"
  for command in \
    "npm run verify:scaffold" \
    "cargo nextest run -p msc-agent --test web_ui" \
    "npm run test:e2e-browser" \
    "npm exec tauri build -- --debug --no-bundle" \
    "bash tools/phase11/linux-webkitgtk-smoke.sh --native" \
    "python3 tools/phase4/headless-link-check.py --all-artifacts target/phase4-headless"; do
    require_text "$ci" "$command"
  done
  for package_name in libwebkit2gtk-4.1-0 webkit2gtk-driver xvfb; do
    require_text "$ci" "$package_name"
  done
  for script in format:check check test:unit build bundle:identity; do
    require_text "$package" "\"$script\""
  done
  require_text "$evidence" "Signing and notarization status"
  require_text "$evidence" "unavailable"

  echo "OK: CI exercises the shared production frontend and agent-served browser bundle on macOS, Linux, and Windows; Linux uses native WebKitGTK and headless proof remains separate"
}

record_platform() {
  local platform="$1"
  local output="$2"
  mkdir -p "$(dirname "$output")"

  local renderer
  case "$platform" in
    linux)
      renderer="WebKitGTK package: $(dpkg-query -W -f='${Version}' libwebkit2gtk-4.1-0 2>/dev/null || printf 'unavailable')"
      ;;
    macos)
      renderer="WKWebView / WebKit framework: $(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' /System/Library/Frameworks/WebKit.framework/Versions/A/Resources/Info.plist 2>/dev/null || sw_vers -productVersion)"
      ;;
    windows)
      renderer="WebView2 runtime: $(powershell -NoProfile -Command '$p=Get-ChildItem "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients" -ErrorAction SilentlyContinue | Get-ItemProperty; ($p | Where-Object {$_.name -match "WebView"} | Select-Object -First 1 -ExpandProperty pv)' 2>/dev/null || printf 'unavailable')"
      ;;
    *) fail "unknown platform '$platform'" ;;
  esac

  cat >"$output" <<EOF
# P11.28 ${platform} desktop and web candidate

- Node.js: $(node --version)
- npm: $(npm --version)
- Rust: $(rustc --version)
- ${renderer}
- Tauri CLI: $(npm --prefix "$ROOT/clients/desktop-web" exec tauri -- --version)

## Signing and notarization status

Unavailable in CI: this candidate has no release signing certificate,
notarization credential, or release-distribution claim. The job proves a
debug, unbundled Tauri build only; release signing and notarization need their
own credentialed release workflow and evidence.
EOF
}

if [[ "${1:-}" == "--synthetic" && "${2:-}" == "--all-surfaces" && "$#" -eq 2 ]]; then
  synthetic
elif [[ "${1:-}" == "--record-platform" && "$#" -eq 3 ]]; then
  record_platform "$2" "$3"
else
  usage
  exit 2
fi
