#!/usr/bin/env python3
"""Remove only the unchanged Purpur player-feed plugin from a stopped server."""

import argparse
import hashlib
from pathlib import Path
import subprocess

from install_purpur_player_feed import DESTINATION, build_jar, stopped_purpur_server


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop Purpur in MSC, then pass --server-stopped")

    try:
        server, plugins = stopped_purpur_server(parser, args.server_dir)
        destination = plugins / DESTINATION
        if destination.is_symlink() or not destination.is_file():
            parser.error(f"probe file is not installed: {destination}")
        expected_digest = hashlib.sha256(build_jar(server)).digest()
        actual_digest = hashlib.sha256(destination.read_bytes()).digest()
        if actual_digest != expected_digest:
            parser.error("probe jar changed since installation; refusing to remove it")
        destination.unlink()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Removed {destination}; all other Purpur files were left in place.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
