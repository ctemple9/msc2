#!/usr/bin/env bash
set -Eeuo pipefail

workspace_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
agent_root="$workspace_root/clients/desktop-web/src-tauri/target/package/agent"
entitlements="$workspace_root/sidecar/bedrock/BedrockSidecar.entitlements"
platform="${1:?expected macOS platform label}"
identity="${APPLE_SIGNING_IDENTITY:?APPLE_SIGNING_IDENTITY must be configured}"

case "$platform:$(uname -m)" in
  macos-x86_64:x86_64) helpers=(msc bedrock-map vantage sidecar/BedrockSidecar) ;;
  macos-aarch64:arm64) helpers=(msc bedrock-map vantage) ;;
  *) echo "macOS platform does not match this runner: $platform" >&2; exit 1 ;;
esac

for helper in "${helpers[@]}"; do
  binary="$agent_root/$helper"
  [[ -x "$binary" ]] || { echo "missing staged macOS helper: $binary" >&2; exit 1; }
  signing_args=(--force --sign "$identity" --options runtime --timestamp)
  if [[ "$helper" == sidecar/BedrockSidecar ]]; then
    # Xcode's staging signature is ad hoc; keep its VM entitlement when replacing it.
    signing_args+=(--entitlements "$entitlements")
  fi
  codesign "${signing_args[@]}" "$binary"
  codesign --verify --strict --verbose=2 "$binary"
  details="$(codesign --display --verbose=4 "$binary" 2>&1)"
  [[ "$details" == *"Authority=Developer ID Application:"* ]] || {
    echo "helper lacks a Developer ID Application signature: $helper" >&2; exit 1;
  }
  [[ "$details" == *"flags=0x10000(runtime)"* ]] || {
    echo "helper lacks hardened runtime: $helper" >&2; exit 1;
  }
  [[ "$details" == *"Timestamp="* ]] || {
    echo "helper lacks a secure timestamp: $helper" >&2; exit 1;
  }
  if [[ "$helper" == sidecar/BedrockSidecar ]]; then
    actual="$(codesign --display --entitlements :- "$binary" 2>/dev/null |
      python3 -c 'import plistlib, sys; print(plistlib.loads(sys.stdin.buffer.read()).get("com.apple.security.virtualization") is True)')"
    [[ "$actual" == True ]] || {
      echo "BedrockSidecar lost its virtualization entitlement" >&2; exit 1;
    }
  fi
  echo "signed and verified $helper"
done
