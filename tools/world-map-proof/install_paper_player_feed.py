#!/usr/bin/env python3
"""Build and install the isolated player-feed plugin into stopped Paper 26.2."""

import argparse
from io import BytesIO
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

from install_java_player_feed import running_server_pids


SOURCE = Path(__file__).resolve().parent / "paper-player-feed"
DESTINATION = "msc-map-player-feed-proof.jar"
API_VERSION = "26.2.build.121-stable"
CLASS_NAME = "dev/msc/mapproof/PaperPlayerFeedProof.class"


def paper_api_jar(server: Path) -> Path:
    version_file = server / ".msc_paper_version.json"
    version = json.loads(version_file.read_text(encoding="utf-8"))
    if version.get("mcVersion") != "26.2" or version.get("build") != 121:
        raise RuntimeError("this proof is compiled for Paper 26.2 build 121 only")
    api = (server / "libraries/io/papermc/paper/paper-api" / API_VERSION
           / f"paper-api-{API_VERSION}.jar")
    if not api.is_file():
        raise RuntimeError(f"installed Paper API jar is missing: {api}")
    return api


def build_jar(server: Path) -> bytes:
    api = paper_api_jar(server)
    source = SOURCE / "src/dev/msc/mapproof/PaperPlayerFeedProof.java"
    plugin_yaml = SOURCE / "plugin.yml"
    if not source.is_file() or not plugin_yaml.is_file():
        raise RuntimeError("Paper player-feed source is incomplete")
    javac = shutil.which("javac")
    if not javac:
        raise RuntimeError("javac is required to build the Paper 26.2 probe")
    # Paper's API inherits Adventure types; use the already-installed runtime
    # libraries as javac's compile classpath without fetching dependencies.
    libraries = sorted((server / "libraries").rglob("*.jar"))
    classpath = os.pathsep.join(str(path) for path in [api, *libraries])

    with tempfile.TemporaryDirectory(prefix="msc-paper-player-feed-") as temporary:
        result = subprocess.run(
            [javac, "--release", "25", "-g:none", "-encoding", "UTF-8",
             "-classpath", classpath, "-d", temporary, str(source)],
            capture_output=True, text=True, check=False,
        )
        if result.returncode:
            raise RuntimeError(f"Paper probe compilation failed:\n{result.stderr.strip()}")
        bytecode = (Path(temporary) / CLASS_NAME).read_bytes()

    archive = BytesIO()
    with ZipFile(archive, "w") as jar:
        for name, contents in (("plugin.yml", plugin_yaml.read_bytes()),
                               (CLASS_NAME, bytecode)):
            entry = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_DEFLATED
            entry.external_attr = 0o644 << 16
            jar.writestr(entry, contents)
    return archive.getvalue()


def stopped_paper_server(parser: argparse.ArgumentParser, server_dir: Path) -> tuple[Path, Path]:
    server = server_dir.resolve(strict=True)
    if not (server / "server.properties").is_file() or not (server / "paper.jar").is_file():
        parser.error("server directory must contain Paper and server.properties")
    plugins = (server / "plugins").resolve(strict=True)
    if not plugins.is_dir() or not plugins.is_relative_to(server):
        parser.error("Paper plugins directory must be inside the selected server")
    pids = running_server_pids(server)
    if pids:
        parser.error(f"Paper server is still running from this folder (PID(s): {pids})")
    return server, plugins


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

    print(f"Installed {destination}; no existing Paper files were replaced.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
