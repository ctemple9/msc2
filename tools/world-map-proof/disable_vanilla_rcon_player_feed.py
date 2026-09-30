#!/usr/bin/env python3
"""Restore Vanilla server properties after its loopback RCON proof."""

import argparse
import json
import os
from pathlib import Path
import subprocess

from enable_vanilla_rcon_player_feed import KEYS, STATE_NAME, atomic_write, read_properties, replace_properties
from install_java_player_feed import running_server_pids


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop Vanilla in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Vanilla server process is still running (PID(s): {pids})")
        state_path = server / STATE_NAME
        if state_path.is_symlink() or not state_path.is_file():
            parser.error(f"proof state is missing or unsafe: {state_path}")
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if state.get("server") != str(server):
            parser.error("proof state belongs to a different server")
        properties = server / "server.properties"
        if properties.is_symlink() or not properties.is_file():
            parser.error("server.properties is missing or unsafe")
        lines, current = read_properties(properties)
        installed = state.get("installed")
        prior = state.get("prior")
        if not isinstance(installed, dict) or not isinstance(prior, dict):
            parser.error("proof state is malformed")
        restore: dict[str, str | None] = {}
        for key in KEYS:
            if current.get(key) != installed.get(key):
                parser.error(f"{key} changed since installation; refusing to overwrite it")
            saved = prior.get(key)
            if not isinstance(saved, dict) or not isinstance(saved.get("present"), bool):
                parser.error("proof state is malformed")
            restore[key] = str(saved.get("value")) if saved["present"] else None
        atomic_write(properties, replace_properties(lines, restore).encode("utf-8"),
                     properties.stat().st_mode & 0o777)
        state_path.unlink()
    except (OSError, RuntimeError, ValueError, TypeError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print("Restored the original RCON and server bind settings; no other server settings were changed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
