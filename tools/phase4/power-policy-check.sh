#!/bin/sh
set -eu

if [ "${1:-}" != "--dry-run" ]; then
  echo "usage: tools/phase4/power-policy-check.sh --dry-run" >&2
  exit 2
fi

python3 - <<'PY'
from dataclasses import dataclass

@dataclass(frozen=True)
class Scenario:
    name: str
    host_role: str
    remote_management: bool
    running_servers: int
    critical_operations: int

def decide(s: Scenario):
    if s.host_role == "dedicated-headless" and s.remote_management:
        return True, "remote management is enabled on a dedicated/headless host"
    if s.running_servers and s.critical_operations:
        return True, f"{s.running_servers} server(s) and {s.critical_operations} critical operation(s) are running"
    if s.running_servers:
        return True, f"{s.running_servers} server(s) are running"
    if s.critical_operations:
        return True, f"{s.critical_operations} critical operation(s) are running"
    return False, ""

def action(platform: str, reason: str):
    if platform == "macos":
        return f'IOPMAssertionCreateWithName(PreventUserIdleSystemSleep, level=255, reason="MSC2: {reason}")'
    if platform == "linux":
        return "systemd-inhibit --what=sleep --who=MSC2 --mode=block --why='{0}' /bin/sh -c 'trap '\\''exit 0'\\'' TERM INT; while :; do sleep 3600; done'".format(reason)
    return "SetThreadExecutionState({0})".format(
        "ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_AWAYMODE_REQUIRED"
        if "dedicated/headless host" in reason
        else "ES_CONTINUOUS | ES_SYSTEM_REQUIRED"
    )

scenarios = [
    Scenario("dedicated idle remote-management", "dedicated-headless", True, 0, 0),
    Scenario("desktop idle remote-management", "normal-desktop", True, 0, 0),
    Scenario("desktop running server", "normal-desktop", False, 1, 0),
    Scenario("desktop critical operation", "normal-desktop", False, 0, 1),
]

for scenario in scenarios:
    prevent_sleep, reason = decide(scenario)
    status = "prevent-sleep" if prevent_sleep else "allow-sleep"
    print(f"scenario={scenario.name} policy={status}")
    if prevent_sleep:
        for platform in ("macos", "linux", "windows"):
            print(f"  {platform}: {action(platform, reason)}")

print("warning-probes:")
print("  macos: pmset -g custom (sleep timer, hibernatemode, standby, autopoweroff)")
print("  linux: /etc/systemd/logind.conf (HandleLidSwitch, IdleAction)")
print("  windows: powercfg /query (standby and hibernate timers)")
print("ok power-policy-dry-run")
PY
