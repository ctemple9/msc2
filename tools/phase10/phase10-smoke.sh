#!/usr/bin/env bash
set -euo pipefail

# P10.35's one offline public-path proof. The integration test starts the
# production `msc serve` binary; each platform job supplies the fixture adapter
# selected by the real composition root. No BDS package, provider, account, or
# private world is used.

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
echo "== exercising the production Bedrock router with the platform fixture adapter =="
cargo nextest run -p msc-agent --test bedrock_production_smoke

echo "PHASE 10 CROSS-BACKEND SYNTHETIC SMOKE PASSED"
