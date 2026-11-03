#!/usr/bin/env python3
"""Opt-in Intel Mac acceptance: real VM writes, restart, relay readiness, deletion.

Requires the installed helper and a STOPPED, fully provisioned source Bedrock
server. Copies only runtime files into a disposable sibling, never its worlds.
No downloads, sudo, application reset, or changes to the source server. On
failure the disposable directory is retained for diagnosis. Expected 1–3 minutes.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import socket
import tempfile
import time


def run(source: Path) -> None:
    if platform.system() != "Darwin" or platform.machine() != "x86_64" or os.geteuid() == 0:
        raise RuntimeError("Run as the installing user on an Intel Mac, not with sudo")
    if not (source / "bedrock_server").is_file():
        raise RuntimeError("Source must contain the installed Bedrock runtime")
    target = Path(tempfile.mkdtemp(prefix="msc2-ownership-check-", dir=source.parent))
    print(f"Disposable server: {target}", flush=True)
    for entry in source.iterdir():
        if entry.is_symlink():
            continue
        if entry.is_file() and (entry.name == "bedrock_server" or ".so" in entry.name):
            shutil.copy2(entry, target / entry.name)
        elif entry.name in ("behavior_packs", "resource_packs", "definitions") and entry.is_dir():
            shutil.copytree(entry, target / entry.name, symlinks=True)
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    (target / "server.properties").write_text(
        f"level-name=Ownership regression\nserver-port={port}\nserver-portv6={port + 1 if port < 65535 else port - 1}\nonline-mode=false\ntransport=raknet\n"
    )
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.settimeout(180)
        connection.connect("/var/run/msc2/bedrock.sock")
        stream = connection.makefile("rwb", buffering=0)

        def send(message):
            stream.write(json.dumps(message).encode() + b"\n")

        def expect(kind):
            deadline = time.monotonic() + 180
            while time.monotonic() < deadline:
                connection.settimeout(max(0.1, deadline - time.monotonic()))
                line = stream.readline(65537)
                if not line or len(line) > 65536:
                    raise RuntimeError("Helper disconnected or exceeded its frame bound")
                message = json.loads(line)
                if message["type"] == kind:
                    return message
                if message["type"] == "terminated":
                    raise RuntimeError(f"Bedrock stopped before {kind}: {message}")
            raise TimeoutError(f"No {kind} within 180 seconds")

        for attempt in range(2):
            send({"type": "provision", "server_dir": str(target), "version": "ownership-acceptance"})
            if not expect("provisioned")["ok"]:
                raise RuntimeError("Helper refused the disposable server")
            send({"type": "start", "memory_gb": 2, "bedrock_port": port, "transport": "raknet"})
            if not expect("started")["accepted"]:
                raise RuntimeError("VM worker refused startup")
            if not expect("ready")["relay_up"]:
                raise RuntimeError("Privileged relay did not verify Bedrock connectivity")
            send({"type": "stop"})
            stopped = expect("terminated")
            if stopped["reason"] != "clean":
                raise RuntimeError(f"Unclean stop: {stopped}")
            database = target / "worlds" / "Ownership regression" / "db"
            if not database.is_dir() or not any(database.iterdir()):
                raise RuntimeError("The VM did not generate a real Bedrock database")
            for root, directories, files in os.walk(target, followlinks=False):
                for name in [".", *directories, *files]:
                    path = Path(root) / name
                    info = path.lstat()
                    if info.st_uid != os.getuid() or info.st_gid != os.getgid():
                        raise RuntimeError(f"Wrong host ownership {info.st_uid}:{info.st_gid}: {path}")
            print(f"Run {attempt + 1}: generated database, verified relay, clean stop and user ownership", flush=True)
        stream.close()
    shutil.rmtree(target)
    if target.exists():
        raise RuntimeError("User could not delete the disposable Bedrock server")
    print("PASS: VM writes and restart preserve ownership; server deletion succeeds")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-server", type=Path, required=True)
    args = parser.parse_args()
    run(args.source_server.expanduser().resolve(strict=True))
