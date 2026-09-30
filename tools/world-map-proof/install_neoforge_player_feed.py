#!/usr/bin/env python3
"""Build and install the isolated player-feed mod in one stopped NeoForge 26.2 server."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

from install_java_player_feed import running_server_pids


SOURCE = (Path(__file__).resolve().parent / "neoforge-player-feed" / "src" /
          "dev/msc/mapproof/NeoForgePlayerFeedProof.java")
MOD_ID = "msc_map_player_feed_proof"
MOD_NAME = "msc-map-player-feed-proof.jar"
STATE_NAME = ".msc-map-player-feed-proof.json"
CLASS_NAME = "dev/msc/mapproof/NeoForgePlayerFeedProof.class"
SUPPORTED_MC = "26.2"


def detect_runtime(server: Path) -> tuple[str, str, Path, Path]:
    run_script = server / "run.sh"
    if not run_script.is_file():
        raise RuntimeError("server folder has no NeoForge run.sh")
    script = run_script.read_text(encoding="utf-8", errors="replace")
    match = re.search(r"@libraries/net/neoforged/neoforge/([^/]+)/unix_args\.txt", script)
    if not match:
        raise RuntimeError("could not identify the installed NeoForge version from run.sh")
    neo_version = match.group(1)
    args_file = server / "libraries/net/neoforged/neoforge" / neo_version / "unix_args.txt"
    if not args_file.is_file():
        raise RuntimeError(f"NeoForge runtime arguments are missing: {args_file}")
    args_text = args_file.read_text(encoding="utf-8", errors="replace")
    mc_match = re.search(r"--fml\.mcVersion\s+([^\s]+)", args_text)
    if not mc_match:
        raise RuntimeError("could not identify the Minecraft version from NeoForge unix_args.txt")
    mc_version = mc_match.group(1)
    if mc_version != SUPPORTED_MC:
        raise RuntimeError(
            f"this proof mod is compiled for Minecraft {SUPPORTED_MC}; selected server is "
            f"Minecraft {mc_version} / NeoForge {neo_version}"
        )
    neo_jar = server / "libraries/net/neoforged/neoforge" / neo_version / f"neoforge-{neo_version}-universal.jar"
    game_jar = server / "libraries/net/neoforged/minecraft-server-patched" / neo_version / f"minecraft-server-patched-{neo_version}.jar"
    if not neo_jar.is_file() or not game_jar.is_file():
        raise RuntimeError("NeoForge or patched Minecraft compile jars are missing from this server")
    return mc_version, neo_version, neo_jar, game_jar


def compile_probe(server: Path, mc_version: str, neo_version: str,
                  neo_jar: Path, game_jar: Path) -> bytes:
    java = shutil.which("javac")
    if not java:
        raise RuntimeError("a JDK 25 javac is required to build the Minecraft 26.2 probe")
    if not SOURCE.is_file():
        raise RuntimeError(f"probe source is missing: {SOURCE}")
    libraries = sorted((server / "libraries").rglob("*.jar"))
    classpath = os.pathsep.join(map(str, [game_jar, neo_jar, *libraries]))
    with tempfile.TemporaryDirectory(prefix="msc-neoforge-player-feed-") as temporary:
        root = Path(temporary)
        classes = root / "classes"
        classes.mkdir()
        result = subprocess.run(
            [java, "--release", "25", "-g:none", "-encoding", "UTF-8",
             "-classpath", classpath, "-d", str(classes), str(SOURCE)],
            capture_output=True, text=True, check=False,
        )
        if result.returncode:
            raise RuntimeError(f"NeoForge probe compilation failed:\n{result.stderr.strip()}")
        class_bytes = (classes / CLASS_NAME).read_bytes()

    metadata = f'''modLoader="javafml"
loaderVersion="[3,)"
license="All Rights Reserved"

[[mods]]
modId="{MOD_ID}"
version="1.0.0"
displayName="MSC Map Player Feed Proof"
description="Temporary server-side player position sampling for the MSC map proof."

[[dependencies.{MOD_ID}]]
modId="neoforge"
type="required"
versionRange="[{neo_version},)"
ordering="NONE"
side="SERVER"

[[dependencies.{MOD_ID}]]
modId="minecraft"
type="required"
versionRange="[{mc_version},{mc_version.rsplit('.', 1)[0]}.{int(mc_version.rsplit('.', 1)[1]) + 1})"
ordering="NONE"
side="SERVER"
'''
    from io import BytesIO
    archive = BytesIO()
    with ZipFile(archive, "w") as jar:
        for name, contents in (("META-INF/neoforge.mods.toml", metadata.encode("utf-8")),
                               (CLASS_NAME, class_bytes)):
            entry = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_DEFLATED
            entry.external_attr = 0o644 << 16
            jar.writestr(entry, contents)
    return archive.getvalue()


def atomic_write(destination: Path, contents: bytes) -> None:
    temporary_name = None
    try:
        with tempfile.NamedTemporaryFile(mode="wb", dir=destination.parent,
                                         prefix=".msc-neoforge-player-feed-", delete=False) as temporary:
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
        destination = mods / MOD_NAME
        state_path = mods / STATE_NAME
        if os.path.lexists(destination) or os.path.lexists(state_path):
            raise RuntimeError("a NeoForge player-feed proof install or state file already exists")
        mc_version, neo_version, neo_jar, game_jar = detect_runtime(server)
        contents = compile_probe(server, mc_version, neo_version, neo_jar, game_jar)
        try:
            atomic_write(destination, contents)
            state = {
                "server": str(server),
                "minecraftVersion": mc_version,
                "neoForgeVersion": neo_version,
                "probe": {"path": MOD_NAME, "sha256": hashlib.sha256(contents).hexdigest()},
            }
            atomic_write(state_path, (json.dumps(state, indent=2) + "\n").encode("utf-8"))
        except Exception:
            destination.unlink(missing_ok=True)
            state_path.unlink(missing_ok=True)
            raise
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Installed {destination} for Minecraft {mc_version} / NeoForge {neo_version}.")
    print("No world, server configuration, or pre-existing mod file was changed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
