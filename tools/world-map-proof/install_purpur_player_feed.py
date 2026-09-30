#!/usr/bin/env python3
"""Build and install the isolated player-feed plugin into stopped Purpur."""

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


SOURCE = Path(__file__).resolve().parent / "purpur-player-feed"
DESTINATION = "msc-map-player-feed-proof.jar"
CLASS_NAME = "dev/msc/mapproof/PurpurPlayerFeedProof.class"
PURPUR_API_PREFIX = "META-INF/libraries/org/purpurmc/purpur/purpur-api/"


def purpur_bundle(server: Path) -> tuple[Path, int, str, str]:
    jar_path = server / "purpur.jar"
    if not jar_path.is_file():
        raise RuntimeError("selected server has no purpur.jar")
    try:
        with ZipFile(jar_path) as bundle:
            version_info = json.loads(bundle.read("version.json"))
            minecraft_version = version_info.get("id")
            java_version = version_info.get("java_version")
            patches = bundle.read("META-INF/patches.list").decode("utf-8")
            api_paths = [name for name in bundle.namelist()
                         if name.startswith(PURPUR_API_PREFIX) and name.endswith(".jar")]
    except (KeyError, OSError, ValueError) as error:
        raise RuntimeError(f"cannot identify the selected Purpur bundle: {error}") from error

    if not isinstance(minecraft_version, str) or not isinstance(java_version, int):
        raise RuntimeError("Purpur bundle does not declare a Minecraft and Java version")
    if not any("/purpur-" in line for line in patches.splitlines()):
        raise RuntimeError("server jar does not contain Purpur patch metadata")
    matching_apis = []
    for name in api_paths:
        relative = name.removeprefix(PURPUR_API_PREFIX)
        version_dir, _, artifact = relative.partition("/")
        if (artifact == f"purpur-api-{version_dir}.jar"
                and (version_dir == f"{minecraft_version}-R0.1-SNAPSHOT"
                     or version_dir.startswith(f"{minecraft_version}.build."))):
            matching_apis.append(name)
    if len(matching_apis) != 1:
        raise RuntimeError(
            f"expected one bundled Purpur API for Minecraft {minecraft_version}; "
            f"found {len(matching_apis)}"
        )
    if len(api_paths) != 1:
        raise RuntimeError(f"expected one bundled Purpur API; found {len(api_paths)}")
    return jar_path, java_version, minecraft_version, matching_apis[0]


def build_jar(server: Path) -> bytes:
    server_jar, java_version, minecraft_version, purpur_api = purpur_bundle(server)
    source = SOURCE / "src/dev/msc/mapproof/PurpurPlayerFeedProof.java"
    plugin_yaml = SOURCE / "plugin.yml"
    if not source.is_file() or not plugin_yaml.is_file():
        raise RuntimeError("Purpur player-feed source is incomplete")
    parts = minecraft_version.split(".")
    if len(parts) < 2 or not all(part.isdecimal() for part in parts[:2]):
        raise RuntimeError(f"unsupported Minecraft version format: {minecraft_version}")
    api_version = ".".join(parts[:2])
    descriptor = plugin_yaml.read_text(encoding="utf-8").replace(
        "@API_VERSION@", api_version
    )
    if "@API_VERSION@" in descriptor:
        raise RuntimeError("plugin descriptor is missing its API version placeholder")
    javac = shutil.which("javac")
    if not javac:
        raise RuntimeError("javac is required to build the Purpur player-feed probe")

    with tempfile.TemporaryDirectory(prefix="msc-purpur-player-feed-") as temporary:
        temporary_path = Path(temporary)
        library_dir = temporary_path / "libraries"
        library_dir.mkdir()
        classpath_entries: list[str] = []
        with ZipFile(server_jar) as bundle:
            for index, entry in enumerate(bundle.infolist()):
                if (not entry.filename.startswith("META-INF/libraries/")
                        or not entry.filename.endswith(".jar")
                        or entry.filename == purpur_api):
                    continue
                extracted = library_dir / f"{index}-{Path(entry.filename).name}"
                extracted.write_bytes(bundle.read(entry))
                classpath_entries.append(str(extracted))
        api_copy = library_dir / "purpur-api.jar"
        with ZipFile(server_jar) as bundle:
            api_copy.write_bytes(bundle.read(purpur_api))
        classpath_entries.append(str(api_copy))

        result = subprocess.run(
            [javac, "--release", str(java_version), "-g:none", "-encoding", "UTF-8",
             "-classpath", os.pathsep.join(classpath_entries), "-d", temporary,
             str(source)],
            capture_output=True, text=True, check=False,
        )
        if result.returncode:
            raise RuntimeError(f"Purpur probe compilation failed:\n{result.stderr.strip()}")
        bytecode = (temporary_path / CLASS_NAME).read_bytes()

    archive = BytesIO()
    with ZipFile(archive, "w") as jar:
        for name, contents in (("plugin.yml", descriptor.encode("utf-8")),
                               (CLASS_NAME, bytecode)):
            entry = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_DEFLATED
            entry.external_attr = 0o644 << 16
            jar.writestr(entry, contents)
    return archive.getvalue()


def stopped_purpur_server(parser: argparse.ArgumentParser,
                          server_dir: Path) -> tuple[Path, Path]:
    server = server_dir.resolve(strict=True)
    if not (server / "server.properties").is_file():
        parser.error("server directory must contain server.properties")
    purpur_bundle(server)
    plugins = (server / "plugins").resolve(strict=True)
    if not plugins.is_dir() or not plugins.is_relative_to(server):
        parser.error("Purpur plugins directory must be inside the selected server")
    pids = running_server_pids(server)
    if pids:
        parser.error(f"Purpur server is still running from this folder (PID(s): {pids})")
    return server, plugins


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
        if os.path.lexists(destination):
            parser.error(f"refusing to overwrite existing file: {destination}")
        jar = build_jar(server)
        temporary_name = None
        try:
            with tempfile.NamedTemporaryFile(
                mode="wb", dir=plugins, prefix=".msc-map-players-", delete=False
            ) as temporary:
                temporary_name = Path(temporary.name)
                temporary.write(jar)
                temporary.flush()
                os.fsync(temporary.fileno())
            temporary_name.replace(destination)
        finally:
            if temporary_name is not None:
                temporary_name.unlink(missing_ok=True)
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))

    digest = hashlib.sha256(destination.read_bytes()).hexdigest()
    print(f"Installed {destination}; SHA-256 {digest}; no existing Purpur files were replaced.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
