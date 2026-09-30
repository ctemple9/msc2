#!/usr/bin/env python3
"""Remove only the unchanged Fabric player-feed proof files."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from install_fabric_player_feed import (
    PROBE_NAME,
    STATE_NAME,
    running_server_pids,
    server_layout,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop the Fabric server in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        mods, _ = server_layout(server)
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Fabric server process is still running from this folder (PID(s): {pids})")
        state_path = mods / STATE_NAME
        if state_path.is_symlink() or not state_path.is_file():
            parser.error(f"proof install state is missing: {state_path}")
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if state.get("server") != str(server):
            parser.error("proof state belongs to a different server directory")
        probe = mods / PROBE_NAME
        if probe.is_symlink() or not probe.is_file():
            parser.error(f"probe file is missing or unsafe: {probe}")
        if hashlib.sha256(probe.read_bytes()).hexdigest() != state.get("probe", {}).get("sha256"):
            parser.error("probe jar changed since installation; refusing to remove it")
        api_state = state.get("fabricApi")
        api = None
        if api_state is not None:
            api = mods / api_state.get("path", "")
            if api.parent != mods or api.name in ("", ".", ".."):
                parser.error("proof state contains an invalid Fabric API path")
            if api.is_symlink() or not api.is_file():
                parser.error(f"Fabric API installed by this proof is missing or unsafe: {api}")
            if hashlib.sha256(api.read_bytes()).hexdigest() != api_state.get("sha256"):
                parser.error("Fabric API jar changed since installation; refusing to remove it")
        probe.unlink()
        if api is not None:
            api.unlink()
        state_path.unlink()
    except (OSError, RuntimeError, ValueError, TypeError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    suffix = " and its Fabric API dependency" if api else ""
    print(f"Removed the Fabric player-feed probe{suffix}.")
    print("All pre-existing server files and mods were left in place.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
