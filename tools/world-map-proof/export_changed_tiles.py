#!/usr/bin/env python3
"""Compare two consistent offline BDS snapshots and export only changed 4x4 tiles."""

import argparse
import json
from pathlib import Path
import subprocess
import sys
from time import perf_counter


ROOT = Path(__file__).resolve().parents[2]
EXPORTER = ROOT / "tools/world-map-proof/target/release/msc-world-map-proof"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, required=True)
    parser.add_argument("--resource-pack", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--chunk-x", type=int, required=True,
                        help="first tile origin; subsequent tiles advance four chunks")
    parser.add_argument("--chunk-z", type=int, required=True)
    parser.add_argument("--tiles-x", type=int, default=2)
    parser.add_argument("--tiles-z", type=int, default=2)
    args = parser.parse_args()
    if not EXPORTER.is_file():
        parser.error("build the release world-map exporter first")
    for path in (args.before, args.after, args.resource_pack):
        if not path.is_dir():
            parser.error(f"missing directory: {path}")
    if not 1 <= args.tiles_x <= 8 or not 1 <= args.tiles_z <= 8:
        parser.error("tile grid must be 1..8 by 1..8")
    args.output_root.mkdir(parents=True, exist_ok=True)

    started = perf_counter()
    comparison = subprocess.run(
        [str(EXPORTER), "compare", str(args.before), str(args.after),
         f"{args.chunk_x},{args.chunk_z}", str(args.tiles_x), str(args.tiles_z)],
        capture_output=True, text=True, check=False,
    )
    if comparison.returncode:
        print(comparison.stderr or comparison.stdout, file=sys.stderr)
        return comparison.returncode
    report = json.loads(comparison.stdout)
    compare_ms = round((perf_counter() - started) * 1000)
    exported = []
    for x, z in report["changedTiles"]:
        destination = args.output_root / f"tile_{x}_{z}"
        # A prior export at this name would mix versions if rendering failed.
        if destination.exists():
            parser.error(f"output already exists: {destination}")
        export = subprocess.run(
            [str(EXPORTER), str(args.after), str(args.resource_pack),
             str(destination), f"{x},{z}"],
            capture_output=True, text=True, check=False,
        )
        if export.returncode:
            print(export.stderr or export.stdout, file=sys.stderr)
            return export.returncode
        if not all((destination / name).is_file()
                   for name in ("terrain.vtile", "terrain.vtexarr")):
            print(f"incomplete tile export: {destination}", file=sys.stderr)
            return 1
        exported.append(str(destination))
    print(json.dumps({
        "checkedChunks": report["checkedChunks"],
        "changedTiles": report["changedTiles"],
        "compareMs": compare_ms,
        "exported": exported,
        "readyMs": round((perf_counter() - started) * 1000),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
