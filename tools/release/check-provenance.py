#!/usr/bin/env python3
"""Generate or verify the reviewable dependency and release provenance records."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
INVENTORY_NAME = "DEPENDENCY-INVENTORY.json"
PROVENANCE_NAME = "RELEASE-PROVENANCE.json"
BUILDER_PREFIX = "BUILD-ENVIRONMENT-"
CHUNK_SIZE = 1024 * 1024


class ProvenanceError(Exception):
    """A human-readable release provenance failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ProvenanceError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(CHUNK_SIZE), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def command_version(*command: str) -> str:
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=False)
    except OSError as error:
        raise ProvenanceError(f"cannot run {' '.join(command)}: {error}") from error
    require(result.returncode == 0, f"{' '.join(command)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def read_json(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ProvenanceError(f"cannot read JSON {path}: {error}") from error
    require(isinstance(value, dict), f"expected a JSON object in {path}")
    return value


def locked_inventory() -> tuple[dict[str, object], dict[str, str]]:
    cargo_lock = ROOT / "Cargo.lock"
    npm_lock = ROOT / "clients/desktop-web/package-lock.json"
    try:
        cargo_data = tomllib.loads(cargo_lock.read_text(encoding="utf-8"))
        npm_data = read_json(npm_lock)
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        raise ProvenanceError(f"cannot read dependency lock files: {error}") from error

    cargo_packages = [
        {
            "name": package["name"],
            "version": package["version"],
            "source": package.get("source", "workspace"),
            **({"checksum": package["checksum"]} if "checksum" in package else {}),
        }
        for package in cargo_data.get("package", [])
    ]
    npm_packages = []
    for path, package in sorted(npm_data.get("packages", {}).items()):
        if not path or not isinstance(package, dict) or "version" not in package:
            continue
        item: dict[str, object] = {
            "name": package.get("name") or path.rsplit("node_modules/", 1)[-1],
            "version": package["version"],
        }
        for key in ("resolved", "integrity", "license"):
            if key in package:
                item[key] = package[key]
        npm_packages.append(item)

    locks = {
        "Cargo.lock": sha256(cargo_lock),
        "clients/desktop-web/package-lock.json": sha256(npm_lock),
    }
    inventory: dict[str, object] = {
        "schemaVersion": 1,
        "lockFiles": locks,
        "dependencies": {
            "cargo": {"count": len(cargo_packages), "packages": cargo_packages},
            "npm": {"count": len(npm_packages), "packages": npm_packages},
        },
        "releaseComponents": {},
    }
    return inventory, locks


def manifest_assets(manifest: dict[str, object], artifacts: Path) -> list[dict[str, object]]:
    platforms = manifest.get("platforms")
    require(isinstance(platforms, dict), "update manifest has no platforms object")
    assets: dict[str, dict[str, object]] = {}
    components: dict[str, list[str]] = {}
    for platform, entry in platforms.items():
        require(isinstance(entry, dict), f"invalid platform entry: {platform}")
        included = entry.get("includedComponents", [])
        require(isinstance(included, list), f"invalid includedComponents for {platform}")
        components[str(platform)] = sorted(str(item) for item in included)
        platform_assets = entry.get("assets")
        require(isinstance(platform_assets, list), f"invalid assets for {platform}")
        for asset in platform_assets:
            require(isinstance(asset, dict), f"invalid asset entry for {platform}")
            filename = asset.get("filename")
            digest = asset.get("sha256")
            require(isinstance(filename, str), f"asset filename missing for {platform}")
            require(isinstance(digest, str) and re.fullmatch(r"[0-9a-f]{64}", digest), f"invalid asset digest for {filename}")
            require(Path(filename).name == filename, f"asset filename contains a path: {filename}")
            require(filename not in assets, f"duplicate manifest asset: {filename}")
            path = artifacts / filename
            require(path.is_file() and not path.is_symlink(), f"release asset is missing or not a regular file: {filename}")
            require(sha256(path) == digest, f"update manifest SHA-256 mismatch for {filename}")
            require(path.stat().st_size == asset.get("bytes"), f"update manifest size mismatch for {filename}")
            assets[filename] = {
                "filename": filename,
                "bytes": path.stat().st_size,
                "sha256": digest,
                "platform": str(platform),
                "role": asset.get("role"),
            }
    require(assets, "update manifest contains no release assets")
    return [assets[name] for name in sorted(assets)]


def release_context(manifest_path: Path, artifacts: Path) -> tuple[dict[str, object], dict[str, object], dict[str, str]]:
    manifest = read_json(manifest_path)
    ci_evidence = read_json(artifacts / "CI-EVIDENCE.json")
    source_commit = ci_evidence.get("source_commit")
    release_tag = ci_evidence.get("release_tag")
    require(isinstance(source_commit, str) and re.fullmatch(r"[0-9a-f]{40,64}", source_commit), "CI evidence has no valid source_commit")
    require(isinstance(release_tag, str) and release_tag == manifest.get("tag"), "CI evidence tag does not match update manifest")
    require(ci_evidence.get("source_commit") == os.environ.get("GITHUB_SHA", source_commit), "CI evidence source commit does not match this release checkout")

    rust_toolchain = (ROOT / "rust-toolchain.toml").read_text(encoding="utf-8")
    rust_match = re.search(r'^channel\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"\s*$', rust_toolchain, re.MULTILINE)
    require(rust_match is not None, "Rust toolchain must be pinned to an exact version")
    ci_workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    release_workflow = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
    node_versions = set(re.findall(r"node-version:\s*['\"]([0-9]+\.[0-9]+\.[0-9]+)['\"]", ci_workflow + release_workflow))
    require(len(node_versions) == 1, "CI and release must use one exact Node.js version")
    nextest_versions = set(re.findall(r"cargo-nextest@([0-9]+\.[0-9]+\.[0-9]+)", ci_workflow + release_workflow))
    require(len(nextest_versions) == 1, "CI and release must use one exact cargo-nextest version")

    builder_paths = sorted(artifacts.glob(f"{BUILDER_PREFIX}*.json"))
    expected_platforms = {"linux-x86_64", "macos-x86_64", "macos-aarch64", "windows-x86_64"}
    builders = [read_json(path) for path in builder_paths]
    require(
        {builder.get("platform") for builder in builders} == expected_platforms,
        "release must carry one builder-environment record for each supported artifact platform",
    )
    for builder in builders:
        platform = builder.get("platform")
        require(builder.get("sourceCommit") == source_commit, f"builder source commit mismatch for {platform}")
        runner = builder.get("runner")
        require(isinstance(runner, dict), f"builder runner details missing for {platform}")
        require(runner.get("imageOS") not in (None, "", "unknown"), f"runner OS image missing for {platform}")
        require(runner.get("imageVersion") not in (None, "", "unknown"), f"runner image version missing for {platform}")
        builder_tools = builder.get("toolchain")
        require(isinstance(builder_tools, dict), f"builder toolchain missing for {platform}")
        require(str(builder_tools.get("rustc", "")).startswith(f"rustc {rust_match.group(1)} "), f"builder Rust mismatch for {platform}")
        require(str(builder_tools.get("node", "")).lstrip("v") == next(iter(node_versions)), f"builder Node.js mismatch for {platform}")
        require(builder_tools.get("cargoNextest") == next(iter(nextest_versions)), f"builder cargo-nextest mismatch for {platform}")

    dependency_inventory, lock_hashes = locked_inventory()
    assets = manifest_assets(manifest, artifacts)
    platforms = manifest["platforms"]
    dependency_inventory["releaseComponents"] = {
        platform: sorted(str(component) for component in entry.get("includedComponents", []))
        for platform, entry in platforms.items()
    }
    rustc_version = command_version("rustc", "--version")
    cargo_version = command_version("cargo", "--version")
    node_version = command_version("node", "--version")
    npm_version = command_version("npm", "--version")
    require(rustc_version.startswith(f"rustc {rust_match.group(1)} "), "installed rustc does not match rust-toolchain.toml")
    require(cargo_version.startswith(f"cargo {rust_match.group(1)} "), "installed Cargo does not match rust-toolchain.toml")
    require(node_version.lstrip("v") == next(iter(node_versions)), "installed Node.js does not match the pinned workflow version")
    toolchain = {
        "rustToolchain": rust_match.group(1),
        "rustc": rustc_version,
        "cargo": cargo_version,
        "node": node_version,
        "npm": npm_version,
        "cargoNextest": next(iter(nextest_versions)),
    }
    provenance = {
        "schemaVersion": 1,
        "sourceCommit": source_commit,
        "releaseTag": release_tag,
        "toolchain": toolchain,
        "buildEnvironments": sorted(builders, key=lambda builder: str(builder["platform"])),
        "updateManifest": {
            "path": manifest_path.name,
            "sha256": sha256(manifest_path),
        },
        "dependencyInventory": {
            "path": INVENTORY_NAME,
            "sha256": "",
            "lockFiles": lock_hashes,
        },
        "artifacts": assets,
    }
    return provenance, dependency_inventory, lock_hashes


def write_records(provenance_path: Path, inventory_path: Path, provenance: dict[str, object], inventory: dict[str, object]) -> None:
    inventory_bytes = canonical_json(inventory)
    provenance["dependencyInventory"]["sha256"] = hashlib.sha256(inventory_bytes).hexdigest()  # type: ignore[index]
    inventory_path.write_bytes(inventory_bytes)
    provenance_path.write_bytes(canonical_json(provenance))


def verify_records(provenance_path: Path, inventory_path: Path, expected: dict[str, object], inventory: dict[str, object]) -> None:
    actual_provenance = read_json(provenance_path)
    actual_inventory = read_json(inventory_path)
    require(actual_provenance == expected, "release provenance does not match the manifest, source, toolchain, or artifact bytes")
    expected_inventory = json.loads(canonical_json(inventory))
    require(actual_inventory == expected_inventory, "dependency inventory does not match the checked-in lock files and release components")
    digest = actual_provenance.get("dependencyInventory")
    require(isinstance(digest, dict) and digest.get("sha256") == sha256(inventory_path), "dependency inventory digest does not match release provenance")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True, help="signed update manifest JSON")
    parser.add_argument("--artifacts", type=Path, help="release asset directory (defaults to the manifest directory)")
    parser.add_argument("--write", action="store_true", help="write inventory and provenance records before verification")
    args = parser.parse_args()
    manifest_path = args.manifest if args.manifest.is_absolute() else ROOT / args.manifest
    artifacts = args.artifacts or manifest_path.parent
    if not artifacts.is_absolute():
        artifacts = ROOT / artifacts
    inventory_path = artifacts / INVENTORY_NAME
    provenance_path = artifacts / PROVENANCE_NAME
    try:
        expected, inventory, _ = release_context(manifest_path, artifacts)
        expected["dependencyInventory"]["sha256"] = hashlib.sha256(canonical_json(inventory)).hexdigest()  # type: ignore[index]
        if args.write:
            write_records(provenance_path, inventory_path, expected, inventory)
            expected = read_json(provenance_path)
        verify_records(provenance_path, inventory_path, expected, inventory)
    except (ProvenanceError, OSError, UnicodeError, KeyError, TypeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    print(f"Source commit: {expected['sourceCommit']}")
    print("Toolchain: " + json.dumps(expected["toolchain"], sort_keys=True))
    print(f"Dependency inventory: {inventory_path}")
    print(f"Release provenance: {provenance_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
