#!/usr/bin/env python3
"""Read current structured Java player samples from MSC's authenticated console."""

import argparse
import json
import math
import os
from pathlib import Path
import select
import subprocess
import sys
from time import monotonic, time


PREFIX = "MSC_MAP_PLAYERS_V1 "


def integer_field(value):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    if not math.isfinite(value) or int(value) != value:
        return None
    return int(value)


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
    previous_sample = None
    last_fresh = monotonic()
    stale_reported = False
    print(f"Watching authenticated console samples for {args.server}; Ctrl-C to stop")

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
                        if PREFIX not in line:
                            continue
                        sample = json.loads(line.split(PREFIX, 1)[1])
                        if not isinstance(sample.get("players"), list):
                            raise ValueError("players is not a list")
                        identity = (integer_field(sample.get("sampledAtMs")),
                                    integer_field(sample.get("sequence")))
                        if None in identity:
                            raise ValueError("sample time or sequence is not an integer")
                        if previous_sample is not None and identity <= previous_sample:
                            continue
                        previous_sample = identity
                        age = max(0.0, time() - identity[0] / 1000)
                        fresh = age <= args.stale_seconds
                        print(json.dumps({"fresh": fresh, "sampleAgeSeconds": round(age, 2),
                                          "sample": sample}), flush=True)
                        if fresh:
                            last_fresh = monotonic()
                            stale_reported = False
                    except (json.JSONDecodeError, AttributeError, ValueError) as error:
                        print(f"invalid console player sample: {error}", file=sys.stderr,
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
