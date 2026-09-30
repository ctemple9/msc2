#!/usr/bin/env python3
"""Remove only the unchanged Vanilla datapack player-feed proof."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from install_java_player_feed import running_server_pids
from install_vanilla_player_feed import (
    PACK_NAME, STATE_NAME, build_pack, server_layout,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop Vanilla in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        world, datapacks = server_layout(server)
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Vanilla server process is still running from this folder (PID(s): {pids})")
        state_path = server / STATE_NAME
        if state_path.is_symlink() or not state_path.is_file():
            parser.error(f"proof install state is missing: {state_path}")
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if state.get("server") != str(server) or state.get("world") != str(world):
            parser.error("proof state belongs to a different server or world")
        pack = datapacks / PACK_NAME
        if state.get("pack") != str(pack.relative_to(server)):
            parser.error("proof state does not name the expected datapack")
        if pack.is_symlink() or not pack.is_file():
            parser.error(f"proof datapack is missing or unsafe: {pack}")
        digest = hashlib.sha256(pack.read_bytes()).hexdigest()
        expected_digest = hashlib.sha256(build_pack()).hexdigest()
        if digest != state.get("sha256") or digest != expected_digest:
            parser.error("datapack changed since installation; refusing to remove it")
        pack.unlink()
        state_path.unlink()
    except (OSError, RuntimeError, ValueError, TypeError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Removed {pack}; no other world or server files were changed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
