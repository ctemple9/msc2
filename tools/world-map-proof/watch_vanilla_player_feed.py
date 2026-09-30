#!/usr/bin/env python3
"""Normalize Vanilla datapack command-feedback records to MSC player samples."""

import argparse
import json
import math
import os
from pathlib import Path
import re
import select
import subprocess
import sys
from time import monotonic, time
import uuid


SEQUENCE_RE = re.compile(r"#msc_sequence\s+has\s+(-?\d+)\s+\[msc_map_players\]", re.I)
TICK_RE = re.compile(r"#msc_tick\s+has\s+(-?\d+)\s+\[msc_map_players\]", re.I)
DIMENSION_RE = re.compile(
    r"#msc_dimension_(overworld|nether|end)\s+has\s+-?\d+\s+\[msc_map_players\]", re.I
)
ENTITY_DATA_RE = re.compile(
    r"(?:data for entity\s+(.+?)\s+is:|(.+?)\s+has the following entity data:)\s*(\[[^\]]*\])",
    re.I,
)
UUID_RE = re.compile(r"^\[I;\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*,\s*(-?\d+)\s*\]$")
NUMBER_RE = re.compile(r"[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?[dDfF]?")
DIMENSIONS = {
    "overworld": "minecraft:overworld",
    "nether": "minecraft:the_nether",
    "end": "minecraft:the_end",
}


def numeric_list(value: str, expected: int) -> list[float] | None:
    if not (value.startswith("[") and value.endswith("]")):
        return None
    parts = value[1:-1].split(",")
    if len(parts) != expected:
        return None
    result = []
    for part in parts:
        part = part.strip()
        if not NUMBER_RE.fullmatch(part):
            return None
        number = float(part[:-1] if part[-1:] in "dDfF" else part)
        if not math.isfinite(number):
            return None
        result.append(number)
    return result


def uuid_from_nbt(value: str) -> str | None:
    match = UUID_RE.fullmatch(value)
    if not match:
        return None
    words = [int(word) & 0xFFFFFFFF for word in match.groups()]
    high = (words[0] << 32) | words[1]
    low = (words[2] << 32) | words[3]
    return str(uuid.UUID(int=(high << 64) | low))


def normalize_player_records(sample: dict) -> list[dict]:
    players = []
    for name, record in sample["records"].items():
        if not all(key in record for key in ("id", "pos", "rotation", "dimension")):
            continue
        players.append({
            "id": record["id"],
            "name": name,
            "dimension": record["dimension"],
            "x": record["pos"][0],
            "y": record["pos"][1],
            "z": record["pos"][2],
            "yaw": record["rotation"][0],
            "pitch": record["rotation"][1],
        })
    return players


def parse_console_line(line: str, current: dict | None) -> tuple[dict | None, dict | None]:
    sequence_match = SEQUENCE_RE.search(line)
    if sequence_match:
        sequence = int(sequence_match.group(1))
        if current is None or sequence != current["sequence"]:
            return {"sequence": sequence, "tick": None, "dimension": None, "records": {}}, None
        completed = current
        return None, completed
    if current is None:
        return current, None
    tick_match = TICK_RE.search(line)
    if tick_match:
        current["tick"] = int(tick_match.group(1))
        return current, None
    dimension_match = DIMENSION_RE.search(line)
    if dimension_match:
        current["dimension"] = DIMENSIONS[dimension_match.group(1).lower()]
        return current, None
    data_match = ENTITY_DATA_RE.search(line)
    if not data_match or current["dimension"] is None:
        return current, None
    first_name, second_name, value = data_match.groups()
    name = first_name or second_name
    name = name.strip()
    record = current["records"].setdefault(name, {})
    parsed_uuid = uuid_from_nbt(value)
    if parsed_uuid is not None:
        record["id"] = parsed_uuid
    else:
        coordinates = numeric_list(value, 3)
        rotation = numeric_list(value, 2)
        if coordinates is not None:
            record["pos"] = coordinates
        elif rotation is not None:
            record["rotation"] = rotation
        else:
            return current, None
        record["dimension"] = current["dimension"]
    return current, None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", required=True, help="MSC server ID or display name")
    parser.add_argument("--stale-seconds", type=float, default=5.0)
    args = parser.parse_args()
    if args.stale_seconds <= 0:
        parser.error("--stale-seconds must be greater than zero")
    cli = Path(__file__).resolve().parents[2] / "target/debug/msc"
    environment = os.environ.copy()
    environment.setdefault(
        "MSC2_DATA_DIR", str(Path.home() / "Library/Application Support/MSC 2")
    )
    last_fresh = monotonic()
    stale_reported = False
    current = None
    previous_sequence = None
    print(f"Watching Vanilla datapack feedback for {args.server}; Ctrl-C to stop")
    process = subprocess.Popen(
        [str(cli), "--json", "console", "follow", "--server", args.server],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=environment,
    )
    pending = b""
    try:
        while True:
            readable, _, _ = select.select([process.stdout], [], [], 1.0)
            if readable:
                chunk = os.read(process.stdout.fileno(), 65536)
                if not chunk:
                    error = process.stderr.read().decode("utf-8", errors="replace").strip()
                    print(error or "MSC console stream closed", file=sys.stderr, flush=True)
                    return process.wait() or 1
                pending += chunk
                if len(pending) > 1_000_000:
                    print("MSC console line exceeded size limit", file=sys.stderr, flush=True)
                    return 1
                while b"\n" in pending:
                    raw, pending = pending.split(b"\n", 1)
                    try:
                        entry = json.loads(raw)
                        line = entry.get("text", "")
                        if not isinstance(line, str):
                            continue
                        if "MSC_MAP_PLAYERS_ERROR " in line:
                            print(line.strip(), file=sys.stderr, flush=True)
                        current, completed = parse_console_line(line, current)
                        if completed is None:
                            continue
                        if previous_sequence is not None and completed["sequence"] <= previous_sequence:
                            continue
                        previous_sequence = completed["sequence"]
                        now_ms = int(time() * 1000)
                        sample = {
                            "sequence": completed["sequence"],
                            "tick": completed["tick"],
                            "sampledAtMs": now_ms,
                            "players": normalize_player_records(completed),
                        }
                        age = max(0.0, time() - now_ms / 1000)
                        print(json.dumps({"fresh": age <= args.stale_seconds,
                                          "sampleAgeSeconds": round(age, 2),
                                          "sample": sample}), flush=True)
                        last_fresh = monotonic()
                        stale_reported = False
                    except (json.JSONDecodeError, AttributeError, TypeError, ValueError) as error:
                        print(f"invalid Vanilla player sample: {error}", file=sys.stderr,
                              flush=True)
            if monotonic() - last_fresh >= args.stale_seconds and not stale_reported:
                print(json.dumps({"fresh": False, "reason": "no new sample within stale window"}),
                      flush=True)
                stale_reported = True
    finally:
        if process.poll() is None:
            process.terminate()
        process.wait()


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(0)
