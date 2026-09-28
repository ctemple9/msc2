#!/usr/bin/env python3
"""Require Tauri's frontend and the agent's embedded frontend to match."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import sys


def files_by_relative_path(root: Path) -> dict[str, Path]:
    if not root.is_dir():
        raise ValueError(f"bundle directory does not exist: {root}")

    files: dict[str, Path] = {}
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"bundle contains a symbolic link: {path}")
        if path.is_file():
            files[path.relative_to(root).as_posix()] = path
    for required in ("index.html", "bundle-identity.json"):
        if required not in files:
            raise ValueError(f"bundle is missing required file: {root / required}")
    return files


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("desktop_bundle", type=Path)
    parser.add_argument("agent_bundle", type=Path)
    args = parser.parse_args()

    try:
        desktop = files_by_relative_path(args.desktop_bundle)
        agent = files_by_relative_path(args.agent_bundle)
    except ValueError as error:
        print(f"client bundle check failed: {error}", file=sys.stderr)
        return 1

    desktop_names = set(desktop)
    agent_names = set(agent)
    missing_from_agent = sorted(desktop_names - agent_names)
    missing_from_desktop = sorted(agent_names - desktop_names)
    different = sorted(
        name
        for name in desktop_names & agent_names
        if digest(desktop[name]) != digest(agent[name])
    )
    if missing_from_agent or missing_from_desktop or different:
        for name in missing_from_agent:
            print(f"agent bundle is missing desktop file: {name}", file=sys.stderr)
        for name in missing_from_desktop:
            print(f"desktop bundle is missing agent file: {name}", file=sys.stderr)
        for name in different:
            print(f"bundle content differs: {name}", file=sys.stderr)
        return 1

    print(
        "OK: desktop and agent bundles contain the same "
        f"{len(desktop_names)} files with matching SHA-256 content"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
