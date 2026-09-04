#!/usr/bin/env python3
"""Create the Windows ICO resource from the checked-in PNG application icon."""

from __future__ import annotations

import struct
import sys
from pathlib import Path


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {Path(sys.argv[0]).name} WORKSPACE_ROOT", file=sys.stderr)
        return 2

    workspace = Path(sys.argv[1])
    source = workspace / "clients" / "desktop-web" / "src-tauri" / "icons" / "icon.png"
    destination = source.with_name("icon.ico")
    payload = source.read_bytes()
    if not payload.startswith(PNG_SIGNATURE):
        raise SystemExit(f"application icon is not a PNG: {source}")

    # ICO permits PNG-encoded image payloads. A zero dimension means the
    # source image is 256 pixels or larger, which covers the checked-in 512px
    # application icon without requiring an image-processing dependency.
    entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(payload), 22)
    destination.write_bytes(struct.pack("<HHH", 0, 1, 1) + entry + payload)
    print(f"prepared Windows icon at {destination}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
