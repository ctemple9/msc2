#!/usr/bin/env python3
"""Prove the agent packages the exact Vite output configured for Tauri."""

from __future__ import annotations

import filecmp
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CLIENT = ROOT / "clients" / "desktop-web"
DIST = CLIENT / "dist"
PACKAGED = ROOT / "crates" / "msc-agent" / "web-ui"


def fail(message: str) -> None:
    raise SystemExit(f"bundle identity check failed: {message}")


def compared_files(directory: Path) -> dict[Path, bytes]:
    return {
        path.relative_to(directory): path.read_bytes()
        for path in sorted(directory.rglob("*"))
        if path.is_file()
    }


def main() -> None:
    config = json.loads((CLIENT / "src-tauri" / "tauri.conf.json").read_text())
    if config["build"]["frontendDist"] != "../dist":
        fail("Tauri must load clients/desktop-web/dist")

    subprocess.run(["npm", "--prefix", str(CLIENT), "run", "build"], check=True)
    if not PACKAGED.is_dir():
        fail("the agent package has no web-ui directory")

    dist_files = compared_files(DIST)
    packaged_files = compared_files(PACKAGED)
    if not dist_files:
        fail("Vite did not produce files")
    if dist_files.keys() != packaged_files.keys():
        missing = sorted(str(path) for path in dist_files.keys() - packaged_files.keys())
        extra = sorted(str(path) for path in packaged_files.keys() - dist_files.keys())
        fail(f"file set differs (missing={missing}, extra={extra})")
    mismatched = [path for path, contents in dist_files.items() if packaged_files[path] != contents]
    if mismatched:
        fail(f"byte mismatch: {', '.join(str(path) for path in mismatched)}")
    if not filecmp.cmp(DIST / "index.html", PACKAGED / "index.html", shallow=False):
        fail("index.html differs")

    print(f"OK: {len(dist_files)} production files are byte-identical for Tauri and msc-agent")


if __name__ == "__main__":
    main()
