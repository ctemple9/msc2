#!/usr/bin/env python3
"""Compare saved Java chunk terrain in two offline region copies and export if changed.

Both inputs must be consistent snapshots of the same region. The tool never
reads a live server path and never writes to either input.
"""

import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tempfile
from time import perf_counter

from nbtlib import File

from prepare_java_terrain_region import AREA, chunk_payload, decode, prepare


RENDERED_TAGS = ("sections", "block_entities", "Heightmaps")


def rendered_hash(region: bytes, x: int, z: int) -> str | None:
    payload = chunk_payload(region, x % 32, z % 32)
    if payload is None:
        return None
    chunk = File.parse(io.BytesIO(decode(*payload)))
    relevant = File({name: chunk[name] for name in RENDERED_TAGS if name in chunk})
    encoded = io.BytesIO()
    relevant.write(encoded)
    return hashlib.sha256(encoded.getvalue()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before-region", type=Path, required=True)
    parser.add_argument("--after-region", type=Path, required=True)
    parser.add_argument("--chunk-x", type=int, required=True)
    parser.add_argument("--chunk-z", type=int, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--vantage", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--flatten-mod-namespaces", action="store_true")
    args = parser.parse_args()
    if args.before_region.name != args.after_region.name:
        parser.error("snapshot regions must have the same filename")
    match = re.fullmatch(r"r\.(-?\d+)\.(-?\d+)\.mca", args.after_region.name)
    if match is None or (int(match[1]), int(match[2])) != (args.chunk_x // 32, args.chunk_z // 32):
        parser.error("snapshot region filename does not match the selected tile")
    if args.chunk_x // 32 != (args.chunk_x + AREA - 1) // 32 or args.chunk_z // 32 != (args.chunk_z + AREA - 1) // 32:
        parser.error("the selected 4x4 tile must fit within one region")
    if not args.assets.is_dir() or not args.vantage.is_file():
        parser.error("missing assets or Vantage CLI")
    if args.output_dir.exists():
        parser.error("output directory already exists")
    started = perf_counter()
    before = args.before_region.read_bytes()
    after = args.after_region.read_bytes()
    changed = []
    missing = []
    for z in range(args.chunk_z, args.chunk_z + AREA):
        for x in range(args.chunk_x, args.chunk_x + AREA):
            old = rendered_hash(before, x, z)
            new = rendered_hash(after, x, z)
            if new is None:
                missing.append([x, z])
            if old != new:
                changed.append([x, z])
    compare_ms = round((perf_counter() - started) * 1000)
    if missing:
        parser.error(f"after snapshot lacks selected chunks: {missing}")
    if not changed:
        print(json.dumps({"checkedChunks": AREA * AREA, "changedChunks": [],
                          "compareMs": compare_ms, "exported": False}, indent=2))
        return 0
    prepared, summary = prepare(after, args.chunk_x, args.chunk_z,
                                args.flatten_mod_namespaces)
    if summary["missingChunks"]:
        parser.error("prepared region has missing chunks")
    local = [args.chunk_x % 32, args.chunk_z % 32,
             (args.chunk_x + AREA - 1) % 32, (args.chunk_z + AREA - 1) % 32]
    args.output_dir.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=f".{args.output_dir.name}-", dir=args.output_dir.parent) as scratch:
        scratch_dir = Path(scratch)
        prepared_path = scratch_dir / args.after_region.name
        prepared_path.write_bytes(prepared)
        command = [str(args.vantage), "meshtex", str(prepared_path),
                   str(scratch_dir / "terrain.vtile"), str(args.assets),
                   *(str(value) for value in local), "--light", "smooth",
                   "--biome-blend", "on"]
        export = subprocess.run(command, capture_output=True, text=True, check=False)
        if export.returncode:
            parser.error((export.stderr or export.stdout).strip())
        report = {"checkedChunks": AREA * AREA, "changedChunks": changed,
                  "compareMs": compare_ms, "exported": True,
                  "readyMs": round((perf_counter() - started) * 1000),
                  "output": str(args.output_dir), "localRange": local}
        (scratch_dir / "compare.json").write_text(json.dumps(report, indent=2) + "\n")
        scratch_dir.rename(args.output_dir)
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
