#!/usr/bin/env python3
"""Install the isolated Fabric player-feed mod into one stopped 26.2 server."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from urllib.request import urlopen
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

from install_java_player_feed import running_server_pids


SOURCE = Path(__file__).resolve().parent / "fabric-player-feed"
PROBE_NAME = "msc-map-player-feed-proof.jar"
STATE_NAME = ".msc-map-player-feed-proof.json"
API_VERSION = "0.161.0+26.2"
API_MAVEN_PATH = "net/fabricmc/fabric-api/fabric-api/0.161.0%2B26.2"
API_NAME = f"fabric-api-{API_VERSION}.jar"
API_BASE_URL = f"https://maven.fabricmc.net/{API_MAVEN_PATH}/fabric-api-0.161.0%2B26.2.jar"
API_SHA256_URL = API_BASE_URL + ".sha256"
CLASS_NAME = "dev/msc/mapproof/FabricPlayerFeedProof.class"


def fetch(url: str) -> bytes:
    with urlopen(url, timeout=30) as response:
        return response.read()


def fetch_api() -> bytes:
    expected = fetch(API_SHA256_URL).decode("ascii").strip().split()[0].lower()
    if len(expected) != 64 or any(character not in "0123456789abcdef" for character in expected):
        raise RuntimeError("Fabric Maven returned an invalid SHA-256 sidecar")
    contents = fetch(API_BASE_URL)
    if hashlib.sha256(contents).hexdigest() != expected:
        raise RuntimeError("Fabric API download did not match the official Maven SHA-256")
    return contents


def server_layout(server: Path) -> tuple[Path, Path]:
    if not (server / "server.properties").is_file():
        raise RuntimeError("server directory must contain server.properties")
    if not (server / "fabric-server-launch.jar").is_file():
        raise RuntimeError("selected server does not look like a Fabric Loader server")
    mods = (server / "mods").resolve(strict=True)
    if not mods.is_dir() or not mods.is_relative_to(server):
        raise RuntimeError("Fabric mods directory must exist inside the selected server")
    loader_jars = list((server / "libraries/net/fabricmc/fabric-loader").glob("*/fabric-loader-*.jar"))
    game_jars = [server / "versions/26.2/server-26.2.jar", server / ".fabric/server/26.2-server.jar"]
    game_jar = next((path for path in game_jars if path.is_file()), None)
    if not loader_jars or game_jar is None:
        raise RuntimeError("the selected Fabric server must have Fabric Loader and Minecraft 26.2 installed")
    return mods, game_jar


def mod_id_from_jar(path: Path) -> tuple[str | None, str | None]:
    from zipfile import BadZipFile
    try:
        with ZipFile(path) as jar:
            metadata = json.loads(jar.read("fabric.mod.json"))
    except (BadZipFile, KeyError, json.JSONDecodeError, OSError):
        return None, None
    return metadata.get("id"), metadata.get("version")


def existing_fabric_api(mods: Path) -> Path | None:
    matches = []
    for path in mods.glob("*.jar"):
        mod_id, version = mod_id_from_jar(path)
        if mod_id == "fabric-api":
            if not isinstance(version, str) or "26.2" not in version:
                raise RuntimeError(f"existing Fabric API does not identify as a 26.2 build: {path.name}")
            matches.append(path)
    if len(matches) > 1:
        raise RuntimeError("multiple Fabric API jars are present; remove duplicates before installing the proof")
    return matches[0] if matches else None


def build_jar(game_jar: Path, loader_jar: Path, api_bytes: bytes) -> bytes:
    java = shutil.which("javac")
    if not java:
        raise RuntimeError("JDK 25 javac is required to build the Fabric 26.2 probe")
    source = SOURCE / "src/main/java/dev/msc/mapproof/FabricPlayerFeedProof.java"
    metadata = SOURCE / "fabric.mod.json"
    if not source.is_file() or not metadata.is_file():
        raise RuntimeError("Fabric player-feed source is incomplete")

    with tempfile.TemporaryDirectory(prefix="msc-fabric-player-feed-") as temporary:
        root = Path(temporary)
        api_path = root / "fabric-api.jar"
        api_path.write_bytes(api_bytes)
        nested_dir = root / "nested-api"
        nested_dir.mkdir()
        with ZipFile(api_path) as aggregate:
            nested_names = [name for name in aggregate.namelist()
                            if name.startswith("META-INF/jars/") and name.endswith(".jar")]
            for name in nested_names:
                (nested_dir / Path(name).name).write_bytes(aggregate.read(name))
        server_root = next((parent for parent in game_jar.parents if (parent / "server.properties").is_file()), None)
        if server_root is None:
            raise RuntimeError("could not locate the selected Fabric server root")
        libraries = sorted((server_root / "libraries").rglob("*.jar"))
        classpath = os.pathsep.join(map(str, [game_jar, loader_jar, *libraries,
                                               *sorted(nested_dir.glob("*.jar"))]))
        classes = root / "classes"
        classes.mkdir()
        result = subprocess.run(
            [java, "--release", "25", "-g:none", "-encoding", "UTF-8",
             "-classpath", classpath, "-d", str(classes), str(source)],
            capture_output=True, text=True, check=False,
        )
        if result.returncode:
            raise RuntimeError(f"Fabric probe compilation failed:\n{result.stderr.strip()}")
        bytecode = (classes / CLASS_NAME).read_bytes()

    from io import BytesIO
    archive = BytesIO()
    with ZipFile(archive, "w") as jar:
        for name, contents in (("fabric.mod.json", metadata.read_bytes()), (CLASS_NAME, bytecode)):
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
        parser.error("stop the Fabric server in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        mods, game_jar = server_layout(server)
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Fabric server process is still running from this folder (PID(s): {pids})")
        probe = mods / PROBE_NAME
        state_path = mods / STATE_NAME
        if os.path.lexists(probe) or os.path.lexists(state_path):
            parser.error("a Fabric player-feed proof install or state file already exists")
        loader_jar = next((server / "libraries/net/fabricmc/fabric-loader").glob("*/fabric-loader-*.jar"))
        api_existing = existing_fabric_api(mods)
        api_bytes = fetch_api()
        api_destination = mods / API_NAME if api_existing is None else None
        if api_destination is not None and os.path.lexists(api_destination):
            parser.error(f"refusing to overwrite existing file: {api_destination}")
        probe_bytes = build_jar(game_jar, loader_jar, api_bytes)
        try:
            if api_destination is not None:
                atomic_write(api_destination, api_bytes, ".msc-fabric-api-")
            atomic_write(probe, probe_bytes, ".msc-map-players-")
            state = {
                "server": str(server),
                "probe": {"path": PROBE_NAME, "sha256": hashlib.sha256(probe_bytes).hexdigest()},
                "fabricApi": ({"path": API_NAME, "sha256": hashlib.sha256(api_bytes).hexdigest()}
                              if api_destination is not None else None),
            }
            atomic_write(state_path, (json.dumps(state, indent=2) + "\n").encode(), ".msc-fabric-state-")
        except Exception:
            probe.unlink(missing_ok=True)
            if api_destination is not None:
                api_destination.unlink(missing_ok=True)
            state_path.unlink(missing_ok=True)
            raise
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Installed {probe}; Fabric API {'was installed for the proof' if api_destination else 'was already present'}.")
    print("No world, server configuration, or existing mod file was changed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
