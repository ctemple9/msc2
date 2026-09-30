#!/usr/bin/env python3
"""Install the player-feed proof pack into one stopped BDS server's active world."""

import argparse
import json
from pathlib import Path
import shutil


PACK_ID = "98328b20-538c-47fd-96e1-9ed1d057d675"
VERSION = [0, 0, 1]
SOURCE = Path(__file__).resolve().parent / "bedrock-player-feed"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--server-stopped", action="store_true",
                        help="confirm BDS was stopped in MSC before changing pack files")
    args = parser.parse_args()
    if not args.server_stopped:
        parser.error("stop BDS in MSC, then pass --server-stopped")
    server = args.server_dir.resolve(strict=True)
    properties = (server / "server.properties").read_text(encoding="utf-8")
    names = [line.split("=", 1)[1].strip() for line in properties.splitlines()
             if line.startswith("level-name=")]
    if len(names) != 1 or not names[0]:
        parser.error("server.properties must have one level-name")
    world = (server / "worlds" / names[0]).resolve(strict=True)
    if not world.is_relative_to(server / "worlds") or not (world / "level.dat").is_file():
        parser.error("active Bedrock world is missing or outside server/worlds")
    pack_dir = server / "behavior_packs" / "msc-map-player-feed-proof"
    if pack_dir.exists():
        parser.error(f"proof pack already exists: {pack_dir}")
    config = world / "world_behavior_packs.json"
    entries = json.loads(config.read_text(encoding="utf-8")) if config.exists() else []
    if not isinstance(entries, list) or any(not isinstance(entry, dict) for entry in entries):
        parser.error("world_behavior_packs.json is not a pack list")
    if any(entry.get("pack_id") == PACK_ID for entry in entries):
        parser.error("proof pack is already assigned to this world")
    backup = config.with_name(config.name + ".before-msc-map-proof")
    if config.exists():
        if backup.exists():
            parser.error(f"prior proof backup already exists: {backup}")
        shutil.copy2(config, backup)
    shutil.copytree(SOURCE, pack_dir)
    entries.append({"pack_id": PACK_ID, "version": VERSION})
    config.write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"server": str(server), "world": str(world),
                      "pack": str(pack_dir), "worldPackList": str(config)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
