#!/usr/bin/env python3
"""Remove only the unchanged KubeJS player-feed probe from a stopped server."""

import argparse
from pathlib import Path
import shutil
import subprocess
import sys

from install_java_player_feed import DESTINATION, SOURCE, running_server_pids


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument(
        "--server-stopped", action="store_true",
        help="confirm the server is stopped in MSC before changing its scripts",
    )
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop the Java server in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        if not (server / "server.properties").is_file():
            parser.error("server directory must contain server.properties")
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Java server process is still running from this folder (PID(s): {pids})")
        destination = server / DESTINATION
        if not destination.is_file():
            parser.error(f"probe file is not installed: {destination}")
        if destination.read_bytes() != SOURCE.read_bytes():
            parser.error("probe file changed since installation; refusing to remove it")
        destination.unlink()
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Removed {destination}; all other server files were left in place.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
