#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ "${1:-}" != "--synthetic" || "$#" -ne 1 ]]; then
  echo "usage: $0 --synthetic" >&2
  exit 2
fi

python3 - "$ROOT/packaging/agent-service-layout.json" <<'PY'
import json
import sys

layout = json.load(open(sys.argv[1]))
assert layout["serviceName"] == "com.ctemple.msc2.agent"
assert layout["managementBind"] == "127.0.0.1:48400"
assert layout["identity"] == "installing-user"
for platform, manager in {
    "macos": "launchd LaunchDaemon",
    "windows": "Windows Service",
    "linux": "systemd",
}.items():
    entry = layout["platforms"][platform]
    assert entry["manager"] == manager
    assert entry["agentPath"]
    assert entry["dataPath"]
print("OK: packaged agent layouts preserve the installing-user service model")
PY

npm --prefix "$ROOT/clients/desktop-web" run test:agent-install
cargo test --manifest-path "$ROOT/clients/desktop-web/src-tauri/Cargo.toml" agent_service --lib
