#!/usr/bin/env python3
"""Audit every distinct palette state in one Java terrain proof against Vantage assets."""

import argparse
import json
from pathlib import Path
import re
import subprocess


TEXTURE_RE = re.compile(r"\btex=([^\s]+)")
ELEMENT_RE = re.compile(r"\belements=(\d+)")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--vantage", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    summary = json.loads(args.summary.read_text())
    categories = {name: [] for name in ("exact", "unresolvedModel", "unresolvedTexture", "noModelGeometry", "fluidOrAir")}
    for block in summary["blockStates"]:
        original = block["name"]
        state = block["state"]
        namespace, local = original.split(":", 1)
        if local in ("air", "cave_air", "void_air", "water", "lava") and namespace == "minecraft":
            categories["fluidOrAir"].append(block)
            continue
        effective = (f"minecraft:{namespace}__{local}" if
                     summary.get("flattenedModNamespaces") and namespace != "minecraft" else original)
        command = [str(args.vantage), "resolve", str(args.assets), effective]
        if state:
            command.append(state)
        result = subprocess.run(command, capture_output=True, text=True, check=False)
        entry = {**block, "effectiveName": effective}
        if result.returncode:
            entry["error"] = (result.stderr or result.stdout).strip()
            categories["unresolvedModel"].append(entry)
            continue
        details = result.stdout + result.stderr
        textures = sorted(set(TEXTURE_RE.findall(details)))
        missing = [name for name in textures if not
                   (args.assets / "textures" / f"{name}.png").is_file()]
        if missing:
            entry["textures"] = missing
            categories["unresolvedTexture"].append(entry)
        elif not any(int(count) > 0 for count in ELEMENT_RE.findall(details)):
            categories["noModelGeometry"].append(entry)
        else:
            categories["exact"].append(entry)
    report = {"summary": str(args.summary), "assets": str(args.assets),
              "distinctPaletteStates": len(summary["blockStates"]),
              "counts": {key: len(value) for key, value in categories.items()},
              "blockCounts": {key: sum(item.get("blocks", 0) for item in value)
                              for key, value in categories.items()},
              "states": categories}
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({**{key: value for key, value in report.items() if key != "states"},
                      "examples": {key: value[:10] for key, value in categories.items()
                                   if key != "exact"}}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
