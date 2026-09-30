#!/usr/bin/env python3
"""Install a temporary datapack player-feed probe into one stopped vanilla 26.3 server."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

from install_java_player_feed import running_server_pids


SOURCE = Path(__file__).resolve().parent / "vanilla-player-feed"
PACK_NAME = "msc-map-player-feed-proof.zip"
STATE_NAME = ".msc-map-vanilla-player-feed-proof.json"


def server_layout(server: Path) -> tuple[Path, Path]:
    properties = server / "server.properties"
    jar_path = server / "vanilla.jar"
    if not properties.is_file() or not jar_path.is_file():
        raise RuntimeError("server directory must contain server.properties and vanilla.jar")
    try:
        with ZipFile(jar_path) as jar:
            version = json.loads(jar.read("version.json"))
            manifest = jar.read("META-INF/MANIFEST.MF").decode("utf-8")
            members = set(jar.namelist())
    except (OSError, KeyError, json.JSONDecodeError) as error:
        raise RuntimeError(f"could not read Minecraft version from vanilla.jar: {error}") from error
    if version.get("id") != "26.3":
        raise RuntimeError("this Vanilla datapack probe is built for Minecraft 26.3 only")
    if ("Main-Class: net.minecraft.bundler.Main" not in manifest
            or "META-INF/patches.list" in members):
        raise RuntimeError("selected jar does not look like the official vanilla server bundle")
    metadata = json.loads((SOURCE / "pack.mcmeta").read_text(encoding="utf-8"))
    declared_pack = metadata.get("pack", {})
    data_format = version.get("pack_version", {}).get("data_major")
    if (declared_pack.get("min_format") != data_format
            or declared_pack.get("max_format") != data_format):
        raise RuntimeError("datapack format does not match the selected Vanilla jar")
    level_name = None
    for line in properties.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith(("#", "!")) and "=" in stripped:
            key, value = stripped.split("=", 1)
            if key.strip() == "level-name":
                level_name = value.strip()
                break
    if not level_name:
        raise RuntimeError("server.properties has no level-name")
    relative = Path(level_name)
    if relative.is_absolute() or len(relative.parts) != 1 or relative.name in (".", ".."):
        raise RuntimeError("level-name must be a single directory name inside the server")
    world = server / relative
    if world.is_symlink():
        raise RuntimeError("refusing to install through a symlinked world directory")
    if not world.is_dir():
        raise RuntimeError("start Vanilla once so its selected world folder exists, then stop it")
    world = world.resolve(strict=True)
    if not world.is_relative_to(server):
        raise RuntimeError("selected world directory must be inside the Vanilla server")
    datapacks = world / "datapacks"
    if datapacks.is_symlink():
        raise RuntimeError("refusing to install through a symlinked datapacks directory")
    if datapacks.exists() and not datapacks.is_dir():
        raise RuntimeError("world datapacks path exists but is not a directory")
    return world, datapacks


def build_pack() -> bytes:
    metadata = SOURCE / "pack.mcmeta"
    if not metadata.is_file():
        raise RuntimeError("Vanilla datapack source is incomplete")
    from io import BytesIO
    archive = BytesIO()
    with ZipFile(archive, "w") as pack:
        for path in sorted(SOURCE.rglob("*")):
            if path.is_symlink():
                raise RuntimeError(f"refusing to include symlinked datapack source: {path}")
            if not path.is_file():
                continue
            name = path.relative_to(SOURCE).as_posix()
            entry = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_DEFLATED
            entry.external_attr = 0o644 << 16
            pack.writestr(entry, path.read_bytes())
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
        parser.error("stop Vanilla in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        world, datapacks = server_layout(server)
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Vanilla server process is still running from this folder (PID(s): {pids})")
        pack = datapacks / PACK_NAME
        state_path = server / STATE_NAME
        if os.path.lexists(pack) or os.path.lexists(state_path):
            parser.error("a Vanilla player-feed proof install or state file already exists")
        contents = build_pack()
        datapacks_existed = datapacks.exists()
        datapacks.mkdir(parents=True, exist_ok=True)
        try:
            atomic_write(pack, contents, ".msc-vanilla-players-")
            state = {"server": str(server), "world": str(world),
                     "pack": str(pack.relative_to(server)),
                     "sha256": hashlib.sha256(contents).hexdigest()}
            atomic_write(state_path, (json.dumps(state, indent=2) + "\n").encode(),
                         ".msc-vanilla-state-")
        except Exception:
            pack.unlink(missing_ok=True)
            state_path.unlink(missing_ok=True)
            if not datapacks_existed:
                datapacks.rmdir()
            raise
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Installed {pack}; no server configuration or world files were replaced.")
    print("Watch with tools/world-map-proof/watch_vanilla_player_feed.py --server <MSC server name>.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
