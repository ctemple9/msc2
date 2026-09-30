#!/usr/bin/env python3
"""Build and install the temporary player-feed mod into one stopped Forge 26.3 server."""

import argparse
from io import BytesIO
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

from install_java_player_feed import running_server_pids


SOURCE = Path(__file__).resolve().parent / "forge-player-feed"
MODS_PATH = Path("mods")
PROBE_NAME = "msc-map-player-feed-proof-forge.jar"
STATE_NAME = ".msc-map-player-feed-proof-forge.json"
CLASS_NAME = "dev/msc/mapproof/ForgePlayerFeedProof.class"
SUPPORTED_GAME_VERSION = "26.3"
SUPPORTED_FORGE_VERSION = "66.0.8"
def forge_layout(server: Path) -> tuple[Path, Path, Path]:
    if not (server / "server.properties").is_file():
        raise RuntimeError("server directory must contain server.properties")
    if not (server / "run.sh").is_file() or not (server / "forge-26.3-66.0.8-shim.jar").is_file():
        raise RuntimeError("selected server does not look like the managed Forge 26.3 / 66.0.8 instance")
    args_file = server / "libraries/net/minecraftforge/forge/26.3-66.0.8/unix_args.txt"
    if not args_file.is_file():
        raise RuntimeError(f"Forge server launch arguments are missing: {args_file}")
    args_text = args_file.read_text(encoding="utf-8", errors="replace")
    run_script = (server / "run.sh").read_text(encoding="utf-8", errors="replace")
    if ("forge-26.3-66.0.8-shim.jar" not in args_text
            or "libraries/net/minecraftforge/forge/26.3-66.0.8/unix_args.txt" not in run_script):
        raise RuntimeError("Forge launch arguments do not match the supported 26.3 / 66.0.8 build")
    mods = (server / MODS_PATH).resolve(strict=True)
    if not mods.is_dir() or not mods.is_relative_to(server):
        raise RuntimeError("Forge mods directory must exist inside the selected server")
    server_jar = server / "libraries/net/minecraft/server/26.3/server-26.3-unpacked.jar"
    forge_dir = server / "libraries/net/minecraftforge/forge/26.3-66.0.8"
    forge_api = forge_dir / "forge-26.3-66.0.8-universal.jar"
    if not server_jar.is_file() or not forge_api.is_file():
        raise RuntimeError("Forge 26.3 server or Forge API libraries are missing")
    return mods, server_jar, forge_api


def build_jar(server: Path, game_jar: Path, forge_api: Path) -> bytes:
    javac = shutil.which("javac")
    if not javac:
        raise RuntimeError("JDK 25 javac is required to build the Forge 26.3 probe")
    source = SOURCE / "src/main/java/dev/msc/mapproof/ForgePlayerFeedProof.java"
    metadata = SOURCE / "src/main/resources/META-INF/mods.toml.in"
    if not source.is_file() or not metadata.is_file():
        raise RuntimeError("Forge player-feed source is incomplete")

    libraries = sorted((server / "libraries").rglob("*.jar"))
    classpath = os.pathsep.join(map(str, [game_jar, forge_api, *libraries]))
    with tempfile.TemporaryDirectory(prefix="msc-forge-player-feed-") as temporary:
        result = subprocess.run(
            [javac, "--release", "25", "-g:none", "-encoding", "UTF-8",
             "-classpath", classpath, "-d", temporary, str(source)],
            capture_output=True, text=True, check=False,
        )
        if result.returncode:
            raise RuntimeError(f"Forge probe compilation failed:\n{result.stderr.strip()}")
        bytecode = (Path(temporary) / CLASS_NAME).read_bytes()

    archive = BytesIO()
    with ZipFile(archive, "w") as jar:
        for name, contents in (("META-INF/mods.toml", metadata.read_bytes()),
                               (CLASS_NAME, bytecode)):
            entry = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_DEFLATED
            entry.external_attr = 0o644 << 16
            jar.writestr(entry, contents)
    return archive.getvalue()


def atomic_write(destination: Path, contents: bytes, prefix: str) -> None:
    temporary_name = None
    try:
        with tempfile.NamedTemporaryFile(mode="wb", dir=destination.parent,
                                         prefix=prefix, delete=False) as temporary:
            temporary_name = Path(temporary.name)
            temporary.write(contents)
            temporary.flush()
            os.fsync(temporary.fileno())
        temporary_name.replace(destination)
    finally:
        if temporary_name is not None:
            temporary_name.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop the Forge server in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        mods, game_jar, forge_api = forge_layout(server)
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Forge server process is still running from this folder (PID(s): {pids})")
        probe = mods / PROBE_NAME
        state_path = mods / STATE_NAME
        if os.path.lexists(probe) or os.path.lexists(state_path):
            parser.error("a Forge player-feed proof install or state file already exists")
        probe_bytes = build_jar(server, game_jar, forge_api)
        state = {
            "server": str(server),
            "gameVersion": SUPPORTED_GAME_VERSION,
            "forgeVersion": SUPPORTED_FORGE_VERSION,
            "probe": {"path": PROBE_NAME, "sha256": hashlib.sha256(probe_bytes).hexdigest()},
        }
        try:
            atomic_write(probe, probe_bytes, ".msc-map-players-")
            atomic_write(state_path, (json.dumps(state, indent=2) + "\n").encode(), ".msc-forge-state-")
        except Exception:
            probe.unlink(missing_ok=True)
            state_path.unlink(missing_ok=True)
            raise
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Installed {probe}; no existing Forge mod or world files were replaced.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
