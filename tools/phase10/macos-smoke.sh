#!/usr/bin/env bash
set -euo pipefail

# P10.18's offline macOS-sidecar proof. The test uses the real Rust sidecar
# adapter and a fake JSON-lines transport; it never boots a VM or downloads BDS.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

usage() {
  echo "Usage: $0 --synthetic"
}

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

[[ "${1:-}" == "--synthetic" && "$#" -eq 1 ]] || {
  usage >&2
  exit 2
}

command -v cargo >/dev/null 2>&1 || fail "missing required tool: cargo"

cd "${ROOT}"
echo "== exercising the synthetic macOS agent-to-sidecar lifecycle =="
cargo nextest run -p msc-agent --test bedrock_macos_routes

echo "== exercising the Rust macOS sidecar adapter =="
cargo nextest run -p msc-application --test bedrock_macos

echo "P10.18 MACOS SYNTHETIC SMOKE PASSED"
