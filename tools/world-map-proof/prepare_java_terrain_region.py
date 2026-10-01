#!/usr/bin/env python3
"""Copy a 4x4 Java Anvil area, adapting 26.3 block palettes for Vantage 0.15.1.

Requires nbtlib. The source region is read only; output is a private proof file.
"""

import argparse
from collections import Counter
import gzip
import io
import json
from pathlib import Path
import re
import struct
import zlib

from nbtlib import File
from nbtlib.tag import Compound, List, String


SECTOR = 4096
AREA = 4
REGION_RE = re.compile(r"r\.(-?\d+)\.(-?\d+)\.mca$")


def selected_world(server: Path) -> Path:
    properties = server / "server.properties"
    if not properties.is_file():
        raise ValueError(f"missing server.properties: {properties}")
    settings = {}
    for line in properties.read_text(encoding="utf-8").splitlines():
        if line.startswith(("#", "!")) or "=" not in line:
            continue
        key, value = line.split("=", 1)
        settings[key.strip()] = value.strip()
    level = settings.get("level-name", "world")
    world = (server / level).resolve(strict=True)
    if not world.is_relative_to(server) or not (world / "level.dat").is_file():
        raise ValueError("selected level is outside the server or lacks level.dat")
    return world


def overworld_region(world: Path, chunk_x: int, chunk_z: int) -> tuple[Path, str]:
    region_name = f"r.{chunk_x // 32}.{chunk_z // 32}.mca"
    for layout, relative in (
        ("root", "region"),
        ("dimensions/minecraft/overworld", "dimensions/minecraft/overworld/region"),
    ):
        path = world / relative / region_name
        if path.is_file():
            return path, layout
    raise ValueError(f"no Overworld region {region_name} in either Java save layout")


def chunk_payload(region: bytes, local_x: int, local_z: int) -> tuple[int, bytes] | None:
    index = (local_z * 32 + local_x) * 4
    location = region[index:index + 4]
    offset = int.from_bytes(location[:3], "big") * SECTOR
    sectors = location[3]
    if offset == 0 or sectors == 0:
        return None
    if offset + 5 > len(region):
        raise ValueError("region chunk location points outside the file")
    length = struct.unpack_from(">I", region, offset)[0]
    if length < 1 or length + 4 > sectors * SECTOR or offset + length + 4 > len(region):
        raise ValueError("invalid region chunk payload length")
    compression = region[offset + 4]
    if compression & 0x80:
        raise ValueError("external .mcc chunk payload is not supported in this proof")
    return compression, region[offset + 5:offset + 4 + length]


def decode(compression: int, payload: bytes) -> bytes:
    if compression == 1:
        return gzip.decompress(payload)
    if compression == 2:
        return zlib.decompress(payload)
    if compression == 3:
        return payload
    raise ValueError(f"unsupported Anvil compression type {compression}")


def legacy_palette_entry(entry, flatten_mod_namespaces: bool = False) -> Compound:
    if isinstance(entry, String):
        return Compound({"Name": String(str(entry))})
    if not isinstance(entry, Compound):
        raise ValueError(f"unexpected block palette entry {type(entry).__name__}")
    if "Name" in entry:
        if flatten_mod_namespaces:
            name = str(entry["Name"])
            namespace, _, local_name = name.partition(":")
            if namespace and namespace != "minecraft" and local_name:
                return Compound({**entry, "Name": String(f"minecraft:{namespace}__{local_name}")})
        return entry
    name = entry.get("id", entry.get(""))
    if not isinstance(name, String):
        raise ValueError("new block palette entry has no block ID")
    result = Compound({"Name": String(str(name))})
    properties = entry.get("properties")
    if properties is not None:
        if not isinstance(properties, Compound):
            raise ValueError("new block palette properties are malformed")
        result["Properties"] = properties
    return result


def normalize_chunk(raw: bytes, flatten_mod_namespaces: bool = False) -> tuple[bytes, int, int, Counter]:
    chunk = File.parse(io.BytesIO(raw))
    version = int(chunk.get("DataVersion", -1))
    normalized = 0
    block_states: Counter = Counter()
    for section in chunk.get("sections", []):
        states = section.get("block_states")
        if states is None or "palette" not in states:
            continue
        palette = states["palette"]
        converted = []
        changed = False
        section_states = []
        for entry in palette:
            original_name = str(entry.get("Name", entry.get("id", entry.get("", "")))) if isinstance(entry, Compound) else str(entry)
            legacy = legacy_palette_entry(entry, flatten_mod_namespaces)
            converted.append(legacy)
            properties = legacy.get("Properties", {})
            state = ",".join(f"{key}={properties[key]}" for key in sorted(properties))
            section_states.append((original_name, state))
            changed |= legacy is not entry
        values = states.get("data")
        if values is None or len(section_states) == 1:
            block_states[section_states[0]] += 4096
        else:
            bits = max(4, (len(section_states) - 1).bit_length())
            per_long = 64 // bits
            mask = (1 << bits) - 1
            if len(values) * per_long < 4096:
                raise ValueError("block-state data is shorter than a section")
            for position in range(4096):
                word = int(values[position // per_long]) & 0xFFFFFFFFFFFFFFFF
                index = (word >> ((position % per_long) * bits)) & mask
                if index >= len(section_states):
                    raise ValueError("block-state data references a missing palette entry")
                block_states[section_states[index]] += 1
        if changed:
            states["palette"] = List[Compound](converted)
            normalized += len(converted)
    if not normalized:
        return raw, version, 0, block_states
    output = io.BytesIO()
    chunk.write(output)
    return output.getvalue(), version, normalized, block_states


def prepare(region: bytes, chunk_x: int, chunk_z: int, flatten_mod_namespaces: bool = False) -> tuple[bytes, dict]:
    result = bytearray(SECTOR * 2)
    found = 0
    normalized = 0
    versions: set[int] = set()
    block_states: Counter = Counter()
    missing = []
    for z in range(chunk_z, chunk_z + AREA):
        for x in range(chunk_x, chunk_x + AREA):
            lx, lz = x % 32, z % 32
            source = chunk_payload(region, lx, lz)
            if source is None:
                missing.append([x, z])
                continue
            raw, version, count, chunk_states = normalize_chunk(
                decode(*source), flatten_mod_namespaces)
            versions.add(version)
            normalized += count
            block_states.update(chunk_states)
            payload = zlib.compress(raw)
            record = struct.pack(">I", len(payload) + 1) + b"\x02" + payload
            sectors = (len(record) + SECTOR - 1) // SECTOR
            if sectors > 255:
                raise ValueError("normalized chunk exceeds the Anvil location limit")
            offset = len(result) // SECTOR
            result.extend(record)
            result.extend(b"\0" * (sectors * SECTOR - len(record)))
            index = (lz * 32 + lx) * 4
            result[index:index + 4] = offset.to_bytes(3, "big") + bytes([sectors])
            result[SECTOR + index:SECTOR + index + 4] = region[SECTOR + index:SECTOR + index + 4]
            found += 1
    if found == 0:
        raise ValueError("selected 4x4 area contains no saved chunks")
    return bytes(result), {
        "presentChunks": found,
        "missingChunks": missing,
        "dataVersions": sorted(versions),
        "normalizedPaletteEntries": normalized,
        "distinctBlockIds": len({name for name, _ in block_states}),
        "blockIds": sorted({name for name, _ in block_states}),
        "blockStates": [{"name": name, "state": state, "blocks": count}
                        for (name, state), count in sorted(block_states.items())],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server-dir", type=Path, required=True)
    parser.add_argument("--chunk-x", type=int, required=True)
    parser.add_argument("--chunk-z", type=int, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--flatten-mod-namespaces", action="store_true",
                        help="rename mod block IDs for a namespace-flattened Vantage asset directory")
    args = parser.parse_args()
    if args.chunk_x // 32 != (args.chunk_x + AREA - 1) // 32 or args.chunk_z // 32 != (args.chunk_z + AREA - 1) // 32:
        parser.error("the selected 4x4 area must fit within one Anvil region")
    try:
        server = args.server_dir.resolve(strict=True)
        world = selected_world(server)
        source, layout = overworld_region(world, args.chunk_x, args.chunk_z)
        if not REGION_RE.fullmatch(source.name):
            raise ValueError("selected region filename is invalid")
        output_dir = args.output_dir.resolve()
        if output_dir.is_relative_to(server):
            raise ValueError("proof output must be outside the server directory")
        output_dir.mkdir(parents=True, exist_ok=True)
        prepared, summary = prepare(source.read_bytes(), args.chunk_x, args.chunk_z,
                                    args.flatten_mod_namespaces)
        result_path = output_dir / source.name
        result_path.write_bytes(prepared)
        summary.update({"source": str(source), "preparedRegion": str(result_path),
                        "saveLayout": layout, "chunkOrigin": [args.chunk_x, args.chunk_z],
                        "flattenedModNamespaces": args.flatten_mod_namespaces,
                        "localRange": [args.chunk_x % 32, args.chunk_z % 32,
                                       (args.chunk_x + AREA - 1) % 32,
                                       (args.chunk_z + AREA - 1) % 32]})
        (output_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps({key: value for key, value in summary.items()
                          if key not in ("blockIds", "blockStates")}, indent=2))
    except (OSError, ValueError, KeyError, zlib.error) as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
