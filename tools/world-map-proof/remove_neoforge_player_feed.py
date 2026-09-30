#!/usr/bin/env python3
"""Remove only the unchanged NeoForge player-feed proof mod."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from install_java_player_feed import running_server_pids
from install_neoforge_player_feed import MOD_NAME, STATE_NAME


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop the NeoForge server in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        if not (server / "server.properties").is_file():
            raise RuntimeError("server directory must contain server.properties")
        mods = (server / "mods").resolve(strict=True)
        if not mods.is_dir() or not mods.is_relative_to(server):
            raise RuntimeError("NeoForge mods directory must exist inside the selected server")
        pids = running_server_pids(server)
        if pids:
            raise RuntimeError(f"NeoForge server process is still running from this folder (PID(s): {pids})")
        state_path = mods / STATE_NAME
        if state_path.is_symlink() or not state_path.is_file():
            raise RuntimeError(f"proof install state is missing or unsafe: {state_path}")
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if state.get("server") != str(server):
            raise RuntimeError("proof state belongs to a different server directory")
        probe = mods / MOD_NAME
        if probe.is_symlink() or not probe.is_file():
            raise RuntimeError(f"probe file is missing or unsafe: {probe}")
        if hashlib.sha256(probe.read_bytes()).hexdigest() != state.get("probe", {}).get("sha256"):
            raise RuntimeError("probe jar changed since installation; refusing to remove it")
        probe.unlink()
        state_path.unlink()
    except (OSError, RuntimeError, ValueError, TypeError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print("Removed the NeoForge player-feed probe; all pre-existing server files and mods were left in place.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
