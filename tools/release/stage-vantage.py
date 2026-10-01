#!/usr/bin/env python3
"""Download and verify the pinned Vantage CLI for one MSC target."""

from __future__ import annotations

import argparse
import hashlib
import io
import os
from pathlib import Path
import sys
import tarfile
from typing import NoReturn
from urllib.request import Request, urlopen
from zipfile import ZipFile


VERSION = "0.15.1"
BASE_URL = f"https://github.com/thoughts-on-things/vantage-mc/releases/download/v{VERSION}"
ASSETS = {
    "macos-x86_64": (
        "vantage-x86_64-macos.tar.gz",
        "dd20d193a508ca260aff1ae9cf7dc42025ac2a9dd104020f7a4937b3479059e2",
        "vantage",
    ),
    "macos-aarch64": (
        "vantage-aarch64-macos.tar.gz",
        "a3043a386fcf9afbfa676997d33c64309ddb7769a14a1a22b473b9ed11e2da5d",
        "vantage",
    ),
    "linux-x86_64": (
        "vantage-x86_64-linux.tar.gz",
        "ff1fb5059017c536d8cd3dc8654155333fba763a12888fa308c1949458f5d89c",
        "vantage",
    ),
    "linux-aarch64": (
        "vantage-aarch64-linux.tar.gz",
        "a94c945ec83778e7170305887cbdbd062e5d8dab35f8db0db63d0d05a1c62fe6",
        "vantage",
    ),
    "windows-x86_64": (
        "vantage-x86_64-windows.zip",
        "d333a297e0e0027dff0c26f20cb111c9eab97cc97bfa65bbad382201b7812b15",
        "vantage.exe",
    ),
}
MAX_ARCHIVE_BYTES = 32 * 1024 * 1024
LICENSE_PATH = Path(__file__).with_name("VANTAGE-LICENSE.txt")


def fail(message: str) -> NoReturn:
    print(f"Vantage staging failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def read_archive(url: str) -> bytes:
    request = Request(url, headers={"User-Agent": "MSC-2-release-stager"})
    try:
        with urlopen(request, timeout=45) as response:
            payload = response.read(MAX_ARCHIVE_BYTES + 1)
    except Exception as error:  # noqa: BLE001 - surfaced with the pinned asset name
        fail(f"could not download the pinned release asset: {error}")
    if len(payload) > MAX_ARCHIVE_BYTES:
        fail(f"release asset exceeds the {MAX_ARCHIVE_BYTES}-byte staging limit")
    return payload


def executable_from_archive(archive_name: str, executable_name: str, payload: bytes) -> bytes:
    if archive_name.endswith(".zip"):
        try:
            with ZipFile(io.BytesIO(payload)) as archive:
                if archive.namelist() != [executable_name]:
                    fail("ZIP contents do not match the expected single executable")
                return archive.read(executable_name)
        except Exception as error:  # noqa: BLE001 - preserve a useful staging error
            fail(f"could not read the Vantage ZIP: {error}")
    try:
        with tarfile.open(fileobj=io.BytesIO(payload), mode="r:gz") as archive:
            members = archive.getmembers()
            if len(members) != 1 or members[0].name != executable_name or not members[0].isfile():
                fail("tarball contents do not match the expected single executable")
            stream = archive.extractfile(members[0])
            if stream is None:
                fail("Vantage executable is missing from the release tarball")
            return stream.read()
    except Exception as error:  # noqa: BLE001 - preserve a useful staging error
        fail(f"could not read the Vantage tarball: {error}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=sorted(ASSETS), required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    archive_name, expected_hash, executable_name = ASSETS[args.platform]
    payload = read_archive(f"{BASE_URL}/{archive_name}")
    actual_hash = hashlib.sha256(payload).hexdigest()
    if actual_hash != expected_hash:
        fail(f"SHA-256 mismatch for {archive_name}: expected {expected_hash}, got {actual_hash}")

    executable = executable_from_archive(archive_name, executable_name, payload)
    if not executable:
        fail("Vantage executable is empty")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    destination = args.output_dir / executable_name
    temporary = destination.with_name(f".{destination.name}.staging")
    temporary.write_bytes(executable)
    if executable_name == "vantage":
        os.chmod(temporary, 0o755)
    temporary.replace(destination)
    license_destination = args.output_dir / "VANTAGE-LICENSE.txt"
    license_destination.write_bytes(LICENSE_PATH.read_bytes())
    print(f"staged Vantage {VERSION} for {args.platform}: {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
