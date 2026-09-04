#!/usr/bin/env python3
"""Generate or verify a flat SHA-256 manifest for release assets."""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path


MANIFEST_LINE = re.compile(r"^([0-9a-f]{64})  (.+)$")
RELEASE_VERSION = r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?"
EXPECTED_BETA_ASSETS = {
    "macos desktop": re.compile(rf"^msc2-(?P<version>{RELEASE_VERSION})-macos-x86_64\.dmg$"),
    "macos headless": re.compile(
        rf"^msc2-headless-(?P<version>{RELEASE_VERSION})-macos-x86_64\.tar\.gz$"
    ),
    "windows desktop": re.compile(rf"^msc2-(?P<version>{RELEASE_VERSION})-windows-x86_64\.msi$"),
    "windows headless": re.compile(
        rf"^msc2-headless-(?P<version>{RELEASE_VERSION})-windows-x86_64\.zip$"
    ),
    "linux desktop": re.compile(rf"^msc2-(?P<version>{RELEASE_VERSION})-linux-x86_64\.deb$"),
    "linux headless": re.compile(
        rf"^msc2-headless-(?P<version>{RELEASE_VERSION})-linux-x86_64\.tar\.gz$"
    ),
}
CHUNK_SIZE = 1024 * 1024


class ManifestError(Exception):
    """A human-readable release manifest failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ManifestError(message)


def asset_files(artifacts: Path, manifest: Path) -> list[Path]:
    require(artifacts.is_dir(), f"artifact directory does not exist: {artifacts}")
    files = sorted(artifacts.iterdir(), key=lambda path: path.name)
    require(files, f"artifact directory is empty: {artifacts}")

    assets: list[Path] = []
    for path in files:
        if path.resolve() == manifest.resolve():
            continue
        require(not path.is_symlink(), f"release asset must not be a symlink: {path.name}")
        require(path.is_file(), f"release artifacts must be flat regular files: {path.name}")
        assets.append(path)

    require(assets, f"artifact directory has no release assets: {artifacts}")
    return assets


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(CHUNK_SIZE), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_beta_assets(assets: list[Path]) -> None:
    """Require the complete, architecture-labeled first-beta asset set."""
    require(
        len(assets) == len(EXPECTED_BETA_ASSETS),
        f"expected {len(EXPECTED_BETA_ASSETS)} beta assets, found {len(assets)}",
    )

    matched: dict[str, str] = {}
    for path in assets:
        matches = [
            (label, pattern.fullmatch(path.name))
            for label, pattern in EXPECTED_BETA_ASSETS.items()
        ]
        label, match = next(((label, match) for label, match in matches if match), (None, None))
        require(label is not None and match is not None, f"unexpected beta asset name: {path.name}")
        require(label not in matched, f"duplicate beta asset role: {label}")
        matched[label] = match.group("version")

    require(
        set(matched) == set(EXPECTED_BETA_ASSETS),
        "beta asset set is incomplete: " + ", ".join(sorted(set(EXPECTED_BETA_ASSETS) - set(matched))),
    )
    versions = set(matched.values())
    require(len(versions) == 1, "beta assets do not share one release version")


def write_manifest(manifest: Path, assets: list[Path]) -> None:
    manifest.parent.mkdir(parents=True, exist_ok=True)
    contents = "".join(f"{sha256(path)}  {path.name}\n" for path in assets)
    manifest.write_text(contents, encoding="utf-8")


def read_manifest(manifest: Path) -> dict[str, str]:
    require(manifest.is_file(), f"manifest does not exist: {manifest}")
    entries: dict[str, str] = {}
    for line_number, line in enumerate(manifest.read_text(encoding="utf-8").splitlines(), start=1):
        match = MANIFEST_LINE.fullmatch(line)
        require(
            match is not None,
            f"{manifest}:{line_number}: expected 64 lowercase hex characters, two spaces, and a filename",
        )
        digest, filename = match.groups()
        require(Path(filename).name == filename, f"{manifest}:{line_number}: asset name must not contain a path")
        require(filename not in entries, f"{manifest}:{line_number}: duplicate asset {filename}")
        entries[filename] = digest

    require(entries, f"manifest is empty: {manifest}")
    return entries


def verify_manifest(manifest: Path, artifacts: Path) -> None:
    assets = asset_files(artifacts, manifest)
    validate_beta_assets(assets)
    actual = {path.name: path for path in assets}
    expected = read_manifest(manifest)

    missing = sorted(set(expected) - set(actual))
    extra = sorted(set(actual) - set(expected))
    require(not missing, f"manifest names missing artifact(s): {', '.join(missing)}")
    require(not extra, f"artifact directory has unlisted file(s): {', '.join(extra)}")

    mismatched = [
        filename
        for filename in sorted(expected)
        if sha256(actual[filename]) != expected[filename]
    ]
    require(not mismatched, f"SHA-256 mismatch for: {', '.join(mismatched)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True, help="manifest file to generate or verify")
    parser.add_argument("--artifacts", type=Path, required=True, help="flat directory containing release assets")
    parser.add_argument(
        "--write",
        action="store_true",
        help="write the manifest from the current artifact bytes before verifying it",
    )
    args = parser.parse_args()

    try:
        assets = asset_files(args.artifacts, args.manifest)
        if args.write:
            write_manifest(args.manifest, assets)
        verify_manifest(args.manifest, args.artifacts)
    except (ManifestError, OSError, UnicodeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    action = "generated and verified" if args.write else "verified"
    print(f"OK: {action} SHA-256 manifest {args.manifest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
