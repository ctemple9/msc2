#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != "--synthetic" ]]; then
  printf '%s\n' 'usage: bash tools/phase12/reset-smoke.sh --synthetic' >&2
  exit 2
fi

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_root"

python3 tools/phase12/reset-contract-check.py --selftest >/dev/null

python3 - <<'PY'
from pathlib import Path

app = Path("clients/desktop-web/src/App.svelte").read_text()
setup = Path("clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte").read_text()
gate = Path("clients/desktop-web/src/lib/help/FirstLaunchGate.svelte").read_text()
intro = Path("clients/desktop-web/src/lib/help/SetupIntro.svelte").read_text()
evidence = Path("docs/msc2/clients/evidence/reset-recovery.md")
matrix = Path("docs/msc2/client-capability-matrix.csv").read_text()

required = {
    "local install continuation": "Install and Continue" in setup,
    "local stopped continuation": "Start and Continue" in setup,
    "incompatible repair": "readiness === 'incompatible'" in setup and "Repair service" in setup,
    "remote pairing action": "async function pairAgain" in app and "Pair Again" in setup,
    "new host identity": "hostStore.removeHost(previousHost.id)" in app,
    "agent-owned setup completion": "/v1/config/host-setup/complete" in intro,
    "no server creation during recovery": "/v1/servers/create" not in gate and "/v1/servers/create" not in setup,
    "walkthrough evidence": evidence.exists(),
    "matrix trace": "P12.19d" in matrix,
}

failed = [name for name, passed in required.items() if not passed]
if failed:
    raise SystemExit("reset smoke failed: " + ", ".join(failed))

print("reset smoke: " + ", ".join(required))
PY
