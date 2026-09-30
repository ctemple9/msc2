#!/usr/bin/env python3
"""Measure one manual saved-terrain refresh without publishing world data."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
from time import perf_counter
from uuid import uuid4


ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--resource-pack", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--chunk-x", type=int, required=True)
    parser.add_argument("--chunk-z", type=int, required=True)
    args = parser.parse_args()

    cli = ROOT / "target/debug/msc"
    exporter = ROOT / "tools/world-map-proof/target/release/msc-world-map-proof"
    if not cli.is_file() or not exporter.is_file():
        parser.error("build the MSC CLI and world-map exporter first")
    if not args.resource_pack.is_dir() or not args.output_root.is_dir():
        parser.error("resource pack and initial viewer output must already exist")

    environment = os.environ.copy()
    environment.setdefault(
        "MSC2_DATA_DIR", str(Path.home() / "Library/Application Support/MSC 2")
    )
    started = perf_counter()
    snapshot = subprocess.run(
        [str(cli), "--json", "world", "map-snapshot"],
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )
    snapshot_ms = round((perf_counter() - started) * 1000)
    if snapshot.returncode != 0:
        print(snapshot.stderr or snapshot.stdout, file=sys.stderr)
        return snapshot.returncode
    operation = json.loads(snapshot.stdout)
    if operation.get("state") != "succeeded":
        print(snapshot.stdout, file=sys.stderr)
        return 1
    result = operation.get("result") or {}
    world_path = Path(result["worldPath"])
    if not world_path.is_dir():
        print("snapshot returned a missing world path", file=sys.stderr)
        return 1

    revision = f"r{uuid4().hex[:10]}"
    output = args.output_root / "revisions" / revision
    export_started = perf_counter()
    export = subprocess.run(
        [
            str(exporter),
            str(world_path),
            str(args.resource_pack),
            str(output),
            f"{args.chunk_x},{args.chunk_z}",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    export_ms = round((perf_counter() - export_started) * 1000)
    if export.returncode != 0:
        print(export.stderr or export.stdout, file=sys.stderr)
        return export.returncode
    if not (output / "terrain.vtile").is_file() or not (output / "terrain.vtexarr").is_file():
        print("export finished without both terrain files", file=sys.stderr)
        return 1

    print(json.dumps({
        "revision": revision,
        "snapshotMs": snapshot_ms,
        "exportMs": export_ms,
        "readyMs": round((perf_counter() - started) * 1000),
        "holdMillis": int(result["holdMillis"]),
        "bytesCopied": int(result["bytesCopied"]),
        "output": str(output),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
