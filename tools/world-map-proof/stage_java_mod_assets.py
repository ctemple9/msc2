#!/usr/bin/env python3
"""Stage client mod assets in a scratch directory for Vantage's flat asset resolver.

Use with prepare_java_terrain_region.py --flatten-mod-namespaces. Nothing is
written to a server save or client installation. Missing assets are reported.
"""

import argparse
import json
from pathlib import Path
import shutil
import zipfile


ASSET_KINDS = ("blockstates", "models", "textures")


def flattened(namespace: str, path: str) -> str:
    return path if namespace == "minecraft" else f"{namespace}__{path}"


def rewrite(value, namespaces: set[str]):
    if isinstance(value, dict):
        return {key: rewrite(item, namespaces) for key, item in value.items()}
    if isinstance(value, list):
        return [rewrite(item, namespaces) for item in value]
    if isinstance(value, str) and not value.startswith("#") and ":" in value:
        namespace, path = value.split(":", 1)
        if namespace in namespaces:
            return f"minecraft:{flattened(namespace, path)}"
    return value


def iter_references(value, key: str = ""):
    if isinstance(value, dict):
        if key == "textures":
            for child in value.values():
                if isinstance(child, str) and not child.startswith("#"):
                    yield "textures", child
            return
        for child_key, child_value in value.items():
            yield from iter_references(child_value, child_key)
    elif isinstance(value, list):
        for child in value:
            yield from iter_references(child, key)
    elif key in ("model", "parent") and isinstance(value, str):
        yield "models", value


def asset_path(root: Path, kind: str, reference: str) -> Path:
    local = reference.split(":", 1)[-1]
    return root / kind / f"{local}.{ 'png' if kind == 'textures' else 'json' }"


def stage(vanilla: Path, mods: Path, summary: Path, output: Path,
          extra_namespaces: set[str], placeholder_blocks: set[str]) -> dict:
    source = json.loads(summary.read_text())
    block_ids = source["blockIds"]
    namespaces = {name.split(":", 1)[0] for name in block_ids if ":" in name}
    namespaces.discard("minecraft")
    namespaces.update(extra_namespaces)
    if output.resolve().is_relative_to(mods.resolve()) or output.resolve().is_relative_to(vanilla.resolve()):
        raise ValueError("stage output must be outside the client installation")
    if output.exists():
        if not (output / ".msc-java-asset-stage").is_file():
            raise ValueError(f"refusing to replace an output without the proof marker: {output}")
        shutil.rmtree(output)
    shutil.copytree(vanilla, output)
    (output / ".msc-java-asset-stage").write_text("Temporary Java terrain proof assets\n")
    vanilla_biomes = vanilla.parent.parent / "data" / "minecraft"
    staged_biomes = output.parent.parent / "data" / "minecraft"
    if vanilla_biomes.is_dir():
        shutil.copytree(vanilla_biomes, staged_biomes, dirs_exist_ok=True)
    origins = {}
    collisions = []
    for jar in sorted(mods.glob("*.jar")):
        with zipfile.ZipFile(jar) as archive:
            for member in archive.infolist():
                parts = Path(member.filename).parts
                if len(parts) < 4 or parts[0] != "assets" or parts[1] not in namespaces:
                    continue
                namespace, kind = parts[1], parts[2]
                if kind not in ASSET_KINDS or member.is_dir():
                    continue
                relative = "/".join(parts[3:])
                target = output / kind / flattened(namespace, relative)
                if target in origins:
                    collisions.append({"asset": str(target.relative_to(output)),
                                       "first": origins[target], "second": jar.name})
                    continue
                target.parent.mkdir(parents=True, exist_ok=True)
                payload = archive.read(member)
                if target.suffix == ".json":
                    try:
                        payload = (json.dumps(rewrite(json.loads(payload), namespaces),
                                              separators=(",", ":")) + "\n").encode()
                    except (UnicodeDecodeError, json.JSONDecodeError):
                        pass
                target.write_bytes(payload)
                origins[target] = jar.name

    for name in sorted(placeholder_blocks):
        if name not in block_ids:
            raise ValueError(f"placeholder block is absent from this tile: {name}")
        namespace, local = name.split(":", 1)
        if namespace == "minecraft":
            raise ValueError("vanilla blocks must not use mod placeholders")
        marker = f"msc_missing__{namespace}__{local}"
        blockstate_path = output / "blockstates" / f"{flattened(namespace, local)}.json"
        model_path = output / "models" / "block" / f"{marker}.json"
        blockstate_path.write_text(json.dumps({"multipart": [{"apply": {
            "model": f"minecraft:block/{marker}"}}]}) + "\n")
        faces = {side: {"texture": "#all"} for side in
                 ("down", "up", "north", "south", "east", "west")}
        model_path.write_text(json.dumps({"textures": {"all": f"block/{marker}"},
                                          "elements": [{"from": [0, 0, 0],
                                                        "to": [16, 16, 16],
                                                        "faces": faces}]}) + "\n")

    missing_blockstates = []
    for name in block_ids:
        if name in ("minecraft:air", "minecraft:cave_air", "minecraft:void_air",
                    "minecraft:water", "minecraft:lava"):
            continue
        namespace, local = name.split(":", 1)
        if not (output / "blockstates" / f"{flattened(namespace, local)}.json").is_file():
            missing_blockstates.append(name)

    missing_models = set()
    missing_textures = set()
    visited = set()

    def inspect_model(reference: str):
        path = asset_path(output, "models", reference)
        if path in visited:
            return
        visited.add(path)
        if not path.is_file():
            missing_models.add(reference)
            return
        try:
            data = json.loads(path.read_text())
        except (UnicodeDecodeError, json.JSONDecodeError):
            missing_models.add(reference)
            return
        for kind, child in iter_references(data):
            if kind == "models":
                inspect_model(child)
            elif not asset_path(output, "textures", child).is_file():
                missing_textures.add(child)

    for name in block_ids:
        if name in missing_blockstates or name.endswith(":air"):
            continue
        namespace, local = name.split(":", 1)
        path = output / "blockstates" / f"{flattened(namespace, local)}.json"
        if not path.is_file():
            continue
        try:
            data = json.loads(path.read_text())
        except (UnicodeDecodeError, json.JSONDecodeError):
            missing_blockstates.append(name)
            continue
        for kind, reference in iter_references(data):
            if kind == "models":
                inspect_model(reference)

    return {"sourceSummary": str(summary), "vanillaAssets": str(vanilla),
            "clientMods": str(mods), "stagedAssets": str(output),
            "modNamespaces": sorted(namespaces), "stagedModFiles": len(origins),
            "markedPlaceholderBlocks": sorted(placeholder_blocks),
            "blockIds": len(block_ids),
            "missingBlockstates": sorted(set(missing_blockstates)),
            "referencedModels": len(visited), "missingModels": sorted(missing_models),
            "missingTextures": sorted(missing_textures), "assetCollisions": collisions}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vanilla-assets", type=Path, required=True)
    parser.add_argument("--client-mods", type=Path, required=True)
    parser.add_argument("--region-summary", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--extra-namespace", action="append", default=[],
                        help="stage a namespace referenced by the selected mods")
    parser.add_argument("--placeholder-block", action="append", default=[],
                        help="mark a known unsupported mod block with the missing checker")
    args = parser.parse_args()
    report = stage(args.vanilla_assets, args.client_mods, args.region_summary,
                   args.output_dir, set(args.extra_namespace),
                   set(args.placeholder_block))
    path = args.output_dir.parent / "asset-audit.json"
    path.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: value if not isinstance(value, list) else
                      {"count": len(value), "items": value[:20]}
                      for key, value in report.items()}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
