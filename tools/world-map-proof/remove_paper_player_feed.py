#!/usr/bin/env python3
"""Remove only the unchanged Paper player-feed plugin from a stopped server."""

import argparse
from pathlib import Path
import subprocess

from install_paper_player_feed import DESTINATION, build_jar, stopped_paper_server


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop Paper in MSC, then pass --server-stopped")

    try:
        server, plugins = stopped_paper_server(parser, args.server_dir)
        destination = plugins / DESTINATION
        if destination.is_symlink() or not destination.is_file():
            parser.error(f"probe file is not installed: {destination}")
        if destination.read_bytes() != build_jar(server):
            parser.error("probe jar changed since installation; refusing to remove it")
        destination.unlink()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Removed {destination}; all other Paper files were left in place.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
