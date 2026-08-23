#!/usr/bin/env bash
set -euo pipefail

# P10.23's one offline public-path proof. The integration tests run the same
# loopback API harness from both the HTTP and real `msc` CLI tests; the three
# backend labels only select native Linux, native Windows, or macOS sidecar
# capability identity. No BDS package, provider, account, or private world is
# used.

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
cargo nextest --version >/dev/null 2>&1 || fail "cargo-nextest is not installed"

cd "${ROOT}"
echo "== exercising the shared Bedrock API and CLI workflow across three fakes =="
cargo nextest run -p msc-agent --test bedrock_routes --test bedrock_cli

echo "PHASE 10 CROSS-BACKEND SYNTHETIC SMOKE PASSED"
