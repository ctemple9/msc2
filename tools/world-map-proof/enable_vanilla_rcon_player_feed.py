#!/usr/bin/env python3
"""Temporarily enable loopback-only RCON for a Vanilla player-feed proof."""

import argparse
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile

from install_java_player_feed import running_server_pids
from install_vanilla_player_feed import server_layout


STATE_NAME = ".msc-map-vanilla-rcon-proof.json"
KEYS = ("server-ip", "enable-rcon", "rcon.port", "rcon.password")


def read_properties(path: Path) -> tuple[list[str], dict[str, str]]:
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    found: dict[str, str] = {}
    for line in lines:
        body = line.rstrip("\r\n")
        stripped = body.lstrip()
        if not stripped or stripped.startswith(("#", "!")) or "=" not in body:
            continue
        key, value = body.split("=", 1)
        key = key.strip()
        if key in KEYS:
            if key in found:
                raise RuntimeError(f"server.properties contains duplicate {key} entries")
            found[key] = value.strip()
    return lines, found


def replace_properties(lines: list[str], values: dict[str, str | None]) -> str:
    seen: set[str] = set()
    out: list[str] = []
    for line in lines:
        body = line.rstrip("\r\n")
        stripped = body.lstrip()
        if stripped and not stripped.startswith(("#", "!")) and "=" in body:
            key = body.split("=", 1)[0].strip()
            if key in values:
                seen.add(key)
                value = values[key]
                if value is None:
                    continue
                ending = line[len(body):]
                out.append(f"{key}={value}{ending}")
                continue
        out.append(line)
    ending = "\r\n" if any(line.endswith("\r\n") for line in lines) else "\n"
    for key, value in values.items():
        if key not in seen and value is not None:
            out.append(f"{key}={value}{ending}")
    return "".join(out)


def atomic_write(path: Path, content: bytes, mode: int) -> None:
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile("wb", dir=path.parent,
                                         prefix=".msc-vanilla-rcon-", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, mode)
        temporary.replace(path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def unused_port() -> int:
    for _ in range(32):
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        if 40000 <= port <= 65000:
            return port
    raise RuntimeError("could not choose a temporary local RCON port")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop Vanilla in MSC, then pass --server-stopped")
    try:
        server = args.server_dir.resolve(strict=True)
        server_layout(server)
        properties = server / "server.properties"
        if properties.is_symlink() or not properties.is_file():
            raise RuntimeError("server.properties is missing or unsafe")
        pids = running_server_pids(server)
        if pids:
            parser.error(f"Vanilla server process is still running (PID(s): {pids})")
        state_path = server / STATE_NAME
        if os.path.lexists(state_path):
            parser.error("a Vanilla loopback RCON proof is already installed")
        lines, original = read_properties(properties)
        port = unused_port()
        password = secrets.token_hex(20)
        installed = {"server-ip": "127.0.0.1", "enable-rcon": "true",
                     "rcon.port": str(port), "rcon.password": password}
        prior = {key: {"present": key in original, "value": original.get(key)} for key in KEYS}
        state = {"server": str(server), "installed": installed, "prior": prior}
        atomic_write(state_path, (json.dumps(state, indent=2) + "\n").encode(), 0o600)
        try:
            content = replace_properties(lines, installed).encode("utf-8")
            atomic_write(properties, content, properties.stat().st_mode & 0o777)
        except Exception:
            atomic_write(properties, "".join(lines).encode("utf-8"),
                         properties.stat().st_mode & 0o777)
            state_path.unlink(missing_ok=True)
            raise
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        parser.error(str(error))
    print(f"Enabled temporary RCON on 127.0.0.1:{port}; the Java server is bound to loopback.")
    print(f"Secret stored in {state_path} with owner-only permissions. Restart Vanilla through MSC.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
