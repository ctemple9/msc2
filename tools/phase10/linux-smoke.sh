#!/usr/bin/env bash
set -euo pipefail

# P10.13's offline public-contract proof.  The tests use a fake BDS boundary;
# this script never downloads a distribution, reads an account, or contacts a
# public server.

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

for tool in cargo; do
  command -v "${tool}" >/dev/null 2>&1 || fail "missing required tool: ${tool}"
done

cd "${ROOT}"
echo "== exercising the Bedrock application boundary =="
cargo nextest run -p msc-application --test bedrock_service

echo "== exercising the synthetic Linux HTTP and CLI contract =="
cargo nextest run -p msc-agent \
  --test bedrock_linux_routes \
  --test bedrock_linux_cli

echo "P10.13 LINUX SYNTHETIC SMOKE PASSED"
