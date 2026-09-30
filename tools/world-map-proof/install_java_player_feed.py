#!/usr/bin/env python3
"""Install the isolated KubeJS player-feed probe into one stopped Java server."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


SOURCE = Path(__file__).resolve().parent / "java-player-feed" / "scripts" / "server.js"
DESTINATION = Path("kubejs/server_scripts/msc_map_players_proof.js")


def java_process_working_directories() -> list[tuple[int, Path]]:
    ps = shutil.which("ps")
    lsof = shutil.which("lsof")
    if not ps or not lsof:
        raise RuntimeError("cannot verify server state: both ps and lsof are required")

    result = subprocess.run(
        [ps, "-axo", "pid=,command="], capture_output=True, text=True, check=True
    )
    processes: list[tuple[int, Path]] = []
    for line in result.stdout.splitlines():
        parts = line.strip().split(None, 1)
        if len(parts) != 2 or "java" not in parts[1].lower():
            continue
        try:
            pid = int(parts[0])
        except ValueError:
            continue
        cwd_result = subprocess.run(
            [lsof, "-a", "-p", str(pid), "-d", "cwd", "-Fn"],
            capture_output=True, text=True, check=False,
        )
        if cwd_result.returncode not in (0, 1):
            raise RuntimeError(f"cannot verify working directory for Java PID {pid}")
        for entry in cwd_result.stdout.splitlines():
            if entry.startswith("n"):
                processes.append((pid, Path(entry[1:]).resolve()))
                break
    return processes


def running_server_pids(server: Path) -> list[int]:
    return [
        pid for pid, cwd in java_process_working_directories()
        if cwd == server or cwd.is_relative_to(server)
    ]


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
        scripts = (server / "kubejs/server_scripts").resolve(strict=True)
        if not scripts.is_dir() or not scripts.is_relative_to(server):
            parser.error("KubeJS server_scripts must exist inside the selected server")
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Java server process is still running from this folder (PID(s): {pids})")

        destination = server / DESTINATION
        if destination.exists():
            parser.error(f"refusing to overwrite existing file: {destination}")
        if not SOURCE.is_file():
            parser.error(f"probe source is missing: {SOURCE}")

        temporary_name = None
        try:
            with tempfile.NamedTemporaryFile(
                mode="wb", dir=scripts, prefix=".msc-map-players-", delete=False
            ) as temporary:
                temporary_name = Path(temporary.name)
                temporary.write(SOURCE.read_bytes())
                temporary.flush()
                os.fsync(temporary.fileno())
            temporary_name.replace(destination)
        finally:
            if temporary_name is not None:
                temporary_name.unlink(missing_ok=True)
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        parser.error(str(error))

    print(f"Installed {destination}; no existing server files were replaced.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
