#!/usr/bin/env python3
"""Show structured player samples from MSC console output; mark silent feeds stale."""

import argparse
import json
import os
from pathlib import Path
import select
import subprocess
import sys
from time import sleep
from time import monotonic


PREFIX = "MSC_MAP_PLAYERS_V1 "


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--poll", action="store_true",
                        help="poll authenticated console tail instead of reading stdin")
    parser.add_argument("--server", default="theboyslatest")
    args = parser.parse_args()
    last_received = None
    stale_reported = False
    previous_sample = None
    while True:
        if args.poll:
            cli = Path(__file__).resolve().parents[2] / "target/debug/msc"
            environment = os.environ.copy()
            environment.setdefault("MSC2_DATA_DIR", str(
                Path.home() / "Library/Application Support/MSC 2"))
            result = subprocess.run(
                [str(cli), "--json", "console", "tail", "--server", args.server,
                 "-n", "100"],
                capture_output=True, text=True, env=environment, check=False,
            )
            if result.returncode:
                print(result.stderr or result.stdout, file=sys.stderr, flush=True)
                return result.returncode
            lines = [entry.get("text", "") for entry in json.loads(result.stdout)]
            if previous_sample is None:
                lines = [line for line in lines if PREFIX in line][-1:]
        else:
            readable, _, _ = select.select([sys.stdin], [], [], 1.0)
            if not readable:
                lines = []
            else:
                line = sys.stdin.readline()
                if not line:
                    return 0
                lines = [line]
        for line in lines:
            if "MSC_MAP_PLAYERS_ERROR " in line:
                print(line.strip(), file=sys.stderr, flush=True)
                continue
            if PREFIX not in line:
                continue
            try:
                sample = json.loads(line.split(PREFIX, 1)[1])
                sequence = sample["sequence"]
                players = sample["players"]
                sampled_at = sample["sampledAtMs"]
                if (not isinstance(sequence, int) or not isinstance(sampled_at, int)
                        or not isinstance(players, list)):
                    raise ValueError("invalid feed envelope")
                identity = (sampled_at, sequence)
                if args.poll and previous_sample is not None and identity <= previous_sample:
                    continue
                previous_sample = identity
                print(json.dumps({"fresh": True, "sample": sample}), flush=True)
                last_received = monotonic()
                stale_reported = False
            except (ValueError, KeyError) as error:
                print(f"invalid player feed sample: {error}", file=sys.stderr, flush=True)
        if last_received is not None and monotonic() - last_received >= 5 and not stale_reported:
            print(json.dumps({"fresh": False, "reason": "no sample for five seconds"}), flush=True)
            stale_reported = True
        if args.poll:
            sleep(1)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
