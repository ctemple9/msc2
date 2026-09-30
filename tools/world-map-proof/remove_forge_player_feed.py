#!/usr/bin/env python3
"""Remove only the unchanged Forge player-feed mod installed by its helper."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from install_forge_player_feed import (
    MODS_PATH,
    PROBE_NAME,
    STATE_NAME,
    SUPPORTED_FORGE_VERSION,
    SUPPORTED_GAME_VERSION,
    running_server_pids,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop the Forge server in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        if not (server / "server.properties").is_file():
            parser.error("server directory must contain server.properties")
        mods = (server / MODS_PATH).resolve(strict=True)
        if not mods.is_dir() or not mods.is_relative_to(server):
            parser.error("Forge mods directory must exist inside the selected server")
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Forge server process is still running from this folder (PID(s): {pids})")
        state_path = mods / STATE_NAME
        if state_path.is_symlink() or not state_path.is_file():
            parser.error(f"proof install state is missing or unsafe: {state_path}")
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if (state.get("server") != str(server)
                or state.get("gameVersion") != SUPPORTED_GAME_VERSION
                or state.get("forgeVersion") != SUPPORTED_FORGE_VERSION):
            parser.error("proof state does not match this Forge server")
        probe_state = state.get("probe", {})
        if probe_state.get("path") != PROBE_NAME:
            parser.error("proof state contains an unexpected probe filename")
        probe = mods / PROBE_NAME
        if probe.is_symlink() or not probe.is_file():
            parser.error(f"probe file is missing or unsafe: {probe}")
        if hashlib.sha256(probe.read_bytes()).hexdigest() != probe_state.get("sha256"):
            parser.error("probe jar changed since installation; refusing to remove it")
        probe.unlink()
        state_path.unlink()
    except (OSError, RuntimeError, ValueError, TypeError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print("Removed the Forge player-feed probe; all pre-existing server files were left in place.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
