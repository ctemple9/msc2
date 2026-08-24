#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ "${1:-}" != "--synthetic" || "${2:-}" != "--all-platforms" || "$#" -ne 2 ]]; then
  echo "usage: $0 --synthetic --all-platforms" >&2
  exit 2
fi

python3 - "$ROOT/docs/msc2/clients/phase11-update.md" "$ROOT/packaging/update-release-schema.json" <<'PY'
import json
import sys
from pathlib import Path

policy = Path(sys.argv[1]).read_text().lower()
required = [
    "signed", "sha-256", "explicit confirmation", "rollback",
    "configuration", "secrets", "worlds", "package manager", "d-010",
]
missing = [term for term in required if term not in policy]
assert not missing, missing

schema = json.loads(Path(sys.argv[2]).read_text())
assert schema["macos"]["requiredArtifacts"] == ["desktop", "agent", "sidecar"]
assert schema["windows"]["requiredArtifacts"] == ["desktop", "agent"]
assert schema["linux"]["installation"] == "package-manager-only"
assert set(schema["excludedUpdateFamilies"]) == {"minecraft-server", "loader", "add-on", "modpack"}
print("OK: all-platform coordinated-update policy is present")
PY

npm --prefix "$ROOT/clients/desktop-web" run test:updates
cargo test --manifest-path "$ROOT/clients/desktop-web/src-tauri/Cargo.toml" coordinated_update --lib
