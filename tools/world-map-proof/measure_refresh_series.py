#!/usr/bin/env python3
"""Collect a few owner-triggered BDS refresh samples for one 4x4 area."""

import argparse
import json
from pathlib import Path
import statistics
import subprocess
import sys


MEASURE = Path(__file__).with_name("measure_refresh.py")
METRICS = ("snapshotMs", "holdMillis", "exportMs", "readyMs", "bytesCopied")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--resource-pack", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--chunk-x", type=int, required=True)
    parser.add_argument("--chunk-z", type=int, required=True)
    parser.add_argument("--samples", type=int, default=3)
    args = parser.parse_args()
    if not 2 <= args.samples <= 5:
        parser.error("choose between 2 and 5 manual samples")

    samples = []
    for index in range(args.samples):
        try:
            input(f"Sample {index + 1}/{args.samples}: press Enter to capture the running BDS world, or Ctrl-C to stop. ")
        except (EOFError, KeyboardInterrupt):
            print("\nMeasurement stopped before another capture.", file=sys.stderr)
            return 1
        command = [
            sys.executable,
            str(MEASURE),
            "--resource-pack", str(args.resource_pack),
            "--output-root", str(args.output_root),
            "--chunk-x", str(args.chunk_x),
            "--chunk-z", str(args.chunk_z),
        ]
        measured = subprocess.run(command, capture_output=True, text=True, check=False)
        if measured.returncode != 0:
            print(measured.stderr or measured.stdout, file=sys.stderr)
            return measured.returncode
        sample = json.loads(measured.stdout)
        samples.append(sample)
        print(json.dumps(sample, indent=2), flush=True)

    ranges = {
        metric: {
            "min": min(sample[metric] for sample in samples),
            "median": statistics.median(sample[metric] for sample in samples),
            "max": max(sample[metric] for sample in samples),
        }
        for metric in METRICS
    }
    print(json.dumps({"samples": len(samples), "range": ranges}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
