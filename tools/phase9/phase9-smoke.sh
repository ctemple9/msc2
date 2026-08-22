#!/usr/bin/env bash
set -euo pipefail

# P9.14's offline synthetic proof.  No provider credentials, public helper
# downloads, or production server paths are used here.  The Rust suites use
# fake transports and temporary directories where they need state.

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

for tool in cargo python3; do
  command -v "${tool}" >/dev/null 2>&1 || fail "missing required tool: ${tool}"
done

cd "${ROOT}"
echo "== checking recorded evidence and management boundary =="
python3 tools/phase9/phase9-check.py --evidence
python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv

echo "== exercising synthetic domain and helper behavior =="
cargo nextest run -p msc-domain \
  --test networking \
  --test helper \
  --test paper_plugin_crash_analysis

echo "== exercising synthetic infrastructure behavior =="
cargo nextest run -p msc-infrastructure \
  --test helper_process \
  --test helper_acquisition \
  --test geyser_resolution \
  --test credential_repository

echo "== exercising synthetic application behavior =="
cargo nextest run -p msc-application \
  --test playit \
  --test resource_packs \
  --test network_diagnostics \
  --test geyser \
  --test xbox_broadcast

echo "== exercising public HTTP and CLI path behavior =="
cargo nextest run -p msc-agent \
  --test playit_routes \
  --test resource_pack_routes \
  --test network_diagnostic_routes \
  --test geyser_routes \
  --test xbox_broadcast_routes \
  --test phase9_routes \
  --test cli_phase9

echo "PHASE 9 SYNTHETIC SMOKE PASSED"
