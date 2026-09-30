#!/usr/bin/env python3
"""Show structured player samples from `msc console follow`; mark silent feeds stale."""

import json
import select
import sys
from time import monotonic


PREFIX = "MSC_MAP_PLAYERS_V1 "


def main() -> int:
    last_received = None
    stale_reported = False
    previous_sequence = None
    while True:
        readable, _, _ = select.select([sys.stdin], [], [], 1.0)
        if readable:
            line = sys.stdin.readline()
            if not line:
                return 0
            if "MSC_MAP_PLAYERS_ERROR " in line:
                print(line.strip(), file=sys.stderr, flush=True)
                continue
            if PREFIX not in line:
                continue
            try:
                sample = json.loads(line.split(PREFIX, 1)[1])
                sequence = sample["sequence"]
                players = sample["players"]
                if not isinstance(sequence, int) or not isinstance(players, list):
                    raise ValueError("invalid feed envelope")
                if previous_sequence is not None and sequence != previous_sequence + 1:
                    print("feed sequence gap or restart", file=sys.stderr, flush=True)
                previous_sequence = sequence
                print(json.dumps({"fresh": True, "sample": sample}), flush=True)
                last_received = monotonic()
                stale_reported = False
            except (ValueError, KeyError) as error:
                print(f"invalid player feed sample: {error}", file=sys.stderr, flush=True)
        if last_received is not None and monotonic() - last_received >= 5 and not stale_reported:
            print(json.dumps({"fresh": False, "reason": "no sample for five seconds"}), flush=True)
            stale_reported = True
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
