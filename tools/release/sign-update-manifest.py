#!/usr/bin/env python3
"""Build and sign the canonical MSC application update manifest."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import re
import sys
from pathlib import Path


RELEASE_VERSION = r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?"
ASSET_PATTERNS = {
    "macos-desktop": re.compile(rf"^msc2-(?P<version>{RELEASE_VERSION})-macos-x86_64\.dmg$"),
    "macos-headless": re.compile(
        rf"^msc2-headless-(?P<version>{RELEASE_VERSION})-macos-x86_64\.tar\.gz$"
    ),
    "windows-desktop": re.compile(
        rf"^msc2-(?P<version>{RELEASE_VERSION})-windows-x86_64\.msi$"
    ),
    "windows-headless": re.compile(
        rf"^msc2-headless-(?P<version>{RELEASE_VERSION})-windows-x86_64\.zip$"
    ),
    "linux-deb": re.compile(rf"^msc2-(?P<version>{RELEASE_VERSION})-linux-x86_64\.deb$"),
    "linux-rpm": re.compile(rf"^msc2-(?P<version>{RELEASE_VERSION})-linux-x86_64\.rpm$"),
    "linux-headless": re.compile(
        rf"^msc2-headless-(?P<version>{RELEASE_VERSION})-linux-x86_64\.tar\.gz$"
    ),
}
CHUNK_SIZE = 1024 * 1024
RELEASE_METADATA = {
    "RELEASE-NOTES.md",
    "SHA256SUMS",
    "UNSIGNED-BETA-NOTICE.txt",
    "msc2-update-manifest.json",
    "msc2-update-manifest.sig",
}


class ManifestError(Exception):
    """A human-readable manifest construction failure."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ManifestError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(CHUNK_SIZE), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=True,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


# RFC 8032 Ed25519 signing, kept here so the release runner needs no Python
# package beyond the standard library. The workflow supplies only the seed;
# the seed is never written to the repository or to a release asset.
Q = 2**255 - 19
L = 2**252 + 27742317777372353535851937790883648493


def inv(value: int) -> int:
    return pow(value, Q - 2, Q)


def xrecover(y: int) -> int:
    xx = (y * y - 1) * inv(121665 * y * y + 1)
    x = pow(xx, (Q + 3) // 8, Q)
    if (x * x - xx) % Q:
        x = (x * 19681161315388985) % Q
    if x % 2:
        x = Q - x
    return x


By = 4 * inv(5) % Q
Bx = xrecover(By)
B = (Bx, By)


def edwards_add(point_a: tuple[int, int], point_b: tuple[int, int]) -> tuple[int, int]:
    x1, y1 = point_a
    x2, y2 = point_b
    d = -121665 * inv(121666) % Q
    denominator_x = inv(1 + d * x1 * x2 * y1 * y2)
    denominator_y = inv(1 - d * x1 * x2 * y1 * y2)
    return (
        (x1 * y2 + x2 * y1) * denominator_x % Q,
        (y1 * y2 + x1 * x2) * denominator_y % Q,
    )


def scalar_mult(point: tuple[int, int], scalar: int) -> tuple[int, int]:
    result = (0, 1)
    addend = point
    while scalar:
        if scalar & 1:
            result = edwards_add(result, addend)
        addend = edwards_add(addend, addend)
        scalar >>= 1
    return result


def encode_point(point: tuple[int, int]) -> bytes:
    x, y = point
    encoded = y | ((x & 1) << 255)
    return encoded.to_bytes(32, "little")


def signing_key_bytes(value: str) -> bytes:
    try:
        key = bytes.fromhex(value)
    except ValueError as error:
        raise ManifestError("release signing key must be hexadecimal") from error
    require(len(key) == 32, "release signing key must be a 32-byte Ed25519 seed")
    return key


def sign(seed: bytes, message: bytes) -> bytes:
    digest = hashlib.sha512(seed).digest()
    scalar = int.from_bytes(digest[:32], "little")
    scalar &= (1 << 254) - 8
    scalar |= 1 << 254
    public = encode_point(scalar_mult(B, scalar))
    nonce = int.from_bytes(hashlib.sha512(digest[32:] + message).digest(), "little") % L
    encoded_nonce = encode_point(scalar_mult(B, nonce))
    challenge = int.from_bytes(
        hashlib.sha512(encoded_nonce + public + message).digest(), "little"
    ) % L
    response = (nonce + challenge * scalar) % L
    return encoded_nonce + response.to_bytes(32, "little")


def match_assets(artifacts: Path, release_id: str) -> dict[str, Path]:
    require(artifacts.is_dir(), f"artifact directory does not exist: {artifacts}")
    matches: dict[str, Path] = {}
    for path in sorted(artifacts.iterdir(), key=lambda item: item.name):
        if path.name in RELEASE_METADATA:
            continue
        require(not path.is_symlink() and path.is_file(), f"release asset must be a regular file: {path.name}")
        matched = False
        for role, pattern in ASSET_PATTERNS.items():
            match = pattern.fullmatch(path.name)
            if match is None:
                continue
            matched = True
            require(role not in matches, f"duplicate release asset role: {role}")
            require(
                match.group("version") == release_id,
                f"{path.name} does not match release ID {release_id}",
            )
            matches[role] = path
            break
        require(matched, f"unexpected release asset name: {path.name}")

    missing = sorted(set(ASSET_PATTERNS) - set(matches))
    require(not missing, "release asset set is incomplete: " + ", ".join(missing))
    return matches


def asset(path: Path, role: str) -> dict[str, object]:
    size = path.stat().st_size
    require(size > 0, f"release asset is empty: {path.name}")
    return {
        "bytes": size,
        "filename": path.name,
        "role": role,
        "sha256": sha256(path),
    }


def make_manifest(
    assets: dict[str, Path],
    release_id: str,
    tag: str,
    api_major: int,
    min_minor: int,
    max_minor: int,
) -> dict[str, object]:
    require(tag == f"v{release_id}", f"tag {tag} does not match release ID {release_id}")
    require(min_minor <= max_minor, "API minimum minor must not exceed maximum minor")

    def entry(
        target: str,
        install_mode: str,
        asset_path: Path,
        role: str,
        included_components: list[str],
    ) -> dict[str, object]:
        result: dict[str, object] = {
            "assets": [asset(asset_path, role)],
            "forbiddenArtifacts": [],
            "includedComponents": included_components,
            "installMode": install_mode,
            "target": target,
        }
        return result

    return {
        "api": {"major": api_major, "maxMinor": max_minor, "minMinor": min_minor},
        "platforms": {
            "macos-desktop-x86_64": entry(
                "x86_64-apple-darwin",
                "tauri-coordinated",
                assets["macos-desktop"],
                "desktop",
                ["agent", "sidecar"],
            ),
            "macos-headless-x86_64": entry(
                "x86_64-apple-darwin",
                "standalone-archive",
                assets["macos-headless"],
                "archive",
                ["agent", "sidecar"],
            ),
            "windows-desktop-x86_64": entry(
                "x86_64-pc-windows-msvc",
                "tauri-coordinated",
                assets["windows-desktop"],
                "desktop",
                ["agent"],
            ),
            "windows-headless-x86_64": entry(
                "x86_64-pc-windows-msvc",
                "standalone-archive",
                assets["windows-headless"],
                "archive",
                ["agent"],
            ),
            "linux-desktop-deb-x86_64": entry(
                "x86_64-unknown-linux-gnu",
                "authorized-package-install",
                assets["linux-deb"],
                "package-deb",
                [],
            ),
            "linux-desktop-rpm-x86_64": entry(
                "x86_64-unknown-linux-gnu",
                "authorized-package-install",
                assets["linux-rpm"],
                "package-rpm",
                [],
            ),
            "linux-headless-x86_64": entry(
                "x86_64-unknown-linux-gnu",
                "standalone-archive",
                assets["linux-headless"],
                "archive",
                ["agent"],
            ),
        },
        "releaseId": release_id,
        "releaseSet": "msc-application",
        "schemaVersion": 1,
        "tag": tag,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True, help="final release asset directory")
    parser.add_argument("--release-id", required=True, help="release version without the v prefix")
    parser.add_argument("--tag", required=True, help="exact immutable Git tag")
    parser.add_argument("--api-major", type=int, required=True)
    parser.add_argument("--api-min-minor", type=int, required=True)
    parser.add_argument("--api-max-minor", type=int, required=True)
    parser.add_argument(
        "--private-key-env",
        default="MSC2_RELEASE_SIGNING_KEY_HEX",
        help="environment variable containing the 32-byte Ed25519 seed in hex",
    )
    parser.add_argument("--manifest", type=Path, required=True, help="canonical manifest output")
    parser.add_argument("--signature", type=Path, required=True, help="detached base64 signature output")
    args = parser.parse_args()

    try:
        assets = match_assets(args.artifacts, args.release_id)
        manifest = make_manifest(
            assets,
            args.release_id,
            args.tag,
            args.api_major,
            args.api_min_minor,
            args.api_max_minor,
        )
        key_value = os.environ.get(args.private_key_env, "").strip()
        require(key_value, f"{args.private_key_env} is not configured")
        signature = sign(signing_key_bytes(key_value), canonical_json(manifest))
        args.manifest.parent.mkdir(parents=True, exist_ok=True)
        args.signature.parent.mkdir(parents=True, exist_ok=True)
        args.manifest.write_bytes(canonical_json(manifest))
        args.signature.write_text(base64.b64encode(signature).decode("ascii") + "\n", encoding="ascii")
    except (ManifestError, OSError, ValueError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    print(f"OK: signed update manifest {args.manifest} with {args.signature}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
