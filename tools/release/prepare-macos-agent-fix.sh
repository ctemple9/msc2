#!/bin/bash
set -euo pipefail

# Keep the desktop's content-addressed agent staging and bootstrap identity
# consistent by changing a copied app, rather than overwriting its cached agent.
PACKAGE_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"
SOURCE_AGENT="${1:-$PACKAGE_DIR/msc}"
EXPECTED_VERSION="${2:-$(cat "$PACKAGE_DIR/MSC2-VERSION")}"
OUTPUT_DIR="${3:-$PACKAGE_DIR/patched}"
SOURCE_APP="${4:-/Applications/MSC 2.app}"
PATCHED_APP="$OUTPUT_DIR/MSC 2.app"

fail() { printf '%s\n' "$*" >&2; exit 1; }
[[ "$(uname -s):$(uname -m)" == Darwin:x86_64 ]] || fail 'Run this package on the Intel Mac.'
[[ -f "$SOURCE_AGENT" && ! -L "$SOURCE_AGENT" ]] || fail 'The replacement agent is missing or is a symbolic link.'
[[ -d "$SOURCE_APP" && ! -L "$SOURCE_APP" ]] || fail 'The installed MSC 2 app is missing or is a symbolic link.'
[[ ! -e "$PATCHED_APP" ]] || fail "Output already exists: $PATCHED_APP. Choose a new output folder."
/usr/bin/lipo "$SOURCE_AGENT" -verify_arch x86_64
APP_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$SOURCE_APP/Contents/Info.plist")"
[[ "$APP_VERSION" == "$EXPECTED_VERSION" ]] || fail "This patch requires MSC 2 $EXPECTED_VERSION; the installed app is $APP_VERSION."
[[ -f "$SOURCE_APP/Contents/Resources/agent/msc" ]] || fail 'The installed app has no bundled agent.'

mkdir -p "$OUTPUT_DIR"
/usr/bin/ditto "$SOURCE_APP" "$PATCHED_APP"
/usr/bin/install -m 0755 "$SOURCE_AGENT" "$PATCHED_APP/Contents/Resources/agent/msc"
/usr/bin/codesign --force --sign - "$PATCHED_APP/Contents/Resources/agent/msc"
# Only the replaced agent and outer app need new signatures. Leave the Swift
# helper's existing signature and virtualization entitlement intact.
/usr/bin/codesign --force --sign - --preserve-metadata=entitlements,flags,runtime "$PATCHED_APP"
/usr/bin/codesign --verify --deep --strict "$PATCHED_APP"
printf '\nPrepared: %s\n' "$PATCHED_APP"
printf '%s\n' 'Stop Minecraft in MSC and wait for Stopped, then quit MSC.'
printf '%s\n' 'Copy the prepared app into Applications, replacing MSC 2. Open it and use Repair agent before starting Minecraft.'
printf '%s\n' 'The installed app, running services, settings and worlds have not been changed by this preparation.'
/usr/bin/open -R "$PATCHED_APP"
