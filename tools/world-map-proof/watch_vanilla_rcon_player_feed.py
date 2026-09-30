#!/usr/bin/env python3
"""Poll official Vanilla player NBT over temporary loopback-only RCON."""

import argparse
import json
import math
from pathlib import Path
import re
import socket
import struct
import subprocess
import time

from enable_vanilla_rcon_player_feed import STATE_NAME
from watch_vanilla_player_feed import numeric_list, uuid_from_nbt


DIMENSIONS = ("minecraft:overworld", "minecraft:the_nether", "minecraft:the_end")
DATA_RE = re.compile(
    r"^(?:(.+?) has the following entity data:|Data for entity (.+?) is:|"
    r"The data of (.+?) is:)\s*(.+)$", re.I
)


def recv_exact(connection: socket.socket, count: int) -> bytes:
    chunks = bytearray()
    while len(chunks) < count:
        part = connection.recv(count - len(chunks))
        if not part:
            raise ConnectionError("RCON connection closed")
        chunks.extend(part)
    return bytes(chunks)


def recv_packet(connection: socket.socket) -> tuple[int, int, str]:
    length = struct.unpack("<i", recv_exact(connection, 4))[0]
    if length < 10 or length > 1_000_000:
        raise ValueError(f"invalid RCON packet length: {length}")
    body = recv_exact(connection, length)
    request_id, packet_type = struct.unpack("<ii", body[:8])
    payload = body[8:-2].decode("utf-8", errors="replace")
    return request_id, packet_type, payload


def send_packet(connection: socket.socket, request_id: int, packet_type: int,
                payload: str) -> None:
    body = struct.pack("<ii", request_id, packet_type) + payload.encode("utf-8") + b"\0\0"
    connection.sendall(struct.pack("<i", len(body)) + body)


class Rcon:
    def __init__(self, port: int, password: str):
        self.connection = socket.create_connection(("127.0.0.1", port), timeout=3)
        self.connection.settimeout(3)
        send_packet(self.connection, 1, 3, password)
        request_id, _, _ = recv_packet(self.connection)
        if request_id != 1:
            self.connection.close()
            raise PermissionError("Vanilla rejected the temporary RCON credential")
        self.request_id = 2

    def command(self, text: str) -> str:
        request_id = self.request_id
        self.request_id += 1
        send_packet(self.connection, request_id, 2, text)
        response_id, _, payload = recv_packet(self.connection)
        if response_id != request_id:
            raise ConnectionError("RCON response ID did not match the command")
        return payload

    def close(self) -> None:
        self.connection.close()


def records(text: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for raw_line in text.splitlines():
        line = raw_line.strip()
        match = DATA_RE.match(line)
        if match:
            name = next(value for value in match.groups()[:3] if value is not None).strip()
            result[name] = match.group(4).strip()
    return result


def collect(rcon: Rcon, sequence: int) -> dict:
    players: list[dict] = []
    for dimension in DIMENSIONS:
        fields: dict[str, dict[str, str]] = {}
        for field in ("UUID", "Pos", "Rotation"):
            output = rcon.command(
                f"execute as @a at @s if dimension {dimension} run data get entity @s {field}"
            )
            for name, value in records(output).items():
                fields.setdefault(name, {})[field] = value
        for name, record in fields.items():
            player_id = uuid_from_nbt(record.get("UUID", ""))
            pos = numeric_list(record.get("Pos", ""), 3)
            rotation = numeric_list(record.get("Rotation", ""), 2)
            if player_id is None or pos is None or rotation is None:
                continue
            if not all(math.isfinite(part) for part in (*pos, *rotation)):
                continue
            players.append({"id": player_id, "name": name, "dimension": dimension,
                            "x": pos[0], "y": pos[1], "z": pos[2],
                            "yaw": rotation[0], "pitch": rotation[1]})
    return {"sequence": sequence, "sampledAtMs": int(time.time() * 1000),
            "players": players}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--interval-seconds", type=float, default=1.0)
    args = parser.parse_args()
    if args.interval_seconds < 0.25:
        parser.error("interval must be at least 0.25 seconds")
    try:
        server = args.server_dir.resolve(strict=True)
        state_path = server / STATE_NAME
        if state_path.is_symlink() or not state_path.is_file():
            raise RuntimeError(f"temporary RCON state is missing or unsafe: {state_path}")
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if state.get("server") != str(server):
            raise RuntimeError("RCON state belongs to a different Vanilla server")
        installed = state.get("installed", {})
        port = int(installed["rcon.port"])
        password = str(installed["rcon.password"])
        if installed.get("server-ip") != "127.0.0.1" or installed.get("enable-rcon") != "true":
            raise RuntimeError("RCON state is not loopback-only")
        props = (server / "server.properties").read_text(encoding="utf-8")
        for key, value in installed.items():
            if not re.search(rf"(?m)^{re.escape(key)}={re.escape(value)}\s*$", props):
                raise RuntimeError(f"server.properties no longer matches the temporary {key} setting")
    except (OSError, KeyError, ValueError, TypeError, RuntimeError, subprocess.SubprocessError) as error:
        parser.error(str(error))

    print("Polling Vanilla player positions over loopback RCON; Ctrl-C to stop", flush=True)
    client: Rcon | None = None
    sequence = 0
    while True:
        started = time.monotonic()
        try:
            if client is None:
                client = Rcon(port, password)
            sequence += 1
            sample = collect(client, sequence)
            print(json.dumps({"fresh": True, "sampleAgeSeconds": 0.0,
                              "sample": sample}), flush=True)
        except (OSError, ValueError, PermissionError, ConnectionError) as error:
            if client is not None:
                client.close()
                client = None
            print(json.dumps({"fresh": False, "reason": str(error)}), flush=True)
        remaining = args.interval_seconds - (time.monotonic() - started)
        if remaining > 0:
            time.sleep(remaining)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(0)
